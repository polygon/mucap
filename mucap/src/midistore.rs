use anyhow::Result;
use midly::MidiMessage;
use midly::live::LiveEvent;
use midly::num::{u4, u7};
use nih_plug::midi::{NoteEvent, sysex::SysExMessage};
use nih_plug::nih_log;

use crate::TransportInfo;

/// Information about a MIDI note with links to the NoteOn and NoteOff events
#[derive(Clone)]
pub struct Note {
    /// Start time of the note in seconds.
    pub t_start: f32,
    /// Index of the NoteOn event in the store.
    pub idx_on: usize,
    /// End time of the note in seconds.
    pub t_end: f32,
    /// Index of the NoteOff event in the store.
    pub idx_off: usize,
    /// MIDI channel (0-15).
    pub channel: u4,
    /// MIDI note number (0-127).
    pub key: u7,
    /// MIDI velocity (0-127).
    pub vel: u7,
}

/// A MIDI bar marker with timing information.
#[derive(Clone, Debug)]
pub struct Bar {
    /// Bar number as reported by DAW.
    pub bar_number: i32,
    /// Time of the bar start in seconds.
    pub t: f32,
}

/// A MIDI event stored in the MidiStore.
pub enum StoreEntry {
    /// A MIDI message with its channel.
    MidiData { channel: u4, data: MidiMessage },
}

/// Stores MIDI events and tracks completed notes, in-flight notes, and timing information.
///
/// The MidiStore manages raw MIDI events, constructs complete notes from NoteOn/NoteOff pairs,
/// and maintains caches of note and time ranges for efficient querying. It also provides information
/// about the current transport and captures bar events if they are provided.
pub struct MidiStore {
    /// Raw MIDI events stored as (time, entry) tuples, time is never decreasing.
    pub store: Vec<(f32, StoreEntry)>,
    /// Completed notes (both NoteOn and NoteOff received).
    pub notes: Vec<Note>,
    /// Notes that have started but not yet ended (in-flight notes).
    pub in_flight: Vec<Note>,
    /// Bar markers extracted from transport information.
    pub bars: Vec<Bar>,
    /// Last recorded bar start position in beats.
    last_bar: Option<f64>,
    /// Cache of minimum and maximum note keys seen so far.
    note_range_cache: Option<(u7, u7)>,
    /// Cache of minimum and maximum times seen so far.
    time_range_cache: Option<(f32, f32)>,
    /// Current transport information.
    pub transport: TransportInfo,
}

impl MidiStore {
    /// Creates a new empty MidiStore.
    pub fn new() -> Self {
        Self {
            store: Vec::with_capacity(60000),
            notes: Vec::with_capacity(10000),
            bars: Vec::with_capacity(1000),
            last_bar: None,
            in_flight: Vec::with_capacity(128 * 16),
            note_range_cache: None,
            time_range_cache: None,
            transport: TransportInfo::default(),
        }
    }

    /// Adds a MIDI event to the store at the given time.
    ///
    /// # Arguments
    /// * `time` - Time in seconds. Must be >= the time of the last added event.
    /// * `data` - MIDI message as a 3-byte array.
    ///
    /// # Returns
    /// Returns `Ok(())` on success, or an error if time ordering is violated.
    ///
    /// # Behavior
    /// - Parses the MIDI message and adds it to the store.
    /// - For NoteOn/NoteOff messages, updates the in-flight notes list and completed notes list.
    /// - NoteOn with velocity 0 is treated as NoteOff per MIDI 1.0 specification.
    pub fn add(&mut self, time: f32, data: [u8; 3]) -> Result<()> {
        if time < self.store.last().map(|e| e.0).unwrap_or(0.0) {
            anyhow::bail!("Later entry exists");
        }
        let ev = LiveEvent::parse(&data)?;
        if let LiveEvent::Midi { channel, message } = ev {
            let entry = StoreEntry::MidiData {
                channel,
                data: message,
            };
            self.store.push((time, entry));
            match message {
                MidiMessage::NoteOn { key: _, vel } if vel == 0 => {
                    // Per MIDI 1.0 Spec: NoteOn with velocity 0 is treated as NoteOff
                    // See: http://midi.teragonaudio.com/tech/midispec/noteon.htm
                    self.add_off(time, self.store.len() - 1, channel, message)
                }
                MidiMessage::NoteOn { key: _, vel: _ } => {
                    self.add_on(time, self.store.len() - 1, channel, message)
                }
                MidiMessage::NoteOff { key: _, vel: _ } => {
                    self.add_off(time, self.store.len() - 1, channel, message)
                }
                _ => (),
            }
        }
        Ok(())
    }

