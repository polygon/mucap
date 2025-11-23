use std::f32;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::Ordering;

use crate::midistore::MidiStore;
use crate::midistore::Note;
use nih_plug::nih_log;
use nih_plug::prelude::AtomicF32;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;

pub enum NoteViewMode {

}

pub enum NoteViewEvent {
    Update,
}

pub struct NoteWindow {
    ///! Provides coordinate translations between time-note space and canvas coordinates
    visible_time: (f32, f32),
    note_range: (u8, u8),
    bounds: BoundingBox,
    transform: vg::Transform2D,
    inverse: vg::Transform2D,
}

pub struct NoteView {
    store: Arc<RwLock<MidiStore>>,
    time: Arc<AtomicF32>,
    visible_time: (f32, f32),
    note_range: (u8, u8),
}

impl NoteView {
    pub fn new(
        cx: &mut Context,
        store: Arc<RwLock<MidiStore>>,
        time: Arc<AtomicF32>,
    ) -> Handle<'_, Self> {
        Self {
            store,
            time,
            visible_time: (0., 30.),
            note_range: (60 - 12, 60 + 11),
        }
        .build(cx, |cx| {
            nih_log!("Spawning Timer Worker!");
            cx.spawn(|cx| {
                nih_log!("Timer worker spawned!");
                while let Ok(_) = cx.emit(NoteViewEvent::Update) {
                    std::thread::sleep(std::time::Duration::from_secs_f32(1. / 60.));
                }
                nih_log!("Timer worker terminating!");
            });
            //Label::new(cx, "This is a custom view!");
        }).on_mouse_down(|cx, button| nih_log!("Mouse Down!, {:?} {:?}", cx.mouse(),button))
    }
}

impl View for NoteView {
    fn element(&self) -> Option<&'static str> {
        Some("noteview")
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &mut Canvas) {
        //nih_log!("DRAW");
        let b = cx.bounds();
        if b.w == 0.0 || b.h == 0.0 {
            return;
        }

        let wnd = NoteWindow::new(self.visible_time, self.note_range, b);

        let t_now = self.time.load(Ordering::Relaxed);
        let (t0, t1) = self.visible_time;

        let mut path = vg::Path::new();
        path.rect(b.x, b.y, b.w, b.h);

        let paint = vg::Paint::box_gradient(
            b.x,
            b.y,
            b.w,
            b.h,
            b.h / 4.,
            b.h / 4.,
            vg::Color::rgb(16, 16, 42),
            vg::Color::rgb(0, 0, 16),
        );
        canvas.fill_path(&path, &paint);

        let mut note_path = vg::Path::new();

        for note in self.store.read().unwrap().notes_in_time(t0, t1) {
            if let Some(trnsf) = wnd.note_to_rect(note) {
                note_path.rect(trnsf.x, trnsf.y + 4., trnsf.w, trnsf.h - 8.);
            }
        }
        for note in self.store.read().unwrap().in_flight.iter() {
            if let Some(trnsf) = wnd.incomplete_note_to_rect(note, t_now) {
                note_path.rect(trnsf.x, trnsf.y + 4., trnsf.w, trnsf.h - 8.);
            }
        }
        let note_paint = vg::Paint::color(vg::Color::rgb(220, 120, 12));
        let rim_paint = vg::Paint::color(vg::Color::rgb(232, 232, 232)).with_line_width(1.0);
        canvas.fill_path(&note_path, &note_paint);
        canvas.stroke_path(&note_path, &rim_paint);

        let mut pos_bar = vg::Path::new();
        /*pos_bar.move_to(self.time.load(Ordering::Relaxed), 0.);
        pos_bar.line_to(self.time.load(Ordering::Relaxed), (SLOTS + 2) as f32);
        let bar_paint = vg::Paint::color(vg::Color::rgb(176, 176, 240)).with_line_width(0.1);*/
        let x0 = wnd.time_to_x(t_now - 2.0);
        let x1 = wnd.time_to_x(t_now) + 1.;
        pos_bar.rect(x0, b.y, x1 - x0, b.h);
        let bar_paint = vg::Paint::linear_gradient_stops(
            x0,
            0.,
            x1,
            0.,
            [
                (0.0, vg::Color::rgba(92, 92, 128, 0)),
                (0.5, vg::Color::rgba(92, 92, 128, 64)),
                (0.75, vg::Color::rgba(92, 92, 128, 128)),
                (1.0, vg::Color::rgba(92, 92, 128, 255)),
            ],
        );
        canvas.fill_path(&pos_bar, &bar_paint);
    }

    fn event(&mut self, _cx: &mut EventContext, event: &mut Event) {
        event.map(|ev, _meta| match ev {
            NoteViewEvent::Update => self.update(),
        })
    }
}

