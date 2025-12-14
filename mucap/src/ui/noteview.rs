use std::num::NonZero;
use std::sync::Arc;
use std::sync::RwLock;
use std::sync::atomic::Ordering;

use crate::midistore::MidiStore;
use crate::midistore::Note;
use crate::ui::zoom_control::ZoomControl;
use nih_plug::nih_log;
use nih_plug::prelude::AtomicF32;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;

use super::miditransfer::MidiTransfers;
use super::style::StyleColors;

pub enum SelectionState {
    None,
    Selecting(f32),
    Selected(f32, f32),
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

impl NoteWindow {
    pub fn note_range(&self) -> (u8, u8) {
        self.note_range
    }

    pub fn set_bounds(&mut self, bounds: BoundingBox) {
        self.bounds = bounds;
        self.update_transforms();
    }

    pub fn set_visible_time(&mut self, visible_time: (f32, f32)) {
        self.visible_time = visible_time;
        self.update_transforms();
    }

    pub fn set_note_range(&mut self, note_range: (u8, u8)) {
        self.note_range = note_range;
        self.update_transforms();
    }

    fn update_transforms(&mut self) {
        let (t0, t1) = self.visible_time;
        let (n0, n1) = self.note_range;

        let X0 = self.bounds.x;
        let X1 = self.bounds.x + self.bounds.w;
        let Y0 = self.bounds.y;
        let Y1 = self.bounds.y + self.bounds.h;

        let x0 = t0;
        let x1 = t1;
        let y0 = n1 as f32 + 1.5;
        let y1 = n0 as f32 - 1.5;

        let ax = (X1 - X0) / (x1 - x0);
        let bx = X0 - ax * x0;
        let ay = (Y1 - Y0) / (y1 - y0);
        let by = Y0 - ay * y0;

        let mut transform = vg::Transform2D::identity();
        transform.scale(ax, ay);
        transform.translate(bx, by);
        self.transform = transform;
        self.inverse = transform.inversed();
    }
}

pub enum SnapMode {
    Snapping,
    Off,
}

pub enum VScrollMode {
    Zoom,
    Pan,
}

pub struct NoteView {
    store: Arc<RwLock<MidiStore>>,
    time: Arc<AtomicF32>,
    zoom_control: ZoomControl,
    note_window: RwLock<NoteWindow>,
    mouse_pos: Option<(f32, f32)>,
    selection: SelectionState,
    t_last_op: f32,
    transfers: MidiTransfers,
    snap: SnapMode,
    vscroll: VScrollMode,
    colors: StyleColors,
}

impl NoteView {
    pub fn new(
        cx: &mut Context,
        store: Arc<RwLock<MidiStore>>,
        time: Arc<AtomicF32>,
    ) -> Handle<'_, Self> {
        Self {
            store: store.clone(),
            time,
            zoom_control: ZoomControl::default(),
            note_window: RwLock::new(NoteWindow::new(
                (0.0, 30.0),
                (60 - 12, 60 + 11),
                BoundingBox::default(),
            )),
            mouse_pos: None,
            selection: SelectionState::None,
            t_last_op: 0.0,
            transfers: MidiTransfers::new(store.clone()),
            vscroll: VScrollMode::Zoom,
            snap: SnapMode::Snapping,
            colors: StyleColors::default(),
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
        })
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

        let mut wnd = match self.note_window.write() {
            Ok(wnd) => wnd,
            Err(_) => return,
        };
        wnd.set_bounds(b);
        wnd.set_visible_time(self.zoom_control.current_range());
        drop(wnd);

        // Drop write lock here to prevent deadlock
        let wnd = self.note_window.read().unwrap();

        let t_now = self.time.load(Ordering::Relaxed);
        let (t0, t1) = self.zoom_control.current_range();

        let mut path = vg::Path::new();
        path.rect(b.x, b.y, b.w, b.h);

        let paint = vg::Paint::box_gradient(
            b.x,
            b.y,
            b.w,
            b.h,
            b.h / 4.,
            b.h / 4.,
            self.colors.bg_light,
            self.colors.bg_dark,
        );
        canvas.fill_path(&path, &paint);

