use atomic_refcell::AtomicRefCell;
use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, egui, EguiState};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::SystemTime;

/// Staging directory where gilligan.py writes program files
const STAGING_DIR: &str = "/tmp/skipper";

/// Global counter for unique plugin instance IDs
static INSTANCE_COUNTER: AtomicU32 = AtomicU32::new(0);

/// Which tab is currently selected
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Tab {
    Live = 0,
    Program = 1,
    Info = 2,
}

/// Maximum notes per program (pre-allocated to avoid audio thread allocs)
const MAX_NOTES: usize = 256;

/// A single MIDI note in a program
#[derive(Clone, Copy, Default)]
struct ProgramNote {
    pitch: u8,           // MIDI pitch 0-127
    velocity: f32,       // 0.0-1.0
    start_beat: f64,     // Start position in beats from program start
    length_beats: f64,   // Duration in beats
    active: bool,        // Is this slot in use?
}

/// Fixed-size string for program names (no heap allocation)
const MAX_NAME_LEN: usize = 64;

/// A staged MIDI program (pre-allocated, fixed size)
#[derive(Clone)]
struct StagedProgram {
    name: [u8; MAX_NAME_LEN],   // Program name (UTF-8, null-terminated)
    name_len: usize,
    version: u32,               // Program version (increments on each load)
    notes: [ProgramNote; MAX_NOTES],
    note_count: usize,
    length_bars: f64,           // Program length in bars (power of 2)
    length_beats: f64,          // Cached: length_bars * beats_per_bar
    loaded: bool,               // Is a program loaded?
}

impl Default for StagedProgram {
    fn default() -> Self {
        Self {
            name: [0u8; MAX_NAME_LEN],
            name_len: 0,
            version: 0,
            notes: [ProgramNote::default(); MAX_NOTES],
            note_count: 0,
            length_bars: 4.0,
            length_beats: 16.0, // 4 bars * 4 beats
            loaded: false,
        }
    }
}

impl StagedProgram {
    /// Set program name (copies into fixed buffer)
    fn set_name(&mut self, name: &str) {
        let bytes = name.as_bytes();
        let len = bytes.len().min(MAX_NAME_LEN - 1);
        self.name[..len].copy_from_slice(&bytes[..len]);
        self.name[len] = 0; // null terminate
        self.name_len = len;
    }

    /// Get program name as string slice
    fn get_name(&self) -> &str {
        std::str::from_utf8(&self.name[..self.name_len]).unwrap_or("(invalid)")
    }
}

/// Tracks which notes are currently playing (for note-off)
#[derive(Clone)]
struct ActiveNotes {
    /// Bit flags for active notes (128 bits = 128 MIDI notes)
    playing: [u64; 2],
    /// End beat for each playing note
    end_beats: [f64; 128],
}

impl Default for ActiveNotes {
    fn default() -> Self {
        Self {
            playing: [0; 2],
            end_beats: [0.0; 128],
        }
    }
}

impl ActiveNotes {
    fn is_playing(&self, pitch: u8) -> bool {
        let idx = pitch as usize;
        let word = idx / 64;
        let bit = idx % 64;
        (self.playing[word] & (1u64 << bit)) != 0
    }

    fn set_playing(&mut self, pitch: u8, end_beat: f64) {
        let idx = pitch as usize;
        let word = idx / 64;
        let bit = idx % 64;
        self.playing[word] |= 1u64 << bit;
        self.end_beats[idx] = end_beat;
    }

    fn clear_playing(&mut self, pitch: u8) {
        let idx = pitch as usize;
        let word = idx / 64;
        let bit = idx % 64;
        self.playing[word] &= !(1u64 << bit);
    }
}

