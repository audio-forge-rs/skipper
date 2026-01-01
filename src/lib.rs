use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use parking_lot::RwLock;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

/// Global counter for unique plugin instance IDs
static INSTANCE_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Which tab is currently selected
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Tab {
    Live = 0,
    Info = 1,
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
    track_info: Option<Arc<TrackInfo>>,
    transport: TransportState,
    sample_rate: f32,
    buffer_size: u32,
    plugin_api: PluginApi,
    current_tab: Tab,
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
            current_tab: Tab::Live,
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
        nih_log!("Skipper v{} instance created (id={})", env!("CARGO_PKG_VERSION"), instance_id);
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

/// Render the live dynamic tab with visual elements
fn render_live_tab(ui: &mut egui::Ui, shared: &SharedState) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Track section with color
            ui.heading("Track");
            ui.add_space(4.0);

            if let Some(ref track) = shared.track_info {
                // Track color bar
                if let Some((r, g, b)) = track.color {
                    let color = egui::Color32::from_rgb(r, g, b);
                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), 24.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().rect_filled(rect, 4.0, color);

                    // Track name on color bar
                    let track_name = track.name.as_deref().unwrap_or("(unnamed)");
                    let text_color = if (r as u32 + g as u32 + b as u32) > 384 {
                        egui::Color32::BLACK
                    } else {
                        egui::Color32::WHITE
                    };
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        track_name,
                        egui::FontId::proportional(16.0),
                        text_color,
                    );
                } else {
                    let track_name = track.name.as_deref().unwrap_or("(unnamed)");
                    ui.label(egui::RichText::new(track_name).size(18.0).strong());
                }

                ui.add_space(4.0);

                // Track type badge
                let track_type = if track.is_for_master {
                    "Master"
                } else if track.is_for_return_track {
                    "Return"
                } else if track.is_for_bus {
                    "Bus"
                } else {
                    "Track"
                };
                ui.label(format!("Type: {}", track_type));
            } else {
                ui.label("(No track info - requires CLAP)");
            }

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // Transport section
            ui.heading("Transport");
            ui.add_space(8.0);

            // Play/Stop/Record status with visual indicators
            ui.horizontal(|ui| {
                let play_color = if shared.transport.playing {
                    egui::Color32::from_rgb(0, 200, 0)
                } else {
                    egui::Color32::GRAY
                };
                let rec_color = if shared.transport.recording {
                    egui::Color32::from_rgb(255, 50, 50)
                } else {
                    egui::Color32::GRAY
                };
                let loop_color = if shared.transport.loop_active {
                    egui::Color32::from_rgb(100, 150, 255)
                } else {
                    egui::Color32::GRAY
                };

                ui.label(egui::RichText::new("PLAY").color(play_color).strong());
                ui.label(egui::RichText::new("REC").color(rec_color).strong());
                ui.label(egui::RichText::new("LOOP").color(loop_color).strong());
            });

            ui.add_space(8.0);

            // Tempo and time signature
            ui.horizontal(|ui| {
                if let Some(tempo) = shared.transport.tempo {
                    ui.label(egui::RichText::new(format!("{:.1} BPM", tempo)).size(24.0).strong());
                }
                if let (Some(num), Some(den)) = (shared.transport.time_sig_numerator, shared.transport.time_sig_denominator) {
                    ui.label(egui::RichText::new(format!("{}/{}", num, den)).size(20.0));
                }
            });

            ui.add_space(8.0);

            // Position
            if let Some(beats) = shared.transport.pos_beats {
                let time_sig_num = shared.transport.time_sig_numerator.unwrap_or(4) as f64;
                let bars = (beats / time_sig_num).floor() as i32 + 1;
                let beat_in_bar = (beats % time_sig_num) + 1.0;
                ui.label(egui::RichText::new(format!("Bar {} : Beat {:.2}", bars, beat_in_bar)).size(18.0).monospace());
            }

            if let Some(secs) = shared.transport.pos_seconds {
                let mins = (secs / 60.0).floor() as i32;
                let remaining_secs = secs % 60.0;
                ui.label(egui::RichText::new(format!("{}:{:05.2}", mins, remaining_secs)).size(16.0).monospace());
            }

            ui.add_space(16.0);
            ui.separator();
            ui.add_space(8.0);

            // Plugin/Host info section
            ui.heading("Plugin");
            ui.label(format!("{} v{}", Skipper::NAME, Skipper::VERSION));
            ui.label(format!("Format: {}", shared.plugin_api));
            ui.label(format!("Sample Rate: {:.0} Hz | Buffer: {} samples", shared.sample_rate, shared.buffer_size));

            if let Some(ref host) = shared.host_info {
                ui.add_space(8.0);
                ui.heading("Host");
                if !host.name.is_empty() {
                    ui.label(format!("{} {}", host.name, host.version));
                }
            }
        });
}