    fn add_on(&mut self, time: f32, idx: usize, channel: u4, message: MidiMessage) {
        if let MidiMessage::NoteOn { key, vel } = message {
            // Check if note is already in-flight, override the existing one (TODO: correct?)
            let new_note = Note {
                idx_on: idx,
                t_start: time,
                t_end: 0.0,
                idx_off: 0,
                channel,
                key,
                vel,
            };
            if let Some(old) = self
                .in_flight
                .iter_mut()
                .filter(|note| note.key == key && note.channel == channel)
                .next()
            {
                *old = new_note;
            } else {
                self.update_ranges(new_note.key, time);
                self.in_flight.push(new_note);
            }
        }
    }

    fn add_off(&mut self, time: f32, idx_off: usize, channel: u4, message: MidiMessage) {
        // Handle both NoteOff and NoteOn with velocity 0 (per MIDI 1.0 spec)
        let key = match message {
            MidiMessage::NoteOff { key, vel: _ } => Some(key),
            MidiMessage::NoteOn { key, vel } if vel == 0 => Some(key),
            _ => None,
        };

        if let Some(key) = key {
            if let Some((idx, _)) = self
                .in_flight
                .iter()
                .enumerate()
                .filter(|(_, note)| note.key == key && note.channel == channel)
                .next()
            {
                let mut note = self.in_flight.remove(idx);
                note.idx_off = idx_off;
                note.t_end = time;
                nih_log!(
                    "New note {} ({}), {:.2} s, starting at {:.2}",
                    note.key,
                    note.channel,
                    note.t_end - note.t_start,
                    note.t_start
                );
                self.update_ranges(note.key, note.t_end);
                self.notes.push(note);
            } else {
                nih_log!("Note Off without Note On @ {:.6}", time);
            }
        }
    }

    fn update_ranges(&mut self, note: u7, time: f32) {
        // Update note range cache
        if let Some((note_min, note_max)) = self.note_range_cache {
            if note < note_min {
                self.note_range_cache = Some((note, note_max));
            } else if note > note_max {
                self.note_range_cache = Some((note_min, note));
            }
        } else {
            self.note_range_cache = Some((note, note));
        }

        // Update time range cache
        if let Some((time_min, time_max)) = self.time_range_cache {
            if time < time_min {
                self.time_range_cache = Some((time, time_max));
            } else if time > time_max {
                self.time_range_cache = Some((time_min, time));
            }
        } else {
            self.time_range_cache = Some((time, time));
        }
    }

    /// Returns the minimum and maximum MIDI note numbers seen in completed notes.
    ///
    /// Returns `None` if no notes have been completed yet.
    pub fn note_range(&self) -> Option<(u7, u7)> {
        self.note_range_cache
    }

    /// Returns the note range as u8 values instead of u7.
    ///
    /// Returns `None` if no notes have been completed yet.
    pub fn note_range_u8(&self) -> Option<(u8, u8)> {
        self.note_range_cache.map(|(n0, n1)| (n0.as_int(), n1.as_int()))
    }