impl StagedProgram {
    /// Load program from JSON (from Gilligan)
    fn load_from_json(&mut self, json: &serde_json::Value) -> bool {
        nih_log!("Loading program from JSON...");

        // Get program name and version
        let name = json.get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Staged Program");
        self.set_name(name);

        self.version = json.get("version")
            .and_then(|v| v.as_u64())
            .unwrap_or(1) as u32;

        // Get length
        self.length_bars = json.get("lengthBars")
            .and_then(|v| v.as_f64())
            .unwrap_or(4.0);
        self.length_beats = json.get("lengthBeats")
            .and_then(|v| v.as_f64())
            .unwrap_or(self.length_bars * 4.0);

        // Parse notes
        let notes = match json.get("notes") {
            Some(serde_json::Value::Array(arr)) => arr,
            _ => {
                nih_log!("No notes array in program JSON");
                self.loaded = false;
                return false;
            }
        };

        self.note_count = 0;
        for note_json in notes.iter().take(MAX_NOTES) {
            let pitch = note_json.get("pitch")
                .and_then(|v| v.as_u64())
                .unwrap_or(60) as u8;
            let start_beat = note_json.get("startBeat")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let length_beats = note_json.get("lengthBeats")
                .and_then(|v| v.as_f64())
                .unwrap_or(1.0);
            let velocity = note_json.get("velocity")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.8) as f32;

            self.notes[self.note_count] = ProgramNote {
                pitch,
                start_beat,
                length_beats,
                velocity,
                active: true,
            };
            self.note_count += 1;
        }

        // Clear remaining slots
        for i in self.note_count..MAX_NOTES {
            self.notes[i].active = false;
        }

