use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::io::Write;

/// Global counter for unique plugin instance IDs
static INSTANCE_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Log to ~/skipper-logs/skipper-plugin-<id>.log
fn log_to_file(instance_id: u32, msg: &str) {
    if let Some(home) = std::env::var_os("HOME") {
        let log_dir = std::path::Path::new(&home).join("skipper-logs");
        let _ = std::fs::create_dir_all(&log_dir);
        let filename = format!("skipper-plugin-{}.log", instance_id);
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_dir.join(&filename))
        {
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = writeln!(file, "[{}] {}", timestamp, msg);
        }
    }
}

/// Transport state from process context
#[derive(Default, Clone)]
struct TransportState {
    tempo: Option<f64>,
    time_sig_numerator: Option<i32>,
    time_sig_denominator: Option<i32>,
    pos_samples: Option<i64>,
    pos_beats: Option<f64>,
    pos_seconds: Option<f64>,
    playing: bool,
    recording: bool,
    loop_active: bool,
    loop_start_beats: Option<f64>,
    loop_end_beats: Option<f64>,
}

/// Shared state between plugin and GUI
struct SharedState {
    host_info: Option<HostInfo>,
    track_info: Option<TrackInfo>,
    transport: TransportState,
    sample_rate: f32,
    buffer_size: u32,
    plugin_api: PluginApi,
}

impl Default for SharedState {
    fn default() -> Self {
        Self {
            host_info: None,
            track_info: None,
            transport: TransportState::default(),
            sample_rate: 44100.0,
            buffer_size: 512,
            plugin_api: PluginApi::Clap,
        }
    }
}

/// The Skipper plugin - displays host and track information
pub struct Skipper {
    params: Arc<SkipperParams>,
    state: Arc<RwLock<SharedState>>,
    instance_id: u32,
}

#[derive(Params)]
struct SkipperParams {
    #[persist = "editor-state"]
    editor_state: Arc<EguiState>,
}

impl Default for Skipper {
    fn default() -> Self {
        let instance_id = INSTANCE_COUNTER.fetch_add(1, Ordering::SeqCst);
        log_to_file(instance_id, "Skipper instance created");
        Self {
            params: Arc::new(SkipperParams::default()),
            state: Arc::new(RwLock::new(SharedState::default())),
            instance_id,
        }
    }
}

impl Default for SkipperParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(520, 600),
        }
    }
}

