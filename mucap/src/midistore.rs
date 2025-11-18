use anyhow::Result;
use midly::MidiMessage;
use midly::live::LiveEvent;
use midly::num::{u4, u7};
use nih_plug::midi::{NoteEvent, sysex::SysExMessage};
use nih_plug::nih_log;

pub struct Note {
    t_start: f32,
    idx_on: usize,
    t_end: f32,
    idx_off: usize,
    channel: u4,
    key: u7,
    vel: u7,
}

pub enum StoreEntry {
    MidiData { channel: u4, data: MidiMessage },
}

pub struct MidiStore {
    store: Vec<(f32, StoreEntry)>,
    notes: Vec<Note>,
    in_flight: Vec<Note>,
}

impl MidiStore {
    pub fn new() -> Self {
        Self {
            store: Vec::with_capacity(60000),
            notes: Vec::with_capacity(10000),
            in_flight: Vec::with_capacity(128 * 16),
        }
    }

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
                self.notes.push(note);
            } else {
                nih_log!("Note Off without Note On @ {:.6}", time);
            }
        }
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
}
