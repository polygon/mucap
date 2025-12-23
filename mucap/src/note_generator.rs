//! NoteGenerator generates random notes so testing the program does not require
//! any MIDI input. This is useful for testing the plugin's functionality without
//! the need for a MIDI device.

use std::collections::HashMap;

use midly::{MidiMessage, live::LiveEvent, num::u7, io::Cursor};
use rand::{Rng, SeedableRng};
use rand_distr::{Distribution, Normal};

pub struct NoteGenerator {
    active: HashMap<u7, ()>,
    note_min: u7,
    note_max: u7,
    avg: f32,
    stddev: f32,
    exp_note_dist: f32,
    exp_note_length: f32,
    play_penalty: f32,
    pub rng: Option<rand::rngs::SmallRng>,
}

impl Default for NoteGenerator {
    fn default() -> Self {
        Self {
            active: HashMap::with_capacity(128),
            note_min: 21.into(),
            note_max: 108.into(),
            avg: 60.0,
            stddev: 5.,
            exp_note_dist: 0.2,     // Average note distance
            exp_note_length: 0.5,   // Average note length
            play_penalty: 1.25,      // Space out notes further by this factor times number of playing notes
            rng: Some(rand::rngs::SmallRng::seed_from_u64(1)),
        }
    }
}

impl NoteGenerator {
    /// Generates random MIDI notes
    ///
    /// Samples a note from a normal distribution centered at `self.avg` with
    /// standard deviation `self.stddev`, then clamps it to the valid MIDI note range.
    ///
    /// # Arguments
    ///
    /// * `dt` - Delta time in seconds since the last call
    ///
    /// # Returns
    ///
    /// Returns `Some([status, note, velocity])` where status is 0x90 (note-on),
    /// or `None` if the normal distribution cannot be created.
    pub fn generate(&mut self, dt: f32) -> Option<[u8; 3]> {
        // Extract rng from self so we can borrow mutably
        // If there is no rng available here, it's a bug
        let mut rng = self.rng.take().unwrap();

        // The note to generate should we decide to do so later
        let normal = Normal::new(self.avg, self.stddev).ok()?;
        let note = normal.sample(&mut rng) as u8;
        let key = note.clamp(self.note_min.as_int(), self.note_max.as_int()) as u8;
        
        let result = if let Some((key, _)) = self.active.iter().filter(|_| {
            exponential_event(&mut rng,dt, self.exp_note_length)
        }).next() {
            let key = *key;
            self.active.remove(&key);
            let ev = LiveEvent::Midi { channel: 0.into(), message: MidiMessage::NoteOff { key, vel: 64.into() } };
            let mut buf = [0u8; 3];
            let mut cursor = Cursor::new(&mut buf[..]);
            ev.write(&mut cursor).unwrap();
            Some(buf)
        } else if !self.active.contains_key(&key.into()) && exponential_event(&mut rng, dt, self.exp_note_dist * (1.0 + self.play_penalty * self.active.len() as f32))
        {
            self.active.insert(key.into(), ());
            let ev = LiveEvent::Midi { channel: 0.into(), message: MidiMessage::NoteOn {key: key.into(), vel: 64.into() }};
            let mut buf = [0u8; 3];
            let mut cursor = Cursor::new(&mut buf[..]);
            ev.write(&mut cursor).unwrap();
            Some(buf)
        } else {
            None
        };
        self.rng = Some(rng);
        result
    }
}

    fn exponential_event(rng: &mut rand::rngs::SmallRng, dt: f32, tau: f32) -> bool {
        rng.random::<f32>() < 1.0 - f32::exp(-dt / tau)
    }