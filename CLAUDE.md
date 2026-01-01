# Skipper + Gilligan - DAW Info Display & AI Control System

A multi-component system for displaying DAW/track information and enabling AI-assisted music production:

1. **Skipper** - Rust CLAP/VST3 plugin (nih-plug) with egui GUI - one instance per track
2. **Gilligan** - Java Bitwig Controller Extension with MCP Server - acts as central hub

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Claude Code (Opus 4.5)                      │
│              "Load bass program on Track 2, drums on Track 3,       │
│               commit on next beat 1..."                             │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼ MCP over HTTP
┌─────────────────────────────────────────────────────────────────────┐
│                     Gilligan (Central Hub)                          │
│                 http://localhost:61170/mcp                          │
│                                                                     │
│   MCP Tools:                      Plugin Registry:                  │
│   • transport_*                   • Skipper instances register      │
│   • list_tracks                   • Track ID ↔ Plugin UUID map      │
│   • stage_program                 • Broadcast commit signals        │
│   • commit_programs               • Real track IDs from Bitwig API  │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
        ▼                      ▼                      ▼
┌───────────────┐      ┌───────────────┐      ┌───────────────┐
│  Skipper #1   │      │  Skipper #2   │      │  Skipper #3   │
│  (Track: Bass)│      │ (Track: Drums)│      │ (Track: Lead) │
│               │      │               │      │               │
│ • UUID: abc   │      │ • UUID: def   │      │ • UUID: ghi   │
│ • Staged MIDI │      │ • Staged MIDI │      │ • Staged MIDI │
│ • Beat-sync   │      │ • Beat-sync   │      │ • Beat-sync   │
└───────┬───────┘      └───────┬───────┘      └───────┬───────┘
        │                      │                      │
        ▼                      ▼                      ▼
   [ Bass Synth ]        [ Drum Kit ]          [ Lead Synth ]
