use nih_plug::nih_log;
use nih_plug::prelude::Editor;
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{ViziaState, ViziaTheming, assets, create_vizia_editor};
use std::sync::{Arc, RwLock};

pub mod noteview;
use noteview::NoteView;

use crate::midistore::MidiStore;

#[derive(Lens)]
struct Data {
    store: Arc<RwLock<MidiStore>>
}

impl Model for Data {}

pub(crate) fn default_state() -> Arc<ViziaState> {
    ViziaState::new(|| (1920, 800))
}

pub(crate) fn create(editor_state: Arc<ViziaState>, store: Arc<RwLock<MidiStore>>) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        Data { store: store.clone() }.build(cx);

        VStack::new(cx, |cx| {
            Label::new(cx, "Drag")
                .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                .font_weight(FontWeightKeyword::Thin)
                .font_size(30.0)
                .height(Pixels(50.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0))
                .on_drag(|cx| {
                    cx.set_drop_data(DropData::File("/home/jan/test.mid".into()));
                    nih_log!(
                        "Drag started: {}, {}",
                        cx.is_draggable(),
                        cx.has_drop_data()
                    );
                });
            Label::new(cx, "Drop")
                .font_family(vec![FamilyOwned::Name(String::from(assets::NOTO_SANS))])
                .font_weight(FontWeightKeyword::Thin)
                .font_size(30.0)
                .height(Pixels(50.0))
                .child_top(Stretch(1.0))
                .child_bottom(Pixels(0.0))
                .on_drop(|cx, data| {
                    nih_log!("Drop started: {:?}", data);
                });
            NoteView::new(cx)
                .width(Stretch(1.0))
                .height(Stretch(1.0));
        }).width(Stretch(1.0)).height(Stretch(1.0));

        ResizeHandle::new(cx);
    })
}
