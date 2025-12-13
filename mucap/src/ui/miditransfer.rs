use core::f32;
use std::{
    collections::HashSet,
    sync::{Arc, RwLock},
};

use crate::{
    TransportInfo,
    midistore::{MidiStore, StoreEntry},
};
use arboard::Clipboard;
use midly::{
    MetaMessage, MidiMessage, Track, TrackEvent, TrackEventKind,
    num::{u4, u15, u24, u28},
};
use nih_plug::{nih_error, nih_log, nih_warn};
use tempfile::{Builder, NamedTempFile};

pub struct MidiTransfers {
    store: Arc<RwLock<MidiStore>>,
    midifile: Option<NamedTempFile>,
    clippy: Option<Clipboard>,
}

impl MidiTransfers {
    pub fn new(store: Arc<RwLock<MidiStore>>) -> Self {
        Self {
            store,
            midifile: None,
            clippy: None,
        }
    }

    /// Creates a MIDI file from the selected time range and copies it to the clipboard.
    ///
    /// This function extracts all MIDI notes and events within the specified time range,
    /// normalizes them relative to the selection start time, and exports them as a standard
    /// MIDI file. The resulting file is then copied to the system clipboard for easy pasting
    /// into other DAWs.
    /// 
    /// The exported clip will run at 480 PPQN and not contain tempo information. Including tempo
    /// caused annoying popups in Bitwig asking if I want to import the tempo information which
    /// is also not doing what I wanted. This has the side effect that the DAW should be set to the
    /// correct tempo as incorrect tempo will cause stretching ot shrinking. You don't need to have
    /// the BPM set before recording, just at the time of exporting. Also you can just stretch the
    /// clip to adjust.
    /// 
    /// This has the side effect that recording in, e.g, 120 BPM, then setting your DAW to 96, you can
    /// still select whole bars with snapping and paste them after slowing the BPM and it will still
    /// be the same number of bars.
    ///
    /// # Arguments
    ///
    /// * `t0` - Start time of the selection in seconds
    /// * `t1` - End time of the selection in seconds
    /// * `transport` - Transport information containing tempo and timing details
    ///
    /// # Behavior
    ///
    /// - Returns early if the selection contains no notes
    /// - Handles "hanging" notes that start before the selection (includes Note-On messages)
    /// - Handles "incomplete" notes that end after the selection (includes Note-Off messages)
    /// - Quantizes all event timings based on the current tempo (480 PPQN)
    /// - Pads the MIDI file to align with bar boundaries when necessary
    /// - Includes EndOfTrack meta event
    ///
    /// # Logging
    ///
    /// Logs warnings on errors (empty selection, file creation, clipboard access) and
    /// informational messages on success.
    pub fn new_selection(&mut self, t0: f32, t1: f32, transport: &TransportInfo) {
        let store = self.store.read().unwrap();

        let notes = store.notes_in_time_select(-f32::INFINITY, f32::INFINITY, t0, t1);

        if notes.count() == 0 {
            nih_warn!("Empty selection, not exporting");
            return;
        }

        let Ok(mut midifile) = Builder::new().prefix("mucap_").suffix(".mid").tempfile() else {
            nih_warn!("Failed to create tmpfile");
            return;
        };

        nih_log!("Created MIDI file: {:?}", midifile.path());

        let mut of = midifile.as_file_mut();
        let ppqn = 480;
        let pps = ppqn as f32 * (transport.tempo as f32 / 60.);
        let mut smf = midly::Smf::new(midly::Header::new(
            midly::Format::SingleTrack,
            midly::Timing::Metrical(u15::new(ppqn)),
        ));
        smf.tracks.push(midly::Track::new());

        // We don't really want the tempo in the file to prevent DAWs like Bitwig from asking
        // if we want to import the tempo for every single paste
        /*smf.tracks[0].push(TrackEvent {
            delta: u28::new(0),
            kind: TrackEventKind::Meta(MetaMessage::Tempo(u24::new(
                (1000000. * 60. / transport.tempo).round() as u32,
            ))),
        });*/

        // Write Note-Ons already active at start
        for (note, sel) in store.notes_in_time_select(-f32::INFINITY, f32::INFINITY, t0, t1) {
            // Skip notes with very short tails that would end on tick 0 as these can
            // cause artifacts (seen in Bitwig Studio)
            let end_tick = ((note.t_end - t0) * pps).round() as i64;
            if sel && note.t_start < t0 && end_tick > 0 {
                if let Some(entry) = store.store.get(note.idx_on) {
                    let StoreEntry::MidiData { channel, data } = entry.1;
                    smf.tracks[0].push(TrackEvent {
                        delta: u28::new(0),
                        kind: TrackEventKind::Midi {
                            channel,
                            message: data,
                        },
                    });
                }
            }
        }

        let mut sum_delta: i64 = 0;

        for (idx, time, channel, message) in store.midi_events() {
            let write = (time >= t0) && (time <= t1);
            if write {
                let track_time = time - t0;
                let best_track_quantized = (track_time * pps).round() as i64;
                if (best_track_quantized < sum_delta) {
                    nih_error!(
                        "Delta moving backwards, from {} to {}",
                        sum_delta,
                        best_track_quantized
                    );
                }
                let delta = (best_track_quantized - sum_delta) as u32;
                sum_delta = best_track_quantized;
                smf.tracks[0].push(TrackEvent {
                    delta: u28::new(delta),
                    kind: TrackEventKind::Midi { channel, message },
                });
            }
        }

        // Write Note-Off at end of selection
        let track_time = t1 - t0;
        let best_track_quantized = (track_time * pps).round() as i64;
        let mut delta_end = ((best_track_quantized - sum_delta) as u32).saturating_sub(1);
        /*sum_delta = best_track_quantized;
        smf.tracks[0].push(TrackEvent {
            delta: u28::new(delta),
            kind: TrackEventKind::Midi {
                channel: 1.into(),
                message: MidiMessage::NoteOff {
                    key: 1.into(),
                    vel: 1.into(),
                },
            },
        });*/

        nih_log!(
            "t0: {}, t1: {}, tdiff: {}, sum_delta: {}",
            t0,
            t1,
            t1 - t0,
            sum_delta
        );

        // Write Note-Offs for incomplete notes
        for (note, sel) in store.notes_in_time_select(-f32::INFINITY, f32::INFINITY, t0, t1) {
            if sel && note.t_end > t1 {
                if let Some(entry) = store.store.get(note.idx_off) {
                    let StoreEntry::MidiData { channel, data } = entry.1;
                    smf.tracks[0].push(TrackEvent {
                        delta: u28::new(delta_end),
                        kind: TrackEventKind::Midi {
                            channel,
                            message: data,
                        },
                    });
                    sum_delta += delta_end as i64;
                    delta_end = 0;
                }
            }
        }

        /*let bar_len_pulses = (pps * transport.bar_length()).round() as i64;
        let abs_delta = (((sum_delta % bar_len_pulses) - bar_len_pulses) % bar_len_pulses).abs();
        if abs_delta > 10 {
            nih_log!(
                "Adjusting delta {}, {}",
                abs_delta,
                sum_delta % bar_len_pulses
            );
            smf.tracks[0].push(TrackEvent {
                delta: u28::new((bar_len_pulses - (sum_delta % bar_len_pulses)) as u32),
                kind: TrackEventKind::Midi {
                    channel: 1.into(),
                    message: MidiMessage::NoteOff {
                        key: 1.into(),
                        vel: 1.into(),
                    },
                },
            });
        };*/
        smf.tracks[0].push(TrackEvent {
            delta: u28::new(delta_end),
            kind: TrackEventKind::Meta(midly::MetaMessage::EndOfTrack),
        });
        if let Ok(_) = smf.save(&midifile.path()) {
            nih_log!("Saved MIDI file: {:?}", midifile.path());
        } else {
            nih_warn!("Error saving MIDI file");
            return;
        }
        nih_log!("{:?}", smf);

        if self.clippy.is_none() {
            let Ok(mut clippy) = Clipboard::new() else {
                nih_log!("Error acquiring clipboard");
                return;
            };
            self.clippy = Some(clippy);
        }

        if let Ok(_) = self
            .clippy
            .as_mut()
            .unwrap()
            .set()
            .file_list(&[midifile.path()])
        {
            nih_log!("Copied path {:?} to clipboard", midifile.path());
        } else {
            nih_warn!("Failed to copy to clipboard");
            return;
        }

        self.midifile = Some(midifile);
    }
}