        let mut bar_path = vg::Path::new();
        for bar in self.store.read().unwrap().bars.iter() {
            let x = wnd.time_to_x(bar.t);
            if (x >= 0.0) && (x < b.w) {
                bar_path.move_to(x, 0.);
                bar_path.line_to(x, b.h);
            }
        }
        //let bar_paint = vg::Paint::color(vg::Color::rgb(128, 64, 12));
        if let Some((x, y)) = self.mouse_pos {
            let (w, h) = (b.w * 0.4, b.h * 0.8);

            let bar_paint = vg::Paint::box_gradient(
                x - w / 2.,
                y - h / 2.,
                w,
                h,
                h,
                h,
                self.colors.bar_glow_bright,
                self.colors.bar_glow_dim,
            );
            canvas.stroke_path(&bar_path, &bar_paint);
        }

        let mut note_path = vg::Path::new();
        let mut selected_note_path = vg::Path::new();

        let sel = match self.selection {
            SelectionState::Selecting(mut sel_t0) => {
                let mut sel_t1 = wnd.x_to_time(self.snap(cx.mouse().cursorx));
                (sel_t0, sel_t1) = (sel_t0.min(sel_t1), sel_t0.max(sel_t1));

                Some((sel_t0, sel_t1))
            }
            SelectionState::Selected(sel_t0, sel_t1) => Some((sel_t0, sel_t1)),
            SelectionState::None => None,
        };

