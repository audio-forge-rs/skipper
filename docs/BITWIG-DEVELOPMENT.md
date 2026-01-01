# Bitwig Development Reference

Technical reference for Claude Code and AI agents working on Skipper/Gilligan.

## Quick Reference

### Controller Script Console

Access via Commander (works on all platforms):
```
Ctrl+Enter (or Cmd+Enter on Mac) → type "console" → select "Show Control Script Console"
```

Alternative methods:
- View menu → Show Control Script Console
- Keyboard shortcut (user-configurable, check Settings → Shortcuts)
- Studio I/O pane → Show Control Script Console

Output from `host.println()` appears here. Type "restart" to reload scripts.

### Java Extension Debugging

Set environment variable before launching Bitwig:
```bash
# macOS (add to ~/.zprofile)
export BITWIG_DEBUG_PORT=5005

# Launch Bitwig from terminal to pick up the variable
/Applications/Bitwig\ Studio.app/Contents/MacOS/BitwigStudio
```

Connect IDE debugger to `localhost:5005`. Bitwig freezes at breakpoints (normal behavior).

### CLAP Plugin Debugging

Bitwig runs plugins in sandbox process: `BitwigPluginHost-X64-SSE41.exe` (Windows) or similar.

Workaround for attach-before-load problem:
1. Disable sandboxing for your plugin
2. Load any plugin to start the host process
3. Attach debugger to plugin host
4. Load your plugin (second load into same host)

Tools:
- [clap-info](https://github.com/surge-synthesizer/clap-info) - CLI scanner for CLAP plugins
- [clap-host](https://github.com/free-audio/clap-host) - Demo host with GUI

### File Locations

| Item | macOS Path |
|------|------------|
| Controller Extensions | `~/Documents/Bitwig Studio/Extensions/` |
| Controller Scripts (JS) | `~/Documents/Bitwig Studio/Controller Scripts/` |
| CLAP Plugins | `~/Library/Audio/Plug-Ins/CLAP/` |
| VST3 Plugins | `~/Library/Audio/Plug-Ins/VST3/` |
| Bitwig Logs | `~/Library/Logs/Bitwig/BitwigStudio.log` |
| Crash Reports | `~/Library/Logs/DiagnosticReports/BitwigPluginHost-*.ips` |

### Extension vs Plugin

| Aspect | Controller Extension (.bwextension) | Audio Plugin (.clap/.vst3) |
|--------|-------------------------------------|---------------------------|
| Language | Java | Any (C/C++/Rust via nih-plug) |
| Purpose | Control DAW, respond to MIDI controllers | Process/generate audio |
| API | Bitwig Controller API | CLAP/VST3 spec |
| Thread | Main thread, can freeze UI | Audio thread, must not block |
| Logging | `host.println()` → Script Console | `nih_log!()` → file/stderr |
| Debugging | BITWIG_DEBUG_PORT + IDE | Attach to plugin host process |

### CLAP Extensions Supported by Bitwig

| Extension | Status | Notes |
|-----------|--------|-------|
| `clap.track-info` | Supported | Track name, color, type flags |
| `clap.timer-support` | Linux only | Returns NULL on Windows/macOS |
| `clap.note-ports` | Supported | MIDI I/O |
| `clap.audio-ports` | Supported | Audio I/O |
| `clap.params` | Supported | Parameter automation |
| `clap.gui` | Supported | Plugin UI |

### nih-plug Specific

Our fork: `audio-forge-rs/nih-plug`

Key additions:
- `InitContext::track_info()` - CLAP track-info at init
- `ProcessContext::track_info()` - Arc<TrackInfo> for audio thread

Audio thread rules (with `assert_process_allocs`):
- No `String`, `Vec`, `Box::new()`, `format!()`
- No file I/O
- No `nih_log!()`
- No dropping last `Arc<T>` if T contains heap data
- Use `AtomicRefCell` with `try_borrow()`, not `parking_lot::RwLock`

### MCP Server Transport

Two transport providers in MCP Java SDK:
- `HttpServletSseServerTransportProvider` - SSE-based (what we use)
- `HttpServletStreamableServerTransportProvider` - Streamable HTTP (what WigAI uses)

SSE transport requires:
1. Client connects to `/sse` endpoint first
2. Server sends session ID via SSE stream
3. Client POSTs to `/mcp` with session ID in subsequent requests

Claude Code MCP config (`~/.claude/mcp.json`):
```json
{
  "mcpServers": {
    "gilligan": {
      "transport": "sse",
      "url": "http://localhost:61170/sse"
    }
  }
}
```

## Key Resources

### Official

- [CLAP Specification](https://github.com/free-audio/clap)
- [Bitwig Controller API](https://www.bitwig.com/userguide/) (Help → Documentation in Bitwig)
- [MCP Specification](https://modelcontextprotocol.io/specification/2025-11-25)
- [MCP Java SDK](https://github.com/modelcontextprotocol/java-sdk)
- [nih-plug](https://github.com/robbert-vdh/nih-plug)

### Community

- [KVR Audio - Bitwig Forum](https://www.kvraudio.com/forum/viewforum.php?f=259)
- [KVR Audio - Controller Scripting Forum](https://www.kvraudio.com/forum/viewforum.php?f=268)
- [KVR Audio - DSP and Plugin Development](https://www.kvraudio.com/forum/viewforum.php?f=33)
- [DrivenByMoss Thread](https://www.kvraudio.com/forum/viewtopic.php?t=502948)
- [CLAP Standard Thread](https://www.kvraudio.com/forum/viewtopic.php?t=583140)

### Examples

- [DrivenByMoss](https://github.com/git-moss/DrivenByMoss) - Premier Bitwig extension
- [WigAI](https://github.com/fabb/WigAI) - MCP server for Bitwig
- [clap-saw-demo](https://github.com/surge-synthesizer/clap-saw-demo) - CLAP plugin example
- [bitwig-controller-tutorial](https://github.com/outterback/bitwig-controller-tutorial) - Java extension setup

## Common Issues

### Extension not appearing in Add Controller

1. Check `META-INF/services/com.bitwig.extension.ExtensionDefinition` exists in JAR
2. Bitwig API dependency must be `<scope>provided</scope>`
3. Check `~/Library/Logs/Bitwig/BitwigStudio.log` for `[extension-registry error]`

### MCP connection failing

1. Verify server running: `lsof -i :61170`
2. Check transport type matches client expectation
3. SSE requires session handshake before POST

### Plugin crash on load

1. Check `~/Library/Logs/DiagnosticReports/` for crash logs
2. Verify no allocations in `process()` callback
3. Test with clap-info first

### Track info not available

1. Must be CLAP format (not VST3)
2. Host must support `clap.track-info` extension
3. Available in `InitContext`, cached for audio thread
