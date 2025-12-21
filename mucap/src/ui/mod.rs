use nih_plug::{nih_log, nih_dbg};
use nih_plug::prelude::{AtomicF32, Editor};
use nih_plug_vizia::vizia::prelude::*;
use nih_plug_vizia::widgets::*;
use nih_plug_vizia::{ViziaState, ViziaTheming, assets, create_vizia_editor};
use std::sync::{Arc, RwLock};

pub mod noteview;
pub mod zoom_control;
pub mod miditransfer;
pub mod style;
use noteview::NoteView;

use crate::config::ConfigStore;
use crate::midistore::MidiStore;
use crate::ui::noteview::NoteViewEvent;

#[derive(Lens)]
struct Data {
    store: Arc<RwLock<MidiStore>>,
    config: Arc<RwLock<ConfigStore>>,
    time: Arc<AtomicF32>,
}

impl Model for Data {}

pub(crate) fn default_state(scale_factor: f32) -> Arc<ViziaState> {
    ViziaState::new_with_default_scale_factor(|| (700, 200), scale_factor as f64)
}

pub(crate) fn create(
    editor_state: Arc<ViziaState>,
    store: Arc<RwLock<MidiStore>>,
    config: Arc<RwLock<ConfigStore>>,
    time: Arc<AtomicF32>,
) -> Option<Box<dyn Editor>> {
    create_vizia_editor(editor_state, ViziaTheming::Custom, move |cx, _| {
        assets::register_noto_sans_light(cx);
        assets::register_noto_sans_thin(cx);

        Data {
            store: store.clone(),
            time: time.clone(),
            config: config.clone(),
        }
        .build(cx);

        VStack::new(cx, |cx| {
            /*Label::new(cx, "Drag")
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
                });*/
            NoteView::new(cx, store.clone(), config.clone(), time.clone())
                .width(Stretch(1.0))
                .height(Stretch(1.0));
        })
        .width(Stretch(1.0))
        .height(Stretch(1.0));

        ResizeHandle::new(cx);
        nih_log!("User Scale Factor at start: {}", cx.user_scale_factor());
    })
}