```

## Why Both Plugin AND Controller Are Needed

**Gilligan (Controller) provides:**
- Full Bitwig API access (real track IDs, all tracks, devices, clips)
- MCP server for Claude Code
- Central coordination point
- Can't do sample-accurate beat sync

**Skipper (Plugin) provides:**
- Per-track presence in device chain
- Sample-accurate `process()` callback for beat detection
- Local buffer to stage MIDI programs
- Synchronized beat-1 release across all instances
- MIDI output to downstream instruments

**Key Use Case: Beat-Synced Multi-Track Commit**
1. Claude Code → Gilligan: "Stage bass on Track 2, drums on Track 3"
2. Gilligan → Skipper instances: "Here's your program, stage it"
3. Each Skipper buffers notes locally (not playing yet)
4. Claude Code → Gilligan: "Commit"
5. Gilligan → Skipper instances: "Commit on next beat 1"
6. Each Skipper watches transport in `process()` callback
7. On beat 1 → ALL Skippers emit MIDI simultaneously

**This requires plugins because:**
- `track.playNote()` in controller API plays immediately (no staging)
- No sample-accurate beat detection in controller API
- Synchronized release needs audio-thread timing

## Plugin Instance Identification

**Challenge:** Neither CLAP nor VST3 provide unique instance IDs or track IDs.

CLAP track-info extension only provides:
- Track name (not unique - multiple tracks can share names)
- Track color
- Track type flags (master/return/bus)
- NO track ID, NO UUID

**Solution:**
1. Each Skipper generates its own UUID on instantiation
2. Skipper registers with Gilligan: "I'm UUID X, on track named Y with color Z"
3. Gilligan uses Bitwig API to find matching track → correlates UUID to real track ID
4. Gilligan maintains UUID ↔ Track ID mapping

## MCP Server Token Optimization

Following [Anthropic's MCP best practices (2025)](https://www.anthropic.com/engineering/code-execution-with-mcp):

| Metric | WigAI | Gilligan | Improvement |
|--------|-------|----------|-------------|
| Tools | 15+ | 7 | ~50% fewer |
| Avg description | 20+ words | ~8 words | ~60% shorter |
| Schema complexity | Detailed | Minimal | ~70% less tokens |
| Estimated overhead | ~2000 tokens | ~600 tokens | **~70% savings** |

### Key Learnings Applied

1. **Minimal tool set**: Only essential operations (transport, tracks, devices)
2. **Concise descriptions**: ~10 words per tool instead of verbose explanations
3. **Filtered responses**: Return only essential data, not full object graphs
4. **Progressive disclosure ready**: Can add `search_tools` meta-tool if needed

Sources:
- [Code execution with MCP - Anthropic](https://www.anthropic.com/engineering/code-execution-with-mcp)
- [Reducing token usage 100x - Speakeasy](https://www.speakeasy.com/blog/how-we-reduced-token-usage-by-100x-dynamic-toolsets-v2)
- [Hidden Cost of MCP - Arsturn](https://www.arsturn.com/blog/hidden-cost-of-mcp-monitor-reduce-token-usage)

## Project Structure

```
skipper/
├── CLAUDE.md                    # This file
├── Cargo.toml                   # Rust workspace (Skipper plugin)
├── src/lib.rs                   # Skipper CLAP/VST3 plugin
│
├── nih-plug/                    # Forked nih-plug with track-info support
│   └── (submodule: audio-forge-rs/nih-plug)
│
├── gilligan/                    # Bitwig Controller Extension + MCP Server
│   ├── pom.xml                  # Maven build (Java 21, MCP SDK 0.11.0)
│   └── src/main/java/com/bedwards/gilligan/
│       ├── GilliganExtension.java          # Main extension
│       ├── GilliganExtensionDefinition.java
│       ├── BitwigApiFacade.java            # Bitwig API wrapper
│       └── mcp/
│           ├── McpServerManager.java       # Jetty + MCP server
│           └── tool/                       # MCP tool implementations
│               ├── TransportPlayTool.java
│               ├── TransportStopTool.java
│               ├── TransportRecordTool.java
│               ├── GetTransportStateTool.java
│               ├── ListTracksTool.java
│               ├── GetSelectedTrackTool.java
│               └── GetSelectedDeviceTool.java
│
├── xtask/                       # Rust build tooling
│   └── src/main.rs
│
├── .claude/                     # Claude Code integration
│   ├── skills/rust-plugin/
│   └── commands/
│
└── .cargo/config.toml           # Cargo aliases
```

## Feature Comparison

| Feature                    | Skipper (CLAP) | Gilligan (MCP) | Why |
|----------------------------|----------------|----------------|-----|
| Host name                  | ✅             | ✅ (always "Bitwig") | CLAP extension |
| Host version               | ✅             | ✅                   | CLAP extension |
| Plugin/extension info      | ✅             | ✅                   | Both have access |
| Track name                 | ✅*            | ✅                   | CLAP track-info ext |
| Track color                | ✅*            | ✅                   | CLAP track-info ext |
| Track type (master/bus)    | ✅*            | ✅                   | CLAP track-info ext |
| Transport state            | ✅ (read)      | ✅ (read/write)      | MCP adds control |
| Tempo                      | ✅             | ✅                   | Transport API |
| Time signature             | ✅             | ✅                   | Transport API |
| Position (beats/samples)   | ✅             | ✅                   | Transport API |
| **Track groups/nesting**   | ❌             | ✅                   | Controller API only |
| **All project tracks**     | ❌             | ✅                   | Controller API only |
| **Device chain**           | ❌             | ✅                   | Controller API only |
| **AI Control (MCP)**       | ❌             | ✅                   | MCP Server |
| **MIDI note emission**     | ✅             | ✅                   | Both can emit MIDI |

\* = Requires CLAP format AND host support for track-info extension

## Build Commands

### Skipper (Rust CLAP/VST3 Plugin)

```bash
# Bundle as CLAP/VST3 (DEBUG - always use this)
cargo xtask bundle skipper

# Output: target/bundled/skipper.clap, target/bundled/skipper.vst3

# IMPORTANT: Bitwig loads plugins directly from target/bundled/
# - ALWAYS use debug builds (no --release flag)
# - NEVER install to ~/Library/Audio/Plug-Ins/CLAP/
# - NEVER use --release flag during development