impl Plugin for Skipper {
    const NAME: &'static str = "Skipper";
    const VENDOR: &'static str = "Audio Forge RS";
    const URL: &'static str = "https://audio-forge-rs.github.io/";
    const EMAIL: &'static str = "brian.mabry.edwards@gmail.com";
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
        nih_log!("Skipper editor() called (id={})", self.instance_id);
        let state = self.state.clone();

        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, _setter, _editor_state| {
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    egui_ctx.request_repaint();

                    // Tab bar
                    let current_tab = {
                        let shared = state.read();
                        shared.current_tab
                    };

                    ui.horizontal(|ui| {
                        if ui.selectable_label(current_tab == Tab::Live, "Live").clicked() {
                            state.write().current_tab = Tab::Live;
                        }
                        if ui.selectable_label(current_tab == Tab::Info, "Info").clicked() {
                            state.write().current_tab = Tab::Info;
                        }
                    });

                    ui.separator();

                    let shared = state.read();

                    match shared.current_tab {
                        Tab::Live => {
                            render_live_tab(ui, &shared);
                        }
                        Tab::Info => {
                            let info_text = build_info_text(&shared);
                            egui::ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .show(ui, |ui| {
                                    ui.add(egui::Label::new(
                                        egui::RichText::new(&info_text).monospace()
                                    ).selectable(true));
                                });
                        }
                    }
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
        let api = context.plugin_api();
        let host_info = context.host_info();
        let track_info = context.track_info();

        nih_log!("========================================");
        nih_log!("Skipper v{} initialize() id={}", env!("CARGO_PKG_VERSION"), self.instance_id);
        nih_log!("========================================");
        nih_log!("API: {:?}", api);
        nih_log!("Sample Rate: {} Hz", buffer_config.sample_rate);
        nih_log!("Buffer Size: {} samples", buffer_config.max_buffer_size);

        // Log host info
        if let Some(ref host) = host_info {
            nih_log!("Host: {} {} ({})", host.name, host.version, host.vendor);
        } else {
            nih_log!("Host: (no host info available)");
        }

        // Log track info in detail
        if let Some(ref track) = track_info {
            nih_log!("Track Name: {:?}", track.name);
            if let Some((r, g, b)) = track.color {
                nih_log!("Track Color: #{:02X}{:02X}{:02X} (RGB: {}, {}, {})", r, g, b, r, g, b);
            } else {
                nih_log!("Track Color: (not provided)");
            }
            nih_log!("Track Type: master={}, return={}, bus={}",
                track.is_for_master, track.is_for_return_track, track.is_for_bus);
            if let Some(ch) = track.audio_channel_count {
                nih_log!("Track Channels: {}", ch);
            }
        } else {
            nih_log!("Track: (no track info available)");
        }

        {
            let mut state = self.state.write();
            state.sample_rate = buffer_config.sample_rate;
            state.buffer_size = buffer_config.max_buffer_size;
            state.plugin_api = api;
            state.host_info = host_info;
            state.track_info = track_info;
        }

        nih_log!("Skipper initialized successfully (id={})", self.instance_id);
        nih_log!("========================================");
        true
    }

    fn process(
        &mut self,
        _buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        // NO LOGGING HERE - audio thread forbids allocation
        // NOTE: Don't update track_info here - it's set in initialize() and updated
        // via CLAP changed() callback. Updating here would deallocate on audio thread.
        let transport = context.transport();

        {
            let mut state = self.state.write();

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
        nih_log!("Skipper deactivated (id={})", self.instance_id);
    }
}

impl ClapPlugin for Skipper {
    const CLAP_ID: &'static str = "rs.audio-forge.skipper";
    const CLAP_DESCRIPTION: Option<&'static str> = Some("Displays host and track information");
    const CLAP_MANUAL_URL: Option<&'static str> = Some("https://audio-forge-rs.github.io/");
    const CLAP_SUPPORT_URL: Option<&'static str> = Some("https://github.com/audio-forge-rs/skipper");
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::NoteEffect,
        ClapFeature::Utility,
        ClapFeature::Analyzer,
    ];
}

impl Vst3Plugin for Skipper {
    const VST3_CLASS_ID: [u8; 16] = *b"SkipperInfoPlugn";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] = &[Vst3SubCategory::Tools];
}

nih_export_clap!(Skipper);
nih_export_vst3!(Skipper);
