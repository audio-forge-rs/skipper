use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use std::sync::Arc;

pub struct Skipper {
    params: Arc<SkipperParams>,
}

#[derive(Params)]
struct SkipperParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,
}

impl Default for Skipper {
    fn default() -> Self {
        Self {
            params: Arc::new(SkipperParams::default()),
        }
    }
}

impl Default for SkipperParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(400, 300),
        }
    }
}

impl Plugin for Skipper {
    const NAME: &'static str = "Skipper";
    const VENDOR: &'static str = "bedwards";
    const URL: &'static str = "https://github.com/bedwards/skipper";
    const EMAIL: &'static str = "";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: NonZeroU32::new(2),
            main_output_channels: NonZeroU32::new(2),
            ..AudioIOLayout::const_default()
        },
    ];
    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, _setter, _editor_state| {
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    ui.heading("Skipper");
                    ui.label("Plugin loaded successfully!");
                });
            },
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        _buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        true
    }

    fn process(
        &mut self,
        _buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        ProcessStatus::Normal
    }
}

impl ClapPlugin for Skipper {
    const CLAP_ID: &'static str = "com.bedwards.skipper";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Displays host and track information");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;
    const CLAP_FEATURES: &'static [ClapFeature] = &[ClapFeature::Utility];
}

impl Vst3Plugin for Skipper {
    const VST3_CLASS_ID: [u8; 16] = *b"SkipperInfoPlugn";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Tools];
}

nih_export_clap!(Skipper);
nih_export_vst3!(Skipper);
