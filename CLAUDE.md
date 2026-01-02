# Skipper + Gilligan - DAW Info Display & AI Control System

A multi-component system for displaying DAW/track information and enabling AI-assisted music production:

1. **Skipper** - Rust CLAP/VST3 plugin (nih-plug) with egui GUI - one instance per track
2. **Gilligan** - Java Bitwig Controller Extension with REST API - acts as central hub
3. **gilligan.py** - Python CLI for transport, tracks, and staging programs

## IMPORTANT: Use CLI Tools, Not MCP

**DO NOT install the MCP server in Claude Code.** Use the Python CLI tools instead:

```bash
# Transport control
python tools/gilligan.py play
python tools/gilligan.py stop

# List tracks
python tools/gilligan.py tracks

# Validate and stage ABC notation
python tools/gilligan.py workflow --track Piano --abc 'c d e f | g a b c |'

# Validate from file
python tools/gilligan.py workflow --track Bass --file programs/template/current/bass.abc
```

The MCP code exists in the codebase for compatibility but is not the preferred interface.

## Quick Reference

See @docs/BITWIG-DEVELOPMENT.md for technical debugging reference.
See @docs/UNDERSTANDING-BITWIG-DEVELOPMENT.md for human-readable overview.

## Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│                         Claude Code (Opus 4.5)                      │
│              "Load bass program on Track 2, drums on Track 3..."    │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
                               ▼ Python CLI (gilligan.py)
┌─────────────────────────────────────────────────────────────────────┐
│                     Gilligan (Central Hub)                          │
│                 REST API: http://localhost:61170/api/               │
│                                                                     │
│   Commands:                       Plugin Registry:                  │
│   • play, stop, transport         • Skipper instances register      │
│   • tracks, snapshot              • Track ID ↔ Plugin UUID map      │
│   • stage (ABC → MIDI)            • Broadcast commit signals        │
└──────────────────────────────┬──────────────────────────────────────┘
                               │
        ┌──────────────────────┼──────────────────────┐
        │                      │                      │
        ▼                      ▼                      ▼
┌───────────────┐      ┌───────────────┐      ┌───────────────┐
│  Skipper #1   │      │  Skipper #2   │      │  Skipper #3   │
│  (Track: Bass)│      │ (Track: Drums)│      │ (Track: Lead) │
│               │      │               │      │               │
│ • Staged MIDI │      │ • Staged MIDI │      │ • Staged MIDI │
│ • Beat-sync   │      │ • Beat-sync   │      │ • Beat-sync   │
└───────┬───────┘      └───────┬───────┘      └───────┬───────┘
        │                      │                      │
        ▼                      ▼                      ▼
   [ Bass Synth ]        [ Drum Kit ]          [ Lead Synth ]
```

## Project Structure

```
skipper/
├── CLAUDE.md                    # This file (imports from .claude/rules/)
├── Cargo.toml                   # Rust workspace (Skipper plugin)
├── src/lib.rs                   # Skipper CLAP/VST3 plugin
│
├── nih-plug/                    # Forked nih-plug with track-info support
│   └── (submodule: audio-forge-rs/nih-plug)
│
├── gilligan/                    # Bitwig Controller Extension + MCP Server
│   ├── pom.xml                  # Maven build (Java 21, MCP SDK 0.11.0)
│   └── src/main/java/com/bedwards/gilligan/
│       ├── GilliganExtension.java
│       ├── BitwigApiFacade.java
│       └── mcp/
│           ├── McpServerManager.java
│           └── tool/*.java
│
├── docs/                        # Documentation
│   ├── BITWIG-DEVELOPMENT.md    # Technical reference
│   └── UNDERSTANDING-BITWIG-DEVELOPMENT.md
│
└── .claude/                     # Claude Code integration
    ├── rules/                   # Modular instruction files
    │   ├── architecture.md
    │   ├── audio-thread.md
    │   ├── java-conventions.md
    │   ├── debugging.md
    │   ├── common-issues.md
    │   └── build-commands.md
    └── skills/
```

## Feature Comparison

| Feature                    | Skipper (CLAP) | Gilligan (MCP) | Why |
|----------------------------|----------------|----------------|-----|
| Host name                  | ✅             | ✅ (always "Bitwig") | CLAP extension |
| Track name/color           | ✅*            | ✅                   | CLAP track-info |
| Transport state            | ✅ (read)      | ✅ (read/write)      | MCP adds control |
| Tempo/time sig/position    | ✅             | ✅                   | Transport API |
| **All project tracks**     | ❌             | ✅                   | Controller API only |
| **Device chain**           | ❌             | ✅                   | Controller API only |
| **AI Control (MCP)**       | ❌             | ✅                   | MCP Server |
| **Beat-accurate timing**   | ✅             | ❌                   | Audio thread only |

\* = Requires CLAP format AND host support for track-info extension

## Bitwig UI Navigation

**Adding Controllers:**
Settings (gear icon) → Controllers → Add → Audio Forge RS → Gilligan

**Controller Script Console (where host.println() goes):**
Commander: Cmd+Enter → type "console" → select "Show Control Script Console"

**Extension folder:**
`~/Documents/Bitwig Studio/Extensions/`

## Resources

### Community
- [KVR Audio - Bitwig Forum](https://www.kvraudio.com/forum/viewforum.php?f=259)
- [KVR Audio - Controller Scripting](https://www.kvraudio.com/forum/viewforum.php?f=268)
- [KVR Audio - DSP/Plugin Development](https://www.kvraudio.com/forum/viewforum.php?f=33)
- [DrivenByMoss Thread](https://www.kvraudio.com/forum/viewtopic.php?t=502948)

### Projects
- [WigAI](https://github.com/fabb/WigAI) - MCP server for Bitwig inspiration
- [DrivenByMoss](https://github.com/git-moss/DrivenByMoss) - Premier Bitwig extension
- [nih-plug](https://github.com/robbert-vdh/nih-plug) - Rust audio plugin framework
- [CLAP](https://github.com/free-audio/clap) - CLever Audio Plug-in specification

### Specifications
- [MCP Java SDK](https://github.com/modelcontextprotocol/java-sdk)
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