        if let Some((sel_t0, sel_t1)) = sel {
            let mut sel_path = vg::Path::new();
            let (sel_x0, sel_x1) = (wnd.time_to_x_coerced(sel_t0), wnd.time_to_x_coerced(sel_t1));
            sel_path.rect(sel_x0, b.y, sel_x1 - sel_x0, b.h);

            let sel_fill = vg::Paint::linear_gradient_stops(
                sel_x0,
                0.,
                sel_x1,
                0.,
                [
                    (0.0, self.colors.selection_bright),
                    (0.2, self.colors.selection_mid_dim),
                    (0.8, self.colors.selection_mid_bright),
                    (1.0, self.colors.selection_bright),
                ],
            );
            canvas.fill_path(&sel_path, &sel_fill);
        }

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
        let note_paint = if let Some((sel_t0, sel_t1)) = sel {
            let (sel_x0, sel_x1) = (wnd.time_to_x_coerced(sel_t0), wnd.time_to_x_coerced(sel_t1));
            let c0 = (sel_x0 - b.x) / b.w;
            let c1 = (sel_x1 - b.x) / b.w;
            let feather = (0.005 as f32).min((sel_x1 - sel_x0).abs() / 5000.0);
            vg::Paint::linear_gradient_stops(
                b.x + 1.0,
                0.,
                b.w + b.x + 1.0,
                0.,
                [
                    (0.0, self.colors.note_unselected),
                    (c0-feather, self.colors.note_unselected),
                    (c0+feather, self.colors.note_selected_bright),
                    (c1-feather, self.colors.note_selected_bright),
                    (c1+feather, self.colors.note_unselected),
                    (1.0, self.colors.note_unselected),
                ],
            )
        } else {
            vg::Paint::color(self.colors.note_unselected)
        };
        let rim_paint = vg::Paint::color(self.colors.note_rim).with_line_width(1.0);
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
                (0.0, self.colors.playhead_transparent),
                (0.5, self.colors.playhead_semi),
                (0.75, self.colors.playhead_opaque),
                (1.0, self.colors.playhead_base),
            ],
        );

        if let Some(mouse_pos) = self.mouse_pos {
            let px = self.snap(mouse_pos.0);
            let mut cursor_bar = vg::Path::new();
            cursor_bar.rect(px - 0.5, b.y, 1.0, b.h);
            let cursor_paint = vg::Paint::color(self.colors.cursor);
            canvas.stroke_path(&cursor_bar, &cursor_paint);
        }

        canvas.fill_path(&pos_bar, &bar_paint);
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|ev, _meta| match ev {
            NoteViewEvent::Update => self.update(),
        });

        if cx.focused() != cx.current() {
            nih_log!("Focusing");
            cx.focus();
        }

        let t_now = self.time.load(Ordering::Relaxed);
        event.map(|ev, _meta| match ev {
            WindowEvent::MouseScroll(x, y) => {
                if x.abs() > 0.1 {
                    self.zoom_control.pan(0.05 * x);
                    self.t_last_op = t_now;
                }
                if y.abs() > 0.1 {
                    use VScrollMode::*;
                    match self.vscroll {
                        Zoom => {
                    if let Some(mouse) = self.mouse_pos {
                        if let Ok(wnd) = self.note_window.read() {
                            let center = wnd.x_to_time(mouse.0);
                            self.zoom_control.zoom(1.0 - 0.05 * y, center);
                        }
                    }                           
                        },
                        Pan => {
                            self.zoom_control.pan(0.05 * y);
                        }
                    }
                    self.t_last_op = t_now;
                }
            }
            WindowEvent::MouseMove(x, y) => {
                self.mouse_pos = Some((*x, *y));
            }
            WindowEvent::MouseLeave => {
                self.mouse_pos = None;
            }
            WindowEvent::MouseDown(button) => {
                let mouse_x = cx.mouse().cursorx;
                match *button {
                    MouseButton::Left => {
                        self.selection = if let Ok(window) = self.note_window.read() {
                            SelectionState::Selecting(window.x_to_time(self.snap(mouse_x)))
                        } else {
                            SelectionState::None
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::MouseUp(button) => {
                let mouse_x = cx.mouse().cursorx;
                match (*button, &self.selection, self.note_window.read()) {
                    (MouseButton::Left, SelectionState::Selecting(t0), Ok(window)) => {
                        let t1 = window.x_to_time(self.snap(mouse_x));
                        self.selection = if (*t0 - t1).abs() > 0.02 {
                            if *t0 < t1 {
                                SelectionState::Selected(*t0, t1)
                            } else {
                                SelectionState::Selected(t1, *t0)
                            }
                        } else {
                            SelectionState::None
                        };
                        if let SelectionState::Selected(t0, t1) = self.selection {
                            let store = self.store.read().unwrap();
                            nih_log!("Pre-Export transport: {:?}", store.transport);
                            self.transfers.new_selection(t0, t1, &store.transport);
                            self.t_last_op = t_now;
                        }
                    }
                    _ => {}
                }
            }
            WindowEvent::KeyDown(code, key) => {
                nih_log!("Key Down: {:?}, {:?}", code, key);
                match key {
                    Some(Key::Shift) => {
                        self.snap = SnapMode::Off;
                        self.vscroll = VScrollMode::Pan;
                    }
                    _ => ()
                }
            }
            WindowEvent::KeyUp(code, key) => {
                nih_log!("Key Up: {:?}, {:?}", code, key);
                match key {
                    Some(Key::Shift) => {
                        self.snap = SnapMode::Snapping;
                        self.vscroll = VScrollMode::Zoom;
                    }
                    _ => ()
                }
            }
            ev => nih_log!("Window Event: {:?}", ev),
        });
    }
}

impl NoteView {
    fn update(&mut self) {
        let t_now = self.time.load(Ordering::Relaxed);

        if let Ok(mut wnd) = self.note_window.write() {
            let (n0, n1) = wnd.note_range();
            if let Some((sn0, sn1)) = self.store.read().unwrap().note_range_u8() {
                if sn0 < n0 {
                    let diff = sn1 - sn0;
                    wnd.set_note_range((sn0, sn0 + 24.max(diff)));
                } else if sn1 > n1 {
                    let diff = sn1 - sn0;
                    wnd.set_note_range((sn1 - 24.max(diff), sn1));
                }
            }
        }

        // Start following playhead after 30 seconds on non-interaction
        if !matches!(self.selection, SelectionState::Selecting(_)) && t_now > self.t_last_op + 30.0
        {
            if t_now > self.zoom_control.current_range().1 - 1. {
                self.zoom_control.set_range((t_now - 20., t_now + 10.));
            }
        }

        self.zoom_control.update_time((0.0, t_now + 30.0));
        self.zoom_control.update(1. / 60.);
    }

    fn snap(&self, x: f32) -> f32 {
        match self.snap {
            SnapMode::Off => return x,
            _ => ()
        };

        let wnd = self.note_window.read().unwrap();
        let tx = wnd.x_to_time(x);
        if let Some(bar) = self.store.read().unwrap().nearest_bar(tx, 1) {
            let total = wnd.visible_time.1 - wnd.visible_time.0;
            let max_snap = total * 0.02;
            let t = if (tx - bar.t).abs() < max_snap { bar.t } else { tx };
            wnd.time_to_x(t)
        } else {
            x
        }
    }
}

impl NoteWindow {
    pub fn new(visible_time: (f32, f32), note_range: (u8, u8), bounds: BoundingBox) -> Self {
        let mut window = Self {
            visible_time,
            note_range,
            bounds,
            transform: vg::Transform2D::identity(),
            inverse: vg::Transform2D::identity(),
        };
        window.update_transforms();
        window
    }

    pub fn note_to_phys(&self, time: f32, key: f32) -> (f32, f32) {
        self.transform.transform_point(time, key as f32)
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
