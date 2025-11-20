use nih_plug::{midi::MidiResult, prelude::*};
use nih_plug_vizia::ViziaState;
use std::sync::{Arc, RwLock, atomic::Ordering, mpsc};

mod midistore;
mod note_generator;
mod ui;

use midistore::MidiStore;
use note_generator::NoteGenerator;

type Samples = i64;

/// A plugin that inverts all MIDI note numbers, channels, CCs, velocities, pressures, and
/// everything else you don't want to be inverted.
pub struct Mucap {
    params: Arc<MucapParams>,
    samples: Samples,
    time: Arc<AtomicF32>,
    store: Arc<RwLock<MidiStore>>,
    tx: Option<mpsc::SyncSender<(f32, [u8; 3])>>,
    note_delivery_thread: Option<std::thread::JoinHandle<()>>,
    generator: NoteGenerator,
}

#[derive(Params)]
struct MucapParams {
    #[persist = "editor-state"]
    editor_state: Arc<ViziaState>,
}

impl Default for Mucap {
    fn default() -> Self {
        let store = Arc::new(RwLock::new(MidiStore::new()));
        Self {
            params: Arc::new(MucapParams::default()),
            samples: 0,
            time: Arc::new(AtomicF32::new(0.0)),
            store: store.clone(),
            tx: None,
            note_delivery_thread: None,
            generator: NoteGenerator::default(),
        }
    }
}

impl Default for MucapParams {
    fn default() -> Self {
        Self {
            editor_state: ui::default_state(),
        }
    }
}

pub struct NoteDeliveryTask {
    rx: mpsc::Receiver<(f32, [u8; 3])>,
    store: Arc<RwLock<MidiStore>>,
}

impl NoteDeliveryTask {
    fn run(&mut self) {
        nih_log!("Hello from the background");
        while let Ok((time, event)) = self.rx.recv() {
            self.store
                .write()
                .unwrap()
                .add(time, event)
                .expect("Failed to add event");
        }
    }
}

impl Plugin for Mucap {
    const NAME: &'static str = "MuCap";
    const VENDOR: &'static str = "Matelab";
    const URL: &'static str = "https://github.com/polygon/mucap";
    const EMAIL: &'static str = "";

    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    // This plugin doesn't have any audio IO
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[];

    const MIDI_INPUT: MidiConfig = MidiConfig::MidiCCs;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::MidiCCs;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = NoteDeliveryTask;

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        ui::create(self.params.editor_state.clone(), self.store.clone(), self.time.clone())
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        if self.note_delivery_thread.is_none() {
            let (tx, rx) = mpsc::sync_channel(16);
            let store = self.store.clone();
            self.note_delivery_thread = Some(std::thread::spawn(|| {
                let mut task = NoteDeliveryTask { rx, store };
                task.run();
            }));
            self.tx = Some(tx);
        };
        true
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // We'll invert the channel, note index, velocity, pressure, CC value, pitch bend, and
        // anything else that is invertable for all events we receive
        while let Some(event) = context.next_event() {
            let ev_samples = self.samples + event.timing() as i64;
            let ev_time = ev_samples as f32 / context.transport().sample_rate;
            if let Some(MidiResult::Basic(buf)) = event.as_midi() {
                self.store.write().unwrap().add(ev_time, buf).unwrap_or(());
            }
            //nih_log!("Event @ {:.6}: {:?}", ev_time, event.as_midi());
        }

        if let Some(buf) = self
            .generator
            .generate(buffer.samples() as f32 / context.transport().sample_rate)
        {
            let ev_time = self.samples as f32 / context.transport().sample_rate;
            self.store.write().unwrap().add(ev_time, buf).unwrap_or(());
        }

        self.samples += buffer.samples() as Samples;
        self.time.store(
            self.samples as f32 / context.transport().sample_rate,
            Ordering::Relaxed,
        );

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Mucap {
    const CLAP_ID: &'static str = "de.matelab.mucap";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Saves your MIDI");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] =
        &[ClapFeature::NoteDetector, ClapFeature::Utility];
}

impl Vst3Plugin for Mucap {
    const VST3_CLASS_ID: [u8; 16] = *b"MutscheKippchen.";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument, Vst3SubCategory::Tools];
}

nih_export_clap!(Mucap);
nih_export_vst3!(Mucap);