        self.loaded = true;
        nih_log!("Loaded program '{}' v{}: {} bars, {} notes",
            self.get_name(), self.version, self.length_bars, self.note_count);
        true
    }

    /// Load a beautiful 4-bar test program (C major arpeggio pattern)
    fn load_test_program(&mut self) {
        // Set program name and version
        self.set_name("C Major Arpeggio");
        self.version = 1;

        nih_log!("========================================");
        nih_log!("Loading program: {} v{}", self.get_name(), self.version);
        nih_log!("========================================");

        // Beautiful 4-bar piece: arpeggiated C major with melody
        // Bar 1: C E G C' (ascending arpeggio)
        // Bar 2: D' C' B A (descending melody)
        // Bar 3: G (half) E (half)
        // Bar 4: C (whole note resolution)

        let notes_data: [(u8, f64, f64, f32); 11] = [
            // Bar 1: C E G C' (beat 0-4)
            (60, 0.0, 1.0, 0.8),   // C4
            (64, 1.0, 1.0, 0.8),   // E4
            (67, 2.0, 1.0, 0.8),   // G4
            (72, 3.0, 1.0, 0.8),   // C5
            // Bar 2: D' C' B A (beat 4-8)
            (74, 4.0, 1.0, 0.8),   // D5
            (72, 5.0, 1.0, 0.8),   // C5
            (71, 6.0, 1.0, 0.8),   // B4
            (69, 7.0, 1.0, 0.8),   // A4
            // Bar 3: G (half) E (half) (beat 8-12)
            (67, 8.0, 2.0, 0.8),   // G4 (half note)
            (64, 10.0, 2.0, 0.8),  // E4 (half note)
            // Bar 4: C (whole) (beat 12-16)
            (60, 12.0, 4.0, 0.8),  // C4 (whole note)
        ];

        self.note_count = notes_data.len();
        self.length_bars = 4.0;
        self.length_beats = 16.0;  // 4 bars * 4 beats

        nih_log!("Program structure: {} bars ({} beats)", self.length_bars, self.length_beats);
        nih_log!("Notes ({}):", self.note_count);

        for (i, (pitch, start, length, vel)) in notes_data.iter().enumerate() {
            self.notes[i] = ProgramNote {
                pitch: *pitch,
                start_beat: *start,
                length_beats: *length,
                velocity: *vel,
                active: true,
            };
            let bar = (*start as i32 / 4) + 1;
            let beat = (*start % 4.0) + 1.0;
            nih_log!("  [{:2}] {} @ bar {} beat {:.1} (len: {} beats)",
                i, Self::pitch_to_name(*pitch), bar, beat, length);
        }

        // Clear remaining slots
        for i in self.note_count..MAX_NOTES {
            self.notes[i].active = false;
        }

        self.loaded = true;
        nih_log!("========================================");
        nih_log!("Program ready: {} v{}", self.get_name(), self.version);
        nih_log!("========================================");
    }

    /// Load a bass program (simple walking bass line in C)
    fn load_bass_program(&mut self) {
        self.set_name("Walking Bass C");
        self.version = 1;

        nih_log!("========================================");
        nih_log!("Loading program: {} v{}", self.get_name(), self.version);
        nih_log!("========================================");

        // 4-bar walking bass line in C major
        // Uses C1-C2 range (MIDI 24-36) - deep bass register
        // Bar 1: C G E G (root-5-3-5 pattern)
        // Bar 2: F C A C (IV chord)
        // Bar 3: G D B D (V chord)
        // Bar 4: C G C E (back to I, leading up)

        let notes_data: [(u8, f64, f64, f32); 16] = [
            // Bar 1: C major (beat 0-4)
            (24, 0.0, 1.0, 0.9),   // C1
            (31, 1.0, 1.0, 0.7),   // G1
            (28, 2.0, 1.0, 0.7),   // E1
            (31, 3.0, 1.0, 0.7),   // G1
            // Bar 2: F major (beat 4-8)
            (29, 4.0, 1.0, 0.9),   // F1
            (36, 5.0, 1.0, 0.7),   // C2
            (33, 6.0, 1.0, 0.7),   // A1
            (36, 7.0, 1.0, 0.7),   // C2
            // Bar 3: G major (beat 8-12)
            (31, 8.0, 1.0, 0.9),   // G1
            (26, 9.0, 1.0, 0.7),   // D1
            (35, 10.0, 1.0, 0.7),  // B1
            (26, 11.0, 1.0, 0.7),  // D1
            // Bar 4: C resolution (beat 12-16)
            (24, 12.0, 1.0, 0.9),  // C1
            (31, 13.0, 1.0, 0.7),  // G1
            (24, 14.0, 1.0, 0.7),  // C1
            (28, 15.0, 1.0, 0.8),  // E1 (leading tone up)
        ];

        self.note_count = notes_data.len();
        self.length_bars = 4.0;
        self.length_beats = 16.0;

        nih_log!("Program structure: {} bars ({} beats)", self.length_bars, self.length_beats);
        nih_log!("Notes ({}):", self.note_count);

        for (i, (pitch, start, length, vel)) in notes_data.iter().enumerate() {
            self.notes[i] = ProgramNote {
                pitch: *pitch,
                start_beat: *start,
                length_beats: *length,
                velocity: *vel,
                active: true,
            };
            let bar = (*start as i32 / 4) + 1;
            let beat = (*start % 4.0) + 1.0;
            nih_log!("  [{:2}] {} @ bar {} beat {:.1} (len: {} beats)",
                i, Self::pitch_to_name(*pitch), bar, beat, length);
        }

        // Clear remaining slots
        for i in self.note_count..MAX_NOTES {
            self.notes[i].active = false;
        }

        self.loaded = true;
        nih_log!("========================================");
        nih_log!("Program ready: {} v{}", self.get_name(), self.version);
        nih_log!("========================================");
    }

    /// Load a guitar power chord program (eighth note rhythm)
    fn load_guitar_program(&mut self) {
        self.set_name("Power Chords 8th");
        self.version = 1;

        nih_log!("========================================");
        nih_log!("Loading program: {} v{}", self.get_name(), self.version);
        nih_log!("========================================");

        // 4-bar power chord progression (C5 - F5 - G5 - C5)
        // All eighth notes (8 per bar, 0.5 beats each)
        // Power chord = root + fifth (no third)

        // Build notes programmatically for eighth note rhythm
        let mut notes_data: Vec<(u8, u8, f64, f64, f32)> = Vec::new(); // (root, fifth, start, len, vel)

        // Chord progression per bar
        let chords: [(u8, u8); 4] = [
            (48, 55), // C3 + G3 (C5 power chord)
            (53, 60), // F3 + C4 (F5 power chord)
            (55, 62), // G3 + D4 (G5 power chord)
            (48, 55), // C3 + G3 (C5 power chord)
        ];

        // All 8 eighth notes per bar
        let pattern: [f64; 8] = [0.0, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5];

        for (bar, (root, fifth)) in chords.iter().enumerate() {
            let bar_start = (bar as f64) * 4.0;
            for &offset in &pattern {
                notes_data.push((*root, *fifth, bar_start + offset, 0.4, 0.85));
            }
        }

        self.note_count = 0;
        self.length_bars = 4.0;
        self.length_beats = 16.0;

        nih_log!("Program structure: {} bars ({} beats)", self.length_bars, self.length_beats);
        nih_log!("Notes:");

        for (root, fifth, start, length, vel) in notes_data.iter() {
            // Add root note
            if self.note_count < MAX_NOTES {
                self.notes[self.note_count] = ProgramNote {
                    pitch: *root,
                    start_beat: *start,
                    length_beats: *length,
                    velocity: *vel,
                    active: true,
                };
                let bar = (*start as i32 / 4) + 1;
                let beat = (*start % 4.0) + 1.0;
                nih_log!("  [{:2}] {} @ bar {} beat {:.1}",
                    self.note_count, Self::pitch_to_name(*root), bar, beat);
                self.note_count += 1;
            }
            // Add fifth
            if self.note_count < MAX_NOTES {
                self.notes[self.note_count] = ProgramNote {
                    pitch: *fifth,
                    start_beat: *start,
                    length_beats: *length,
                    velocity: *vel * 0.9,
                    active: true,
                };
                nih_log!("  [{:2}] {} @ same time (fifth)",
                    self.note_count, Self::pitch_to_name(*fifth));
                self.note_count += 1;
            }
        }

        // Clear remaining slots
        for i in self.note_count..MAX_NOTES {
            self.notes[i].active = false;
        }

        self.loaded = true;
        nih_log!("Total notes: {}", self.note_count);
        nih_log!("========================================");
        nih_log!("Program ready: {} v{}", self.get_name(), self.version);
        nih_log!("========================================");
    }

    /// Load a drums program (basic rock beat with C3 trigger)
    fn load_drums_program(&mut self) {
        self.set_name("Rock Beat C3");
        self.version = 1;

        nih_log!("========================================");
        nih_log!("Loading program: {} v{}", self.get_name(), self.version);
        nih_log!("========================================");

        // 4-bar basic rock drum pattern
        // All notes on C3 (MIDI 48) - drum machine maps this
        // Kick on 1, 3 | Snare on 2, 4 (backbeat)

        // C3 = MIDI 48
        let c3: u8 = 48;

        let mut idx = 0;

        // 4 bars, each bar: kick(1) snare(2) kick(3) snare(4)
        for bar in 0..4 {
            let bar_start = (bar as f64) * 4.0;

            // Beat 1: Kick
            self.notes[idx] = ProgramNote {
                pitch: c3,
                start_beat: bar_start + 0.0,
                length_beats: 0.25,
                velocity: 0.9,
                active: true,
            };
            idx += 1;

            // Beat 2: Snare (backbeat)
            self.notes[idx] = ProgramNote {
                pitch: c3,
                start_beat: bar_start + 1.0,
                length_beats: 0.25,
                velocity: 0.85,
                active: true,
            };
            idx += 1;

            // Beat 3: Kick
            self.notes[idx] = ProgramNote {
                pitch: c3,
                start_beat: bar_start + 2.0,
                length_beats: 0.25,
                velocity: 0.9,
                active: true,
            };
            idx += 1;

            // Beat 4: Snare (backbeat)
            self.notes[idx] = ProgramNote {
                pitch: c3,
                start_beat: bar_start + 3.0,
                length_beats: 0.25,
                velocity: 0.85,
                active: true,
            };
            idx += 1;
        }

        self.note_count = idx;
        self.length_bars = 4.0;
        self.length_beats = 16.0;

        // Clear remaining slots
        for i in self.note_count..MAX_NOTES {
            self.notes[i].active = false;
        }

        self.loaded = true;
        nih_log!("Total notes: {}", self.note_count);
        nih_log!("========================================");
        nih_log!("Program ready: {} v{}", self.get_name(), self.version);
        nih_log!("========================================");
    }

    /// Convert MIDI pitch to note name for display
    fn pitch_to_name(pitch: u8) -> &'static str {
        const NAMES: [&str; 128] = [
            "C-1", "C#-1", "D-1", "D#-1", "E-1", "F-1", "F#-1", "G-1", "G#-1", "A-1", "A#-1", "B-1",
            "C0", "C#0", "D0", "D#0", "E0", "F0", "F#0", "G0", "G#0", "A0", "A#0", "B0",
            "C1", "C#1", "D1", "D#1", "E1", "F1", "F#1", "G1", "G#1", "A1", "A#1", "B1",
            "C2", "C#2", "D2", "D#2", "E2", "F2", "F#2", "G2", "G#2", "A2", "A#2", "B2",
            "C3", "C#3", "D3", "D#3", "E3", "F3", "F#3", "G3", "G#3", "A3", "A#3", "B3",
            "C4", "C#4", "D4", "D#4", "E4", "F4", "F#4", "G4", "G#4", "A4", "A#4", "B4",
            "C5", "C#5", "D5", "D#5", "E5", "F5", "F#5", "G5", "G#5", "A5", "A#5", "B5",
            "C6", "C#6", "D6", "D#6", "E6", "F6", "F#6", "G6", "G#6", "A6", "A#6", "B6",
            "C7", "C#7", "D7", "D#7", "E7", "F7", "F#7", "G7", "G#7", "A7", "A#7", "B7",
            "C8", "C#8", "D8", "D#8", "E8", "F8", "F#8", "G8", "G#8", "A8", "A#8", "B8",
            "C9", "C#9", "D9", "D#9", "E9", "F9", "F#9", "G9",
        ];
        if (pitch as usize) < NAMES.len() {
            NAMES[pitch as usize]
        } else {
            "???"
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
    track_info: Option<Arc<TrackInfo>>,
    transport: TransportState,
    sample_rate: f32,
    buffer_size: u32,
    plugin_api: PluginApi,
    current_tab: Tab,
    // Program playback state
    program: StagedProgram,
    active_notes: ActiveNotes,
    last_program_beat: f64,  // Last beat position we processed (for note triggers)
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
            program: StagedProgram::default(),
            active_notes: ActiveNotes::default(),
            last_program_beat: -1.0,
        }
    }
}