# Run tests
cargo test test_plugin_receives_track_name -- --nocapture
```

### Gilligan (Java Bitwig Extension + MCP Server)

```bash
cd gilligan

# Build (creates fat JAR with all dependencies)
mvn package

# Install to Bitwig Extensions folder
mvn install

# Output: ~/Documents/Bitwig Studio/Extensions/Gilligan.bwextension
```

**Requirements:**
- Java 21+ LTS
- Maven 3.8+
- Bitwig Studio 5.2.7+ (API version 19)

## MCP Server Usage

### Claude Code Configuration

Add to `~/.claude/mcp.json`:

```json
{
  "servers": {
    "gilligan": {
      "transport": "sse",
      "url": "http://localhost:61170/mcp"
    }
  }
}
```

### Testing MCP Server

```bash
# Verify server is running (while Bitwig is open with Gilligan enabled)
curl http://localhost:61170/mcp

# Example tool call via Claude Code
# User: "Start playback in Bitwig"
# Claude invokes: transport_play
```

### Available Tools

| Tool | Description | Returns |
|------|-------------|---------|
| `transport_play` | Start playback | "Playback started" |
| `transport_stop` | Stop playback | "Playback stopped" |
| `transport_record` | Toggle recording | "Recording toggled" |
| `get_transport` | Get transport state | JSON: tempo, position, status |
| `list_tracks` | List all tracks | JSON array: name, color, type |
| `get_selected_track` | Get selected track | JSON: name, color, isGroup |
| `get_selected_device` | Get selected device | JSON: name, exists |

## Rust Conventions

### nih-plug Patterns

- `Plugin` trait: Main plugin implementation
- `Params` trait: Parameter definitions with `#[derive(Params)]`
- `InitContext`: Access to `host_info()`, `track_info()` (our additions)
- `ProcessContext`: Access to `transport()` for real-time info
- `create_egui_editor`: GUI creation with egui

### Custom nih-plug Extensions

We forked nih-plug (audio-forge-rs/nih-plug) to add:

```rust
// In InitContext trait:
fn host_info(&self) -> Option<HostInfo>;    // CLAP host name/version
fn track_info(&self) -> Option<TrackInfo>;  // CLAP track-info extension
```

These are exposed via the prelude:
```rust
use nih_plug::prelude::{HostInfo, TrackInfo};
```

## Java Conventions

### MCP Tool Pattern

```java
public class MyTool {
    public static McpServerFeatures.SyncToolSpecification create(
            BitwigApiFacade facade, ControllerHost host) {

        McpSchema.Tool tool = McpSchema.Tool.builder()
            .name("tool_name")
            .description("Brief description (~10 words)")
            .inputSchema(McpSchema.EMPTY_OBJECT_SCHEMA)
            .build();

        return McpServerFeatures.SyncToolSpecification.builder()
            .tool(tool)
            .handler((exchange, request) -> {
                // Implementation
                return new McpSchema.CallToolResult(
                    List.of(new McpSchema.TextContent("Result")),
                    false  // isError
                );
            })
            .build();
    }
}
```

### Key Dependencies

```xml
<!-- MCP SDK (via BOM) -->
<dependency>
    <groupId>io.modelcontextprotocol.sdk</groupId>
    <artifactId>mcp</artifactId>
</dependency>

<!-- Jetty for HTTP server -->
<dependency>
    <groupId>org.eclipse.jetty</groupId>
    <artifactId>jetty-server</artifactId>
    <version>11.0.20</version>
</dependency>

<!-- Bitwig Controller API - PROVIDED by Bitwig runtime -->
<dependency>
    <groupId>com.bitwig</groupId>
    <artifactId>extension-api</artifactId>
    <version>19</version>
    <scope>provided</scope>
</dependency>
```

## IPC Architecture (Planned)

### Future: skipper-hub

For coordinating multiple Skipper plugin instances with Gilligan:

```
┌──────────────────────────────────────────────────────┐
│                    skipper-hub                       │
│              (Rust daemon, Unix socket)              │
│                                                      │
│   • Plugin/extension discovery & registration        │
│   • State synchronization                            │
│   • MIDI program staging & beat-synced commit        │
│   • Transport state broadcast                        │
└──────────────────────────────────────────────────────┘
```

