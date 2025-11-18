
use nih_plug::nih_log;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::vizia::vg;

pub struct NoteView {}

impl NoteView {
    pub fn new(cx: &mut Context) -> Handle<'_, Self> {
        Self {}.build(cx, |cx| {
            Label::new(cx, "This is a custom view!");
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
        
        let aspect = b.w / b.h;

        canvas.translate(-b.x, b.y);
        canvas.scale(b.w / aspect, b.h);
        
        let mut path = vg::Path::new();
        path.rect(0., 0., aspect, 1.);
        let paint = vg::Paint::color(vg::Color::rgb(16, 16, 42));
        canvas.fill_path(&path, &paint);
        let paint = vg::Paint::color(vg::Color::rgb(32, 32, 32)).with_line_width(0.05);
        canvas.stroke_path(&path, &paint);
    }
}