/// The Skipper plugin - displays host and track information
pub struct Skipper {
    params: Arc<SkipperParams>,
    state: Arc<AtomicRefCell<SharedState>>,
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
            state: Arc::new(AtomicRefCell::new(SharedState::default())),
            instance_id,
        }
    }
}

/// Gilligan REST API URL
const GILLIGAN_URL: &str = "http://localhost:61170/api";

/// Register with Gilligan and get any staged program
fn register_with_gilligan(uuid: &str, track_name: &str) -> Option<serde_json::Value> {
    nih_log!("Registering with Gilligan: uuid={}, track={}", uuid, track_name);

    let url = format!("{}/register", GILLIGAN_URL);
    let body = serde_json::json!({
        "uuid": uuid,
        "track": track_name
    });

    match ureq::post(&url)
        .set("Content-Type", "application/json")
        .send_string(&body.to_string())
    {
        Ok(response) => {
            match response.into_string() {
                Ok(text) => {
                    nih_log!("Gilligan response: {}", text);
                    match serde_json::from_str::<serde_json::Value>(&text) {
                        Ok(json) => {
                            if let Some(program) = json.get("program") {
                                if !program.is_null() {
                                    return Some(program.clone());
                                }
                            }
                            None
                        }
                        Err(e) => {
                            nih_log!("Failed to parse Gilligan response: {}", e);
                            None
                        }
                    }
                }
                Err(e) => {
                    nih_log!("Failed to read Gilligan response: {}", e);
                    None
                }
            }
        }
        Err(e) => {
            nih_log!("Failed to register with Gilligan: {}", e);
            None
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
fn build_info_text(shared: &SharedState, track_info: &Option<Arc<TrackInfo>>) -> String {
    let mut lines = Vec::new();

    lines.push("==================================================".to_string());
    lines.push("SKIPPER - DAW Info Display".to_string());
    lines.push("==================================================".to_string());
    lines.push(String::new());

    // Track name (prominent)
    if let Some(ref track) = track_info {
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
fn render_live_tab(ui: &mut egui::Ui, shared: &SharedState, track_info: &Option<Arc<TrackInfo>>) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Track section with color
            ui.heading("Track");
            ui.add_space(4.0);

            if let Some(ref track) = track_info {
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

/// Render the Program tab showing staged/current program
fn render_program_tab(ui: &mut egui::Ui, shared: &SharedState, track_info: &Option<Arc<nih_plug::prelude::TrackInfo>>) {
    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let program = &shared.program;

            // Program header
            ui.heading("Staged Program");
            ui.add_space(8.0);

            if program.loaded {
                // Program name and version with colored badge
                ui.horizontal(|ui| {
                    let badge_color = egui::Color32::from_rgb(100, 200, 100);
                    ui.label(egui::RichText::new("●").color(badge_color).size(16.0));
                    ui.label(egui::RichText::new(program.get_name()).size(18.0).strong());
                    ui.label(egui::RichText::new(format!("v{}", program.version))
                        .size(14.0)
                        .color(egui::Color32::GRAY));
                });

                ui.add_space(4.0);

                // Program stats
                ui.horizontal(|ui| {
                    ui.label(format!("{} bars", program.length_bars));
                    ui.label("•");
                    ui.label(format!("{} beats", program.length_beats));
                    ui.label("•");
                    ui.label(format!("{} notes", program.note_count));
                });

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                // Note list
                ui.heading("Notes");
                ui.add_space(4.0);

                // Column headers
                ui.horizontal(|ui| {
                    ui.label(egui::RichText::new("Bar").size(12.0).color(egui::Color32::GRAY));
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new("Beat").size(12.0).color(egui::Color32::GRAY));
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new("Note").size(12.0).color(egui::Color32::GRAY));
                    ui.add_space(20.0);
                    ui.label(egui::RichText::new("Len").size(12.0).color(egui::Color32::GRAY));
                });

                ui.add_space(4.0);

                // Note rows
                for i in 0..program.note_count {
                    let note = &program.notes[i];
                    if !note.active {
                        continue;
                    }

                    let bar = (note.start_beat / 4.0).floor() as i32 + 1;
                    let beat = (note.start_beat % 4.0) + 1.0;
                    let note_name = StagedProgram::pitch_to_name(note.pitch);

                    // Highlight current beat position
                    let is_current = if let Some(pos_beats) = shared.transport.pos_beats {
                        let program_beat = pos_beats % program.length_beats;
                        program_beat >= note.start_beat &&
                            program_beat < note.start_beat + note.length_beats
                    } else {
                        false
                    };

                    let text_color = if is_current {
                        egui::Color32::from_rgb(100, 255, 100)
                    } else {
                        egui::Color32::WHITE
                    };

                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(format!("{:2}", bar))
                            .monospace()
                            .color(text_color));
                        ui.add_space(24.0);
                        ui.label(egui::RichText::new(format!("{:.1}", beat))
                            .monospace()
                            .color(text_color));
                        ui.add_space(20.0);
                        ui.label(egui::RichText::new(format!("{:4}", note_name))
                            .monospace()
                            .strong()
                            .color(text_color));
                        ui.add_space(16.0);
                        ui.label(egui::RichText::new(format!("{:.1}", note.length_beats))
                            .monospace()
                            .color(text_color));
                    });
                }

                ui.add_space(16.0);
                ui.separator();
                ui.add_space(8.0);

                // Current position in program
                ui.heading("Playback");
                if let Some(pos_beats) = shared.transport.pos_beats {
                    let program_beat = pos_beats % program.length_beats;
                    let program_bar = (program_beat / 4.0).floor() as i32 + 1;
                    let beat_in_bar = (program_beat % 4.0) + 1.0;

                    ui.label(egui::RichText::new(
                        format!("Program position: Bar {} Beat {:.2}", program_bar, beat_in_bar)
                    ).size(16.0).monospace());

                    ui.label(format!("Transport position: {:.2} beats", pos_beats));
                } else {
                    ui.label("(transport position unavailable)");
                }
            } else {
                // No program loaded
                ui.label(egui::RichText::new("No program loaded").size(16.0).color(egui::Color32::GRAY));
                ui.add_space(16.0);

                // Show help text with actual track name
                let track_name = track_info.as_ref()
                    .and_then(|i| i.name.as_ref())
                    .map(|s| s.as_str())
                    .unwrap_or("TrackName");
                ui.label("Use Gilligan CLI to load a program:");
                ui.label(egui::RichText::new(format!(
                    "gilligan.py workflow --track {} --abc 'c d e f'", track_name))
                    .monospace()
                    .size(12.0));
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
    const MIDI_OUTPUT: MidiConfig = MidiConfig::Basic;  // Enable MIDI output for note playback
    const SAMPLE_ACCURATE_AUTOMATION: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        nih_log!("Skipper editor() called (id={})", self.instance_id);
        let state = self.state.clone();
        let instance_id = self.instance_id;

        create_egui_editor(
            self.params.editor_state.clone(),
            (),
            |_, _| {},
            move |egui_ctx, setter, _editor_state| {
                egui::CentralPanel::default().show(egui_ctx, |ui| {
                    egui_ctx.request_repaint();

                    // Get latest track info from context (updated by CLAP changed callback)
                    let track_info = setter.raw_context.track_info();

                    // Register with Gilligan when track info available and no program loaded
                    // Keep trying until we get a program (allows staging after plugin load)
                    let has_program = if let Ok(s) = state.try_borrow() {
                        s.program.note_count > 0
                    } else {
                        true // Assume loaded if can't check
                    };

                    if !has_program {
                        if let Some(ref info) = track_info {
                            if let Some(ref track_name) = info.name {
                                if !track_name.is_empty() {
                                    let uuid = format!("skipper-{}", instance_id);
                                    if let Some(program_json) = register_with_gilligan(&uuid, track_name) {
                                        // Load the program from Gilligan
                                        if let Ok(mut s) = state.try_borrow_mut() {
                                            s.program.load_from_json(&program_json);
                                        }
                                    }
                                }
                            }
                        }
                    }

                    // Try to borrow state - skip frame if audio thread holds lock
                    let Ok(shared) = state.try_borrow() else {
                        ui.label("Loading...");
                        return;
                    };

                    let current_tab = shared.current_tab;

                    // Release borrow before tab clicks can mutate
                    drop(shared);

                    ui.horizontal(|ui| {
                        if ui.selectable_label(current_tab == Tab::Live, "Live").clicked() {
                            if let Ok(mut s) = state.try_borrow_mut() {
                                s.current_tab = Tab::Live;
                            }
                        }
                        if ui.selectable_label(current_tab == Tab::Program, "Program").clicked() {
                            if let Ok(mut s) = state.try_borrow_mut() {
                                s.current_tab = Tab::Program;
                            }
                        }
                        if ui.selectable_label(current_tab == Tab::Info, "Info").clicked() {
                            if let Ok(mut s) = state.try_borrow_mut() {
                                s.current_tab = Tab::Info;
                            }
                        }
                    });

                    ui.separator();

                    // Re-borrow for rendering
                    let Ok(shared) = state.try_borrow() else {
                        return;
                    };

                    match current_tab {
                        Tab::Live => {
                            render_live_tab(ui, &shared, &track_info);
                        }
                        Tab::Program => {
                            render_program_tab(ui, &shared, &track_info);
                        }
                        Tab::Info => {
                            let info_text = build_info_text(&shared, &track_info);
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
            let mut state = self.state.borrow_mut();
            state.sample_rate = buffer_config.sample_rate;
            state.buffer_size = buffer_config.max_buffer_size;
            state.plugin_api = api;
            state.host_info = host_info;
            state.track_info = track_info;
        }

        // Spawn background thread to register with Gilligan once track info is available
        let state_clone = self.state.clone();
        let instance_id = self.instance_id;
        std::thread::spawn(move || {
            // Wait for track info to be populated (up to 5 seconds)
            for _ in 0..50 {
                std::thread::sleep(std::time::Duration::from_millis(100));

                let track_name = if let Ok(s) = state_clone.try_borrow() {
                    s.track_info.as_ref()
                        .and_then(|t| t.name.as_ref())
                        .filter(|n| !n.is_empty())
                        .cloned()
                } else {
                    None
                };

                if let Some(name) = track_name {
                    // Check if already has program
                    let has_program = if let Ok(s) = state_clone.try_borrow() {
                        s.program.note_count > 0
                    } else {
                        false
                    };

                    if !has_program {
                        let uuid = format!("skipper-{}", instance_id);
                        if let Some(program_json) = register_with_gilligan(&uuid, &name) {
                            if let Ok(mut s) = state_clone.try_borrow_mut() {
                                s.program.load_from_json(&program_json);
                            }
                        }
                    }
                    break;
                }
            }
        });

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

        // Use try_borrow_mut to avoid panic if GUI is reading state
        // If contention, skip this update - GUI will get next one
        if let Ok(mut state) = self.state.try_borrow_mut() {
            // Update transport state for GUI
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

            // === MIDI Note Emission ===
            // Only emit notes if playing and we have a loaded program
            if transport.playing && state.program.loaded {
                if let Some(pos_beats) = transport.pos_beats() {
                    let program_length = state.program.length_beats;
                    if program_length > 0.0 {
                        // Calculate position within program (looping)
                        let program_beat = pos_beats % program_length;
                        let last_beat = state.last_program_beat;

                        // Detect wrap: position jumped backwards significantly
                        // Use a threshold to handle floating point precision
                        let wrapped = last_beat >= 0.0 && program_beat < last_beat - 1.0;

                        // Also detect first frame after transport start (last_beat was -1)
                        let first_frame = last_beat < 0.0;

                        // On wrap or first frame: clear all active notes
                        if wrapped || first_frame {
                            for pitch in 0u8..128 {
                                if state.active_notes.is_playing(pitch) {
                                    context.send_event(NoteEvent::NoteOff {
                                        timing: 0,
                                        voice_id: None,
                                        channel: 0,
                                        note: pitch,
                                        velocity: 0.0,
                                    });
                                    state.active_notes.clear_playing(pitch);
                                }
                            }
                        }

                        // Check each note for note-on and note-off events
                        for i in 0..state.program.note_count {
                            let note = &state.program.notes[i];
                            if !note.active {
                                continue;
                            }

                            let note_start = note.start_beat;
                            let note_end = note.start_beat + note.length_beats;
                            let pitch = note.pitch;

                            // Note-on: trigger if we just crossed the start beat
                            let should_trigger = if wrapped || first_frame {
                                // Wrap or start: trigger all notes from 0 to current position
                                note_start <= program_beat + 0.01
                            } else {
                                // Normal case: did we cross the start beat?
                                note_start > last_beat && note_start <= program_beat + 0.01
                            };

                            if should_trigger && !state.active_notes.is_playing(pitch) {
                                // Send note-on
                                let velocity = (note.velocity * 127.0) as u8;
                                context.send_event(NoteEvent::NoteOn {
                                    timing: 0,
                                    voice_id: None,
                                    channel: 0,
                                    note: pitch,
                                    velocity: note.velocity,
                                });
                                state.active_notes.set_playing(pitch, note_end);
                            }

                            // Note-off: trigger if we crossed the end beat
                            if state.active_notes.is_playing(pitch) {
                                let note_end_beat = state.active_notes.end_beats[pitch as usize];
                                let should_end = if wrapped {
                                    note_end_beat > last_beat || note_end_beat <= program_beat
                                } else {
                                    note_end_beat > last_beat && note_end_beat <= program_beat
                                };

                                if should_end {
                                    context.send_event(NoteEvent::NoteOff {
                                        timing: 0,
                                        voice_id: None,
                                        channel: 0,
                                        note: pitch,
                                        velocity: 0.0,
                                    });
                                    state.active_notes.clear_playing(pitch);
                                }
                            }
                        }

                        state.last_program_beat = program_beat;
                    }
                }
            } else if !transport.playing {
                // Transport stopped - send note-off for all active notes
                for pitch in 0u8..128 {
                    if state.active_notes.is_playing(pitch) {
                        context.send_event(NoteEvent::NoteOff {
                            timing: 0,
                            voice_id: None,
                            channel: 0,
                            note: pitch,
                            velocity: 0.0,
                        });
                        state.active_notes.clear_playing(pitch);
                    }
                }
                state.last_program_beat = -1.0;
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