### Beat-Synced MIDI Loading Flow

1. CLI sends MIDI program to hub
2. Hub broadcasts to all registered plugins/extensions
3. Each component loads notes into staging buffer
4. On "commit" message:
   - If transport playing: wait for next beat 1 (downbeat)
   - If transport stopped: immediate commit
5. All components switch to new program simultaneously

## Logging & Debugging

### nih_log! Macros (REQUIRED)

Use `nih_plug::nih_log!()` for ALL plugin logging:

```rust
nih_plug::nih_log!("Message here");
nih_plug::nih_log!("With args: {} {:?}", value, struct);
```

**NEVER use file I/O, String allocation, or format!() in `process()`** - the audio thread forbids memory allocation. The `assert_process_allocs` feature will crash the plugin.

### Starting Bitwig with Logging

To capture nih_log output, start Bitwig from terminal with `NIH_LOG` env var:

```bash
# Set log file and start Bitwig (macOS)
export NIH_LOG=~/skipper-nih.log
/Applications/Bitwig\ Studio.app/Contents/MacOS/BitwigStudio

# Or one-liner
NIH_LOG=~/skipper-nih.log /Applications/Bitwig\ Studio.app/Contents/MacOS/BitwigStudio
```

### Reading Logs

```bash
# View nih_log output
cat ~/skipper-nih.log

# Example output:
# 09:04:15 [INFO] skipper: Skipper v0.3.9 instance created (id=0)
# 09:04:15 [INFO] skipper: Skipper editor() called (id=0)
# 09:04:15 [INFO] nih_plug::wrapper::clap::wrapper: >>> CLAP track-info changed: name=Some("Skipper")
# 09:04:15 [INFO] nih_plug_egui::editor: EguiEditor::spawn() called
# 09:04:15 [DEBUG] egui_glow::painter: opengl version: 4.1 ATI-7.1.6
```

### NIH_LOG Environment Variable

- `NIH_LOG=stderr` → output to stderr (default)
- `NIH_LOG=/path/to/file.log` → output to file
- `NIH_LOG=windbg` → Windows debugger (Windows only)

### Audio Thread Rules

**CRITICAL:** The `process()` function runs on the audio thread. With `assert_process_allocs` enabled, ANY memory allocation OR deallocation crashes:

❌ **FORBIDDEN in process():**
- `String` creation or `format!()`
- `Vec` allocation or resizing
- `Box::new()` or any heap allocation
- File I/O (`std::fs::*`)
- `std::env::var_os()` - allocates internally
- `nih_log!()` or any logging - allocates strings
- Cloning types that contain `String` or `Vec`
- **Dropping the last `Arc<T>` if T contains heap data** - deallocation!
- Replacing `Option<Arc<T>>` values - drops old value, may deallocate

✅ **SAFE in process():**
- Pre-allocated buffers
- Stack variables (fixed-size arrays, primitives)
- Atomic operations (`AtomicU32`, `AtomicBool`, etc.)
- Cloning `Arc<T>` (just increments ref count, no allocation)
- Reading through `Arc<RwLock<T>>`
- Copying `Copy` types (integers, floats, tuples of Copy types)

⚠️ **SUBTLE TRAPS:**
```rust
// BAD: Cloning TrackInfo allocates (contains String fields)
let track_info = context.track_info(); // Returns Arc<TrackInfo>
state.track_info = track_info.clone(); // Safe - Arc clone is cheap

// BAD: But REPLACING an Option<Arc<T>> can DROP the old Arc!
if new_info != state.track_info {
    state.track_info = new_info; // If old Arc had refcount=1, this DEALLOCATES!
}

// GOOD: Update state only from main thread callbacks (initialize, changed)
// Don't poll/update track_info in process() at all
```

**Rule of thumb:** In `process()`, only READ cached data. UPDATE cached data from main-thread callbacks (`initialize()`, CLAP `changed()` callbacks, etc.).

### Shared State Between GUI and Audio Thread