    /// Returns the minimum and maximum times in seconds seen in completed notes.
    ///
    /// Returns `None` if no notes have been completed yet.
    pub fn time_range(&self) -> Option<(f32, f32)> {
        self.time_range_cache
    }

    /// Returns an iterator of all notes that overlap with the time range [t0, t1].
    ///
    /// A note overlaps if `t0 < note.t_end && t1 > note.t_start`.
    pub fn notes_in_time(&self, t0: f32, t1: f32) -> impl Iterator<Item = &Note> {
        self.notes.iter().filter(move |note| (t0 < note.t_end) && (t1 > note.t_start))
    }

    /// Returns an iterator of notes in [t0, t1] paired with a boolean indicating if they're selected.
    ///
    /// A note is considered "selected" if it overlaps with [sel_t0, sel_t1].
    pub fn notes_in_time_select(&self, t0: f32, t1: f32, sel_t0: f32, sel_t1: f32) -> impl Iterator<Item = (&Note, bool)> {
        self.notes_in_time(t0, t1).map(move |note| (note, sel_t0 < note.t_end && sel_t1 > note.t_start))
    }

    /// Returns an iterator over all MIDI events as (index, time, channel, message) tuples.
    pub fn midi_events(&self) -> impl Iterator<Item = (usize, f32, u4, MidiMessage)> {
        self.store.iter().enumerate().map(|(idx, (time, entry))| {
            match entry {
                StoreEntry::MidiData { channel, data } => (idx, *time, *channel, *data)
            }
        })
    }

    /// Adds a bar marker based on the provided transport information.
    ///
    /// Calculates the bar number and time, then adds it to the bars list.
    /// Duplicate consecutive bars (within 0.01 beats) are skipped.
    pub fn add_bar(&mut self, transport: TransportInfo) {
        self.transport = transport;
        if !self.transport.playing {
            self.last_bar = None;
            return;
        }
        if let Some(last_bar) = self.last_bar {
            if (self.transport.bar_start_pos_beats - last_bar).abs() < 0.01 {
                return;
            }
        }
        nih_log!("self.transport Info: {:?}", self.transport);
        self.last_bar = Some(self.transport.bar_start_pos_beats);
        let t = self.transport.time - (self.transport.pos_beats as f32 - self.transport.bar_start_pos_beats as f32) * 60. / self.transport.tempo as f32;
        let beats_per_bar = 4. * self.transport.time_sig.0 as f32 / self.transport.time_sig.1 as f32;
        let bar_number = (self.transport.bar_start_pos_beats as f32 / beats_per_bar).round() as i32;

        let bar = Bar {
            bar_number,
            t
        };

        nih_log!("Add Bar: {:?}: {}, {}", bar, self.transport.time, t);
        self.bars.push(bar);
    }

    /// Returns an iterator of bars whose bar_number is divisible by n.
    ///
    /// Useful for filtering to only major bars (e.g., every 4th bar).
    pub fn get_bars(&self, n: i32) -> impl Iterator<Item = &Bar> {
        self.bars.iter().filter(move |bar| bar.bar_number % n == 0)
    }

