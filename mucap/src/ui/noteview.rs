
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::Ordering;

use nih_plug::nih_log;
use nih_plug::prelude::AtomicF32;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;
use crate::midistore::MidiStore;
use crate::midistore::Note;

pub struct NoteView {
    store: Arc<RwLock<MidiStore>>,
        time: Arc<AtomicF32>,
}

impl NoteView {
    pub fn new(cx: &mut Context, store: Arc<RwLock<MidiStore>>, time: Arc<AtomicF32>) -> Handle<'_, Self> {
        Self { store, time }.build(cx, |cx| {
            //Label::new(cx, "This is a custom view!");
        })
    }
}

impl View for NoteView {
    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        //nih_log!("DRAW");
        let b = cx.bounds();
        if b.w == 0.0 || b.h == 0.0 {
            return;
        }

        const SLOTS: usize = 48;

        
        canvas.translate(-b.x, b.y);
        canvas.scale(b.w / 60.0, b.h / (SLOTS+2) as f32);
        //canvas.translate(0., 24.0);
        
        let mut path = vg::Path::new();
        path.rect(0., 0., 60., (SLOTS+2) as f32);
        let paint = vg::Paint::color(vg::Color::rgb(16, 16, 42));
        canvas.fill_path(&path, &paint);
        let paint = vg::Paint::color(vg::Color::rgb(32, 32, 32)).with_line_width(0.05);
        canvas.stroke_path(&path, &paint);

        let mut note_path = vg::Path::new();
        for note in self.store.read().unwrap().notes.iter() {
            note_path.move_to(note.t_start, note.key.as_int() as f32 - 60. + 24.);
            note_path.line_to(note.t_end, note.key.as_int() as f32 - 60. + 24.);
        }
        for note in self.store.read().unwrap().in_flight.iter() {
            note_path.move_to(note.t_start, note.key.as_int() as f32 - 60. + 24.);
            note_path.line_to(self.time.load(Ordering::Relaxed), note.key.as_int() as f32 - 60. + 24.)
        }
        let note_paint = vg::Paint::color(vg::Color::rgb(220, 120, 12)).with_line_width(0.45);
        let note_paint2 = vg::Paint::color(vg::Color::rgb(255, 0, 0)).with_line_width(0.1);
        canvas.stroke_path(&note_path, &note_paint);
        canvas.stroke_path(&note_path, &note_paint2);
        let mut pos_bar = vg::Path::new();
        pos_bar.move_to(self.time.load(Ordering::Relaxed), 0.);
        pos_bar.line_to(self.time.load(Ordering::Relaxed), (SLOTS+2) as f32);
        let bar_paint = vg::Paint::color(vg::Color::rgb(176, 176, 240)).with_line_width(0.1);
        canvas.stroke_path(&pos_bar, &bar_paint);
    }
}