/// Build all display information as a single copyable text string
fn build_info_text(shared: &SharedState) -> String {
    let mut lines = Vec::new();

    lines.push("==================================================".to_string());
    lines.push("SKIPPER - DAW Info Display".to_string());
    lines.push("==================================================".to_string());
    lines.push(String::new());

    // Track name (prominent)
    if let Some(ref track) = shared.track_info {
        let track_name = match &track.name {
            Some(name) if !name.is_empty() => name.clone(),
            _ => "(rename track in DAW)".to_string(),
        };
        lines.push(format!("TRACK: {}", track_name));
        if let Some((r, g, b)) = track.color {
            lines.push(format!("COLOR: #{:02X}{:02X}{:02X}", r, g, b));
        }
    } else {
        lines.push("TRACK: (requires CLAP + host track-info)".to_string());
    }
    lines.push(String::new());

    // Plugin info
    lines.push("--------------------------------------------------".to_string());
    lines.push("PLUGIN".to_string());
    lines.push("--------------------------------------------------".to_string());
    lines.push(format!("Format:      {}", shared.plugin_api));
    lines.push(format!("Name:        {}", Skipper::NAME));
    lines.push(format!("Version:     {}", Skipper::VERSION));
    lines.push(format!("Sample Rate: {:.0} Hz", shared.sample_rate));
    lines.push(format!("Buffer Size: {} samples", shared.buffer_size));
    lines.push(String::new());

    // Host info
    lines.push("--------------------------------------------------".to_string());
    lines.push("HOST".to_string());
    lines.push("--------------------------------------------------".to_string());
    if let Some(ref host) = shared.host_info {
        lines.push(format!("Name:    {}", if host.name.is_empty() { "(not provided)" } else { &host.name }));
        lines.push(format!("Vendor:  {}", if host.vendor.is_empty() { "(not provided)" } else { &host.vendor }));
        lines.push(format!("Version: {}", if host.version.is_empty() { "(not provided)" } else { &host.version }));
    } else {
        lines.push("(not available - requires CLAP format)".to_string());
    }
    lines.push(String::new());

    // Track info details
    lines.push("--------------------------------------------------".to_string());
    lines.push("TRACK INFO".to_string());
    lines.push("--------------------------------------------------".to_string());
    if let Some(ref track) = shared.track_info {
        let track_name = match &track.name {
            Some(name) if !name.is_empty() => name.clone(),
            Some(_) => "(empty - rename in DAW)".to_string(),
            None => "(host doesn't provide)".to_string(),
        };
        lines.push(format!("Name:       {}", track_name));

        let color_str = match track.color {
            Some((r, g, b)) => format!("#{:02X}{:02X}{:02X}", r, g, b),
            None => "(not provided)".to_string(),
        };
        lines.push(format!("Color:      {}", color_str));

        let channels = match track.audio_channel_count {
            Some(count) => format!("{}", count),
            None => "(not provided)".to_string(),
        };
        lines.push(format!("Channels:   {}", channels));

        let track_type = if track.is_for_master {
            "Master"
        } else if track.is_for_return_track {
            "Return/Aux"
        } else if track.is_for_bus {
            "Bus"
        } else {
            "Regular"
        };
        lines.push(format!("Track Type: {}", track_type));
    } else {
        lines.push("(not available - requires CLAP + host track-info)".to_string());
    }
    lines.push(String::new());

    // Transport
    lines.push("--------------------------------------------------".to_string());
    lines.push("TRANSPORT".to_string());
    lines.push("--------------------------------------------------".to_string());

    let play_status = if shared.transport.playing { "PLAYING" } else { "STOPPED" };
    let rec_status = if shared.transport.recording { " | REC" } else { "" };
    let loop_status = if shared.transport.loop_active { " | LOOP" } else { "" };
    lines.push(format!("Status: {}{}{}", play_status, rec_status, loop_status));

    let tempo = match shared.transport.tempo {
        Some(t) => format!("{:.2} BPM", t),
        None => "(not available)".to_string(),
    };
    lines.push(format!("Tempo:    {}", tempo));

    let time_sig = match (shared.transport.time_sig_numerator, shared.transport.time_sig_denominator) {
        (Some(num), Some(den)) => format!("{}/{}", num, den),
        _ => "(not available)".to_string(),
    };
    lines.push(format!("Time Sig: {}", time_sig));

    let position = match shared.transport.pos_beats {
        Some(beats) => {
            let time_sig_num = shared.transport.time_sig_numerator.unwrap_or(4) as f64;
            let bars = (beats / time_sig_num).floor() as i32 + 1;
            let beat_in_bar = (beats % time_sig_num) + 1.0;
            format!("Bar {} | Beat {:.2}", bars, beat_in_bar)
        }
        None => "(not available)".to_string(),
    };
    lines.push(format!("Position: {}", position));

    let time = match shared.transport.pos_seconds {
        Some(secs) => {
            let mins = (secs / 60.0).floor() as i32;
            let remaining_secs = secs % 60.0;
            format!("{}:{:05.2}", mins, remaining_secs)
        }
        None => "(not available)".to_string(),
    };
    lines.push(format!("Time:     {}", time));

    lines.join("\n")
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
        let state = self.state.clone();

        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, _setter, _editor_state| {
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    let shared = state.read();
                    egui_ctx.request_repaint();

                    let info_text = build_info_text(&shared);

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            ui.add(egui::Label::new(
                                egui::RichText::new(&info_text).monospace()
                            ).selectable(true));
                        });
                });
            },
        )
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        context: &mut impl InitContext<Self>,
    ) -> bool {
        let id = self.instance_id;
        let api = context.plugin_api();
        let host_info = context.host_info();
        let track_info = context.track_info();

        log_to_file(id, &format!("Initializing - API={:?}", api));
        log_to_file(id, &format!("host_info={:?}", host_info));
        log_to_file(id, &format!("track_info={:?}", track_info));

        {
            let mut state = self.state.write();
            state.sample_rate = buffer_config.sample_rate;
            state.buffer_size = buffer_config.max_buffer_size;
            state.plugin_api = api;
            state.host_info = host_info;
            state.track_info = track_info;
        }

        log_to_file(id, "Initialized successfully");
        true
    }

    fn process(
        &mut self,
        _buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        let track_info = context.track_info();
        let transport = context.transport();

        {
            let mut state = self.state.write();

            if track_info != state.track_info {
                if let Some(ref ti) = track_info {
                    log_to_file(self.instance_id, &format!(
                        "Track info updated: name={:?}", ti.name
                    ));
                }
                state.track_info = track_info;
            }

            state.transport.tempo = transport.tempo;
            state.transport.time_sig_numerator = transport.time_sig_numerator;
            state.transport.time_sig_denominator = transport.time_sig_denominator;
            state.transport.pos_samples = transport.pos_samples();
            state.transport.pos_beats = transport.pos_beats();
            state.transport.pos_seconds = transport.pos_seconds();
            state.transport.playing = transport.playing;
            state.transport.recording = transport.recording;
            state.transport.loop_active = transport.loop_range_beats().is_some();
            if let Some((start, end)) = transport.loop_range_beats() {
                state.transport.loop_start_beats = Some(start);
                state.transport.loop_end_beats = Some(end);
            }
        }

        ProcessStatus::Normal
    }

    fn deactivate(&mut self) {
        log_to_file(self.instance_id, "Deactivated");
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
