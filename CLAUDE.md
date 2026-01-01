# Skipper + Gilligan - DAW Info Display & AI Control System

A multi-component system for displaying DAW/track information and enabling AI-assisted music production:

1. **Skipper** - Rust CLAP/VST3 plugin (nih-plug) with egui GUI
2. **Gilligan** - Java Bitwig Controller Extension with MCP Server
3. **skipper-hub** (planned) - IPC daemon for cross-component communication
4. **skipper-cli** (planned) - CLI tool for Claude Code integration

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Claude Code (Opus 4.5)                      │
│                    "Start playback on Bitwig..."                    │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼ MCP over HTTP
┌─────────────────────────────────────────────────────────────────────┐
│                     Gilligan MCP Server                             │
│                 http://localhost:61170/mcp                          │
│                                                                     │
│   Token-Optimized Tools (7 tools, ~70% less overhead than WigAI):   │
│   • transport_play     - Start playback                             │
│   • transport_stop     - Stop playback                              │
│   • transport_record   - Toggle recording                           │
│   • get_transport      - Get tempo, position, status                │
│   • list_tracks        - List all tracks                            │
│   • get_selected_track - Get selected track info                    │
│   • get_selected_device - Get selected device info                  │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼
┌─────────────────────────────────────────────────────────────────────┐
│                     Bitwig Studio                                   │
│                                                                     │
│   ┌───────────────┐                    ┌───────────────┐            │
│   │   Skipper     │                    │   Gilligan    │            │
│   │  (CLAP/VST3)  │                    │ (Bitwig Ext)  │            │
│   │               │                    │               │            │
│   │ • Host info   │                    │ • Host info   │            │
│   │ • Track info* │                    │ • Track info  │            │
│   │ • Transport   │                    │ • Transport   │            │
│   │ • MIDI emit   │                    │ • Device chain│            │
│   └───────────────┘                    │ • All tracks  │            │
│          │                             │ • MCP Server  │            │
│          ▼                             └───────────────┘            │
│   ┌─────────────────┐                                               │
│   │  DAW Track      │                                               │
│   │  (instruments,  │                                               │
│   │   effects)      │                                               │
│   └─────────────────┘                                               │
└─────────────────────────────────────────────────────────────────────┘
```

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

<!-- Bitwig Controller API -->
<dependency>
    <groupId>com.bitwig</groupId>
    <artifactId>extension-api</artifactId>
    <version>19</version>
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

## Common Issues

### Skipper Plugin

**"Track info not available"**
- Ensure using CLAP format (not VST3)
- Host must support CLAP track-info extension (Bitwig 4.4+)

**"Host info empty"**
- Some hosts don't populate all fields
- Bitwig populates name/version; other hosts vary

### Gilligan Extension

**"Extension not loading"**
- Check Java version (21+ required)
- Verify .bwextension in correct folder
- Check Bitwig console for errors (View > Toggle Console)

**"MCP server not responding"**
- Check port 61170 is not in use
- Verify Gilligan is enabled in Bitwig Settings > Extensions
- Check firewall settings

**"Values not updating"**
- Call `markInterested()` on values you want to observe
- Bitwig only sends updates for interested values

## Resources

- [WigAI](https://github.com/fabb/WigAI) - Inspiration for MCP server in Bitwig
- [DrivenByMoss](https://github.com/git-moss/DrivenByMoss) - Premier Bitwig extension example
- [MCP Java SDK](https://github.com/modelcontextprotocol/java-sdk) - Official MCP SDK
- [MCP Specification](https://modelcontextprotocol.io/specification/2025-11-25)
- [Anthropic MCP Best Practices](https://www.anthropic.com/engineering/code-execution-with-mcp)

## License

- Skipper: AGPL-3.0
- Gilligan: AGPL-3.0
- nih-plug fork: ISC (upstream license)