When sharing state between GUI and audio thread, use `AtomicRefCell` (not `parking_lot::RwLock` which allocates):

```rust
use atomic_refcell::AtomicRefCell;

struct SharedState { /* ... */ }
state: Arc<AtomicRefCell<SharedState>>
```

**CRITICAL:** Both sides must use `try_borrow`/`try_borrow_mut` to avoid panics:

```rust
// ❌ BAD: borrow() panics if other thread holds lock
let shared = state.borrow();  // PANIC if audio thread has borrow_mut!

// ✅ GOOD: try_borrow gracefully handles contention
let Ok(shared) = state.try_borrow() else {
    return;  // Skip this frame, try again next time
};
```

**In process() (audio thread):**
```rust
// Skip update if GUI is reading - missing one frame is imperceptible
if let Ok(mut state) = self.state.try_borrow_mut() {
    state.transport.tempo = transport.tempo;
    // ...
}
```

**In GUI (main thread):**
```rust
// Skip frame if audio thread is writing
let Ok(shared) = state.try_borrow() else {
    ui.label("Loading...");
    return;
};
```

**Why not RwLock?** `parking_lot::RwLock` allocates thread-local data on first lock acquisition, crashing with `assert_process_allocs`.

## Debugging Guidelines

**NEVER delete code to debug.** Instead:
1. Add more `nih_log!()` calls to pinpoint exact crash location
2. Handle more exceptions/edge cases
3. Wrap suspicious code in error handling
4. Stay laser focused on the specific issue

**Logging approach:**
- Add granular logs before and after each suspicious operation
- Log function entry/exit points
- Log variable values at key points
- Use `nih_log!()` everywhere (NOT custom file logging)
- Start Bitwig from terminal to see logs

**Forking dependencies:**
- If you need to fork, clone, or submodule anything, use the `https://github.com/audio-forge-rs` org
- Use `gh` CLI for GitHub operations
- Commit and push changes frequently

## Forked Dependencies