    /// Finds the nearest bar (filtered by n) to the given time.
    ///
    /// Returns the bar whose time is closest to the given time,
    /// considering only bars where bar_number % n == 0.
    pub fn nearest_bar(&self, time: f32, n: i32) -> Option<&Bar> {
        self.get_bars(n).reduce(|b1, b2| if (b1.t - time).abs() < (b2.t - time).abs() { b1 } else { b2 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note_on(channel: u8, key: u8, vel: u8) -> [u8; 3] {
        [0x90 | channel, key, vel]
    }

    fn note_off(channel: u8, key: u8, vel: u8) -> [u8; 3] {
        [0x80 | channel, key, vel]
    }

    #[test]
    fn test_simple_note_on_off() {
        let mut store = MidiStore::new();

        store.add(0.0, note_on(0, 60, 100)).unwrap();
        assert_eq!(store.in_flight.len(), 1);
        assert_eq!(store.store.len(), 1);

        store.add(1.0, note_off(0, 60, 0)).unwrap();
        assert_eq!(store.in_flight.len(), 0);
        assert_eq!(store.store.len(), 2);
        assert_eq!(store.notes.len(), 1);
    }

    #[test]
    fn test_multiple_sequential_notes() {
        let mut store = MidiStore::new();

        store.add(0.0, note_on(0, 60, 100)).unwrap();
        store.add(1.0, note_off(0, 60, 0)).unwrap();

        store.add(2.0, note_on(0, 60, 90)).unwrap();
        store.add(3.0, note_off(0, 60, 0)).unwrap();

        store.add(4.0, note_on(0, 60, 80)).unwrap();
        store.add(5.0, note_off(0, 60, 0)).unwrap();

        assert_eq!(store.in_flight.len(), 0);
        assert_eq!(store.store.len(), 6);
        assert_eq!(store.notes.len(), 3);
    }

    #[test]
    fn test_overlapping_notes_different_keys() {
        let mut store = MidiStore::new();

        store.add(0.0, note_on(0, 60, 100)).unwrap();
        store.add(0.5, note_on(0, 64, 90)).unwrap();
        store.add(0.75, note_on(0, 67, 80)).unwrap();

        assert_eq!(store.in_flight.len(), 3);

        store.add(1.0, note_off(0, 60, 0)).unwrap();
        assert_eq!(store.in_flight.len(), 2);

        store.add(1.5, note_off(0, 67, 0)).unwrap();
        assert_eq!(store.in_flight.len(), 1);

        store.add(2.0, note_off(0, 64, 0)).unwrap();
        assert_eq!(store.in_flight.len(), 0);

        assert_eq!(store.store.len(), 6);
        assert_eq!(store.notes.len(), 3);
    }

    #[test]
    fn test_note_override_same_key_channel() {
        // Test that NoteOn for same key/channel overrides existing note
        let mut store = MidiStore::new();

        store.add(0.0, note_on(0, 60, 100)).unwrap();
        assert_eq!(store.in_flight.len(), 1);
        assert_eq!(store.in_flight[0].vel, u7::new(100));

        // Send another NoteOn on same key - should override
        store.add(0.5, note_on(0, 60, 80)).unwrap();
        assert_eq!(store.in_flight.len(), 1);
        assert_eq!(store.in_flight[0].vel, u7::new(80));
        assert_eq!(store.in_flight[0].t_start, 0.5);

        store.add(1.0, note_off(0, 60, 0)).unwrap();
        assert_eq!(store.notes.len(), 1);
        assert_eq!(store.notes[0].vel, u7::new(80));
    }

    #[test]
    fn test_multiple_channels() {
        let mut store = MidiStore::new();

        // Channel 0 (0x90 = 10010000, channel 0)
        store.add(0.0, note_on(0, 60, 100)).unwrap();
        // Channel 1 (0x91 = 10010001, channel 1)
        store.add(0.1, note_on(1, 60, 90)).unwrap();

        assert_eq!(store.in_flight.len(), 2);

        // Turn off channel 0
        store.add(1.0, note_off(0, 60, 0)).unwrap();
        assert_eq!(store.in_flight.len(), 1);
        assert_eq!(store.in_flight[0].channel, u4::new(1));

        // Turn off channel 1
        store.add(1.5, note_off(1, 60, 0)).unwrap();
        assert_eq!(store.in_flight.len(), 0);
        assert_eq!(store.notes.len(), 2);
    }

    #[test]
    fn test_note_on_velocity_zero_as_note_off() {
        // NoteOn with velocity 0 should be treated as NoteOff per MIDI 1.0 spec
        let mut store = MidiStore::new();

        store.add(0.0, note_on(0, 60, 100)).unwrap();
        assert_eq!(store.in_flight.len(), 1);

        // NoteOn with vel=0 should properly close the note
        store.add(1.0, note_on(0, 60, 0)).unwrap();
        assert_eq!(store.in_flight.len(), 0);
        assert_eq!(store.notes.len(), 1);
        assert_eq!(store.notes[0].t_end, 1.0);
    }

    #[test]
    fn test_time_order_violation() {
        let mut store = MidiStore::new();

        store.add(1.0, note_on(0, 60, 100)).unwrap();

        // Attempting to add event in the past should fail
        let result = store.add(0.5, note_off(0, 60, 0));
        assert!(result.is_err());
    }

    #[test]
    fn test_note_off_without_note_on() {
        let mut store = MidiStore::new();

        // NoteOff without corresponding NoteOn
        store.add(0.0, note_off(0, 60, 0)).unwrap();

        // Should not crash, note should be ignored (logged)
        assert_eq!(store.in_flight.len(), 0);
        assert_eq!(store.notes.len(), 0);
        assert_eq!(store.store.len(), 1);
    }

    #[test]
    fn test_note_properties() {
        let mut store = MidiStore::new();

        store.add(0.5, note_on(0, 72, 95)).unwrap();
        store.add(1.5, note_off(0, 72, 0)).unwrap();

        assert_eq!(store.notes.len(), 1);
        let note = &store.notes[0];
        assert_eq!(note.key, u7::new(72));
        assert_eq!(note.vel, u7::new(95));
        assert_eq!(note.t_start, 0.5);
        assert_eq!(note.t_end, 1.5);
        assert_eq!(note.idx_on, 0);
        assert_eq!(note.idx_off, 1);
        assert_eq!(note.channel, u4::new(0));
    }

    #[test]
    fn test_non_note_midi_events() {
        // Test that non-note MIDI events are stored but don't affect notes
        let mut store = MidiStore::new();

        // Control Change message (0xB0 = CC on channel 0, 0x07 = volume, 0x40 = value 64)
        store.add(0.0, [0xB0, 0x07, 0x40]).unwrap();

        assert_eq!(store.in_flight.len(), 0);
        assert_eq!(store.notes.len(), 0);
        assert_eq!(store.store.len(), 1);
    }

    #[test]
    fn test_mixed_channels_and_keys() {
        // Complex scenario: overlapping notes on different channels/keys with some overrides
        let mut store = MidiStore::new();

        // Channel 0, key 60
        store.add(0.0, note_on(0, 60, 100)).unwrap();
        // Channel 1, key 60 (different channel, should not conflict)
        store.add(0.2, note_on(1, 60, 90)).unwrap();
        // Channel 0, key 64
        store.add(0.3, note_on(0, 64, 85)).unwrap();
        // Channel 0, key 60 again (override first note)
        store.add(0.5, note_on(0, 60, 75)).unwrap();

        assert_eq!(store.in_flight.len(), 3);

        // Turn off channel 0, key 60 (should remove the overridden note)
        store.add(1.0, note_off(0, 60, 0)).unwrap();
        assert_eq!(store.in_flight.len(), 2);
        assert_eq!(store.notes.len(), 1);
        assert_eq!(store.notes[0].vel, u7::new(75)); // Should be the override velocity

        // Turn off channel 1, key 60
        store.add(1.2, note_off(1, 60, 0)).unwrap();
        assert_eq!(store.in_flight.len(), 1);
        assert_eq!(store.notes.len(), 2);

        // Turn off channel 0, key 64
        store.add(1.5, note_off(0, 64, 0)).unwrap();
        assert_eq!(store.in_flight.len(), 0);
        assert_eq!(store.notes.len(), 3);
    }

    #[test]
    fn test_note_range() {
        let mut store = MidiStore::new();

        // Empty store should return None
        assert_eq!(store.note_range(), None);

        // Single note
        store.add(0.0, note_on(0, 60, 100)).unwrap();
        store.add(1.0, note_off(0, 60, 0)).unwrap();
        assert_eq!(store.note_range(), Some((u7::new(60), u7::new(60))));

        // Multiple notes with range
        store.add(2.0, note_on(0, 55, 100)).unwrap();
        store.add(3.0, note_off(0, 55, 0)).unwrap();
        assert_eq!(store.note_range(), Some((u7::new(55), u7::new(60))));

        store.add(4.0, note_on(0, 72, 100)).unwrap();
        store.add(5.0, note_off(0, 72, 0)).unwrap();
        assert_eq!(store.note_range(), Some((u7::new(55), u7::new(72))));
    }

    #[test]
    fn test_time_range() {
        let mut store = MidiStore::new();

        // Empty store should return None
        assert_eq!(store.time_range(), None);

        // Single note (0.5 - 1.5)
        store.add(0.5, note_on(0, 60, 100)).unwrap();
        store.add(1.5, note_off(0, 60, 0)).unwrap();
        assert_eq!(store.time_range(), Some((0.5, 1.5)));

        // Add earlier note (0.1 - 0.9) - extends minimum
        store.add(2.0, note_on(0, 55, 100)).unwrap();
        store.add(2.9, note_off(0, 55, 0)).unwrap();
        // At this point, we have notes ending at 2.9, so max extends
        assert_eq!(store.time_range(), Some((0.5, 2.9)));

        // Add later note (3.0 - 5.5) - extends maximum
        store.add(3.0, note_on(0, 72, 100)).unwrap();
        store.add(5.5, note_off(0, 72, 0)).unwrap();
        assert_eq!(store.time_range(), Some((0.5, 5.5)));
    }

    #[test]
    fn test_notes_in_time() {
        let mut store = MidiStore::new();

        // Create notes at different time ranges:
        // Note 1: 0.0-1.0
        store.add(0.0, note_on(0, 60, 100)).unwrap();
        store.add(1.0, note_off(0, 60, 0)).unwrap();

        // Note 2: 2.0-3.0
        store.add(2.0, note_on(0, 64, 100)).unwrap();
        store.add(3.0, note_off(0, 64, 0)).unwrap();

        // Note 3: 3.5-5.0
        store.add(3.5, note_on(0, 67, 100)).unwrap();
        store.add(5.0, note_off(0, 67, 0)).unwrap();

        // Query before any notes
        let notes = store.notes_in_time(-1.0, 0.0).collect::<Vec<_>>();
        assert_eq!(notes.len(), 0);

        // Query overlapping only note 1
        let notes = store.notes_in_time(0.5, 1.5).collect::<Vec<_>>();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].key, u7::new(60));

        // Query overlapping note 2 and 3
        let notes = store.notes_in_time(2.5, 4.0).collect::<Vec<_>>();
        assert_eq!(notes.len(), 2);
        let keys: Vec<u8> = notes.iter().map(|n| n.key.as_int()).collect();
        assert!(keys.contains(&64));
        assert!(keys.contains(&67));

        // Query overlapping all notes
        let notes = store.notes_in_time(0.0, 5.0).collect::<Vec<_>>();
        assert_eq!(notes.len(), 3);

        // Query between notes
        let notes = store.notes_in_time(1.5, 2.0).collect::<Vec<_>>();
        assert_eq!(notes.len(), 0);

        // Query starting before note 1 and ending in note 3
        let notes = store.notes_in_time(-0.5, 4.0).collect::<Vec<_>>();
        assert_eq!(notes.len(), 3);

        // Query at exact note boundaries
        let notes = store.notes_in_time(1.0, 2.0).collect::<Vec<_>>();
        assert_eq!(notes.len(), 0);

        // Query just before note 2
        let notes = store.notes_in_time(1.9, 2.1).collect::<Vec<_>>();
        assert_eq!(notes.len(), 1);
        assert_eq!(notes[0].key, u7::new(64));
    }
}