impl NoteView {
    fn update(&mut self) {
        let t_now = self.time.load(Ordering::Relaxed);
        let (t0, t1) = self.visible_time;
        if t_now > t1 {
            self.visible_time = (t0 + 15., t1 + 15.);
        }

        let (n0, n1) = self.note_range;
        if let Some((sn0, sn1)) = self.store.read().unwrap().note_range_u8() {
            if sn0 < n0 {
                let diff = sn1 - sn0;
                self.note_range = (sn0, sn0 +24.max(diff));
            } else if sn1 > n1 {
                let diff = sn1 - sn0;
                self.note_range = (sn1 - 24.max(diff), sn1);
            }
        }
    }
}

impl NoteWindow {
    pub fn new(visible_time: (f32, f32), note_range: (u8, u8), bounds: BoundingBox) -> Self {
        let (t0, t1) = visible_time;
        let (n0, n1) = note_range;

        let X0 = bounds.x;
        let X1 = bounds.x + bounds.w;
        let Y0 = bounds.y;
        let Y1 = bounds.y + bounds.h;

        let x0 = t0;
        let x1 = t1;
        let y0 = n1 as f32 + 1.5;
        let y1 = n0 as f32 - 1.5;

        let ax = (X1 - Y0) / (x1 - x0);
        let bx = X0 - ax * x0;
        let ay = (Y1 - Y0) / (y1 - y0);
        let by = Y0 - ay * y0;


        let mut transform = vg::Transform2D::identity();
        transform.scale(ax, ay);
        transform.translate(bx, by);
        let inverse = transform.inversed();
        Self {
            visible_time,
            note_range,
            bounds,
            transform,
            inverse,
        }
    }

    pub fn note_to_phys(&self, time: f32, key: f32) -> (f32, f32) {
        self.transform.transform_point(
            time,
            key as f32
        )
    }

    pub fn note_to_phys_coerced(&self, time: f32, key: f32) -> (f32, f32) {
        let (x, y) = self.note_to_phys(time, key);
        (
            x.clamp(self.bounds.x, self.bounds.x + self.bounds.w),
            y.clamp(self.bounds.y, self.bounds.y + self.bounds.h),
        )
    }

    pub fn time_to_x(&self, time: f32) -> f32 {
        self.note_to_phys(time, 0.).0
    }

    pub fn time_to_x_coerced(&self, time: f32) -> f32 {
        self.note_to_phys_coerced(time, 0.).0
    }

    pub fn note_to_rect(&self, note: &Note) -> Option<BoundingBox> {
        let (t0, t1, key) = (note.t_start, note.t_end, note.key);
        if (t0 > self.visible_time.1)
            || (t1 < self.visible_time.0)
            || (key < self.note_range.0)
            || (key > self.note_range.1)
        {
            return None;
        }

        let key = key.as_int() as f32;

        let tl = self.note_to_phys_coerced(t0, key + 0.5);
        let br = self.note_to_phys_coerced(t1, key - 0.5);

        Some(BoundingBox::from_min_max(tl.0, tl.1, br.0, br.1))
    }

    pub fn incomplete_note_to_rect(&self, note: &Note, t_now: f32) -> Option<BoundingBox> {
        let mut n = note.clone();
        n.t_end = t_now;
        self.note_to_rect(&n)
    }

    pub fn x_to_time(&self, x: f32) -> f32 {
        self.inverse.transform_point(x, 0.).0
    }
}