We maintain forks of key dependencies under the [audio-forge-rs](https://github.com/audio-forge-rs) organization:

### nih-plug (audio-forge-rs/nih-plug)
- **Why:** Added CLAP track-info extension support, Arc-based track info caching
- **Branch:** `main`
- **Key additions:**
  - `InitContext::track_info()` - CLAP track-info extension
  - `ProcessContext::track_info()` - Arc<TrackInfo> for audio thread safety
  - Cached track info updated via CLAP `changed()` callback

### baseview (audio-forge-rs/baseview)
- **Why:** Fixed null pointer crash in macOS view initialization
- **Branch:** `fix-null-window-crash`
- **Fix:** Added null check in `become_first_responder` before calling `isKeyWindow`
- **Location:** `nih-plug/nih_plug_egui/Cargo.toml` references this fork

### egui-baseview (audio-forge-rs/egui-baseview)
- **Why:** Updated to use our forked baseview with null window fix
- **Branch:** `fix-null-window-crash`
- **Change:** Updated baseview dependency to audio-forge-rs/baseview

## Common Issues

### Skipper Plugin

**"Track info not available"**
- Ensure using CLAP format (not VST3)
- Host must support CLAP track-info extension (Bitwig 4.4+)

**"Host info empty"**
- Some hosts don't populate all fields
- Bitwig populates name/version; other hosts vary

### Gilligan Extension

**"Extension not loading" / "No extensions found"**
1. Check Bitwig log file for errors:
   ```bash
   cat ~/Library/Logs/Bitwig/BitwigStudio.log | grep -i "gilligan\|extension\|error"
   ```
2. Common causes:
   - Bitwig API bundled in JAR (must use `<scope>provided</scope>`)
   - Java version mismatch (21+ required)
   - Wrong extension folder

**"Extension not in Add Controller menu"**
- Go to: Settings → Controllers → Add → look for vendor name
- Check log: `~/Library/Logs/Bitwig/BitwigStudio.log`
- Look for: `[extension-registry error] Error scanning extension file`

**pom.xml Requirements:**
```xml
<!-- CRITICAL: Bitwig API must be 'provided' - NOT bundled in JAR -->
<dependency>
    <groupId>com.bitwig</groupId>
    <artifactId>extension-api</artifactId>
    <version>19</version>
    <scope>provided</scope>  <!-- REQUIRED! -->
</dependency>
```

**REQUIRED: Java SPI Service File**
Bitwig uses Java ServiceLoader to discover extensions. Create:
`src/main/resources/META-INF/services/com.bitwig.extension.ExtensionDefinition`

with content:
```
com.bedwards.gilligan.GilliganExtensionDefinition
```

**Verify JAR contents:**
```bash
jar tf target/gilligan-*.jar | grep "^com/" | head -20
# Should only show: com/bedwards/gilligan/...
# Should NOT show: com/bitwig/...
```

**"MCP server not responding"**
- Check port 61170 is not in use
- Verify Gilligan is enabled in Bitwig Settings > Controllers
- Check firewall settings

**"Values not updating"**
- Call `markInterested()` on values you want to observe
- Bitwig only sends updates for interested values

### Bitwig Log Files

```bash
# Main Bitwig log (extension loading, errors)
~/Library/Logs/Bitwig/BitwigStudio.log

# Previous session log
~/Library/Logs/Bitwig/BitwigStudio-previous-run.log

# Plugin-specific logs
~/Library/Logs/Bitwig/logs/skipper-plugin-0.log

# Engine log
~/Library/Logs/Bitwig/engine.log

# Crash reports
~/Library/Logs/DiagnosticReports/BitwigPluginHost-*.ips
```

### Bitwig UI Navigation

**Adding Controllers:**
Settings (gear icon) → Controllers → Add button → Select vendor → Select model

**Controller Script Console (where host.println() goes):**
Commander: Ctrl+Enter (Cmd+Enter on Mac) → type "console" → select "Show Control Script Console"

**Extension folder:**
`~/Documents/Bitwig Studio/Extensions/`

**Bitwig version info:**
`~/Library/Application Support/Bitwig/Bitwig Studio/last-run-info.txt`

## Additional Documentation

- [docs/BITWIG-DEVELOPMENT.md](docs/BITWIG-DEVELOPMENT.md) - Technical reference for AI agents (debugging, file locations, API details)
- [docs/UNDERSTANDING-BITWIG-DEVELOPMENT.md](docs/UNDERSTANDING-BITWIG-DEVELOPMENT.md) - Human-readable essay explaining plugins, extensions, and AI integration

## Resources

### Community
- [KVR Audio - Bitwig Forum](https://www.kvraudio.com/forum/viewforum.php?f=259) - Active community discussions
- [KVR Audio - Controller Scripting](https://www.kvraudio.com/forum/viewforum.php?f=268) - Extension development help
- [KVR Audio - DSP/Plugin Development](https://www.kvraudio.com/forum/viewforum.php?f=33) - CLAP/VST3/nih-plug discussions
- [DrivenByMoss Thread](https://www.kvraudio.com/forum/viewtopic.php?t=502948) - 450+ pages of Bitwig extension knowledge

### Projects
- [WigAI](https://github.com/fabb/WigAI) - Inspiration for MCP server in Bitwig
- [DrivenByMoss](https://github.com/git-moss/DrivenByMoss) - Premier Bitwig extension example
- [nih-plug](https://github.com/robbert-vdh/nih-plug) - Rust audio plugin framework
- [CLAP](https://github.com/free-audio/clap) - CLever Audio Plug-in specification

### Specifications
- [MCP Java SDK](https://github.com/modelcontextprotocol/java-sdk) - Official MCP SDK
- [MCP Specification](https://modelcontextprotocol.io/specification/2025-11-25)
- [Anthropic MCP Best Practices](https://www.anthropic.com/engineering/code-execution-with-mcp)

## Author

- **Name:** Brian Edwards
- **Email:** brian.mabry.edwards@gmail.com
- **Website:** https://audio-forge-rs.github.io/
- **GitHub Org:** https://github.com/audio-forge-rs
- **Vendor Name:** Audio Forge RS

## License

- Skipper: AGPL-3.0
- Gilligan: AGPL-3.0
- nih-plug fork: ISC (upstream license)
