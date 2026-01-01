# Skipper + Gilligan

AI-assisted music production for Bitwig Studio.

This project enables Claude Code and other AI assistants to control Bitwig Studio through a combination of a Rust audio plugin and a Java controller extension. The plugin provides sample-accurate timing on each track, while the controller extension serves as a central hub with MCP (Model Context Protocol) server for AI communication.

**New to this project?** Start with [Understanding Bitwig Development](docs/UNDERSTANDING-BITWIG-DEVELOPMENT.md) - a human-readable essay explaining how plugins, extensions, and AI integration work together.

## Components

### Skipper (Rust CLAP/VST3 Plugin)

A per-track plugin built with [nih-plug](https://github.com/robbert-vdh/nih-plug) that displays track information and provides sample-accurate beat synchronization.

- Displays host name, track name, and color via CLAP track-info extension
- Runs in the audio thread for precise timing
- Stages MIDI content for synchronized release across tracks

### Gilligan (Java Bitwig Controller Extension)

A controller extension that acts as the central coordinator and hosts an MCP server.

- Full Bitwig Controller API access
- MCP server at `http://localhost:61170/sse`
- Tools for transport control, track management, and device inspection
- Coordinates multiple Skipper instances for beat-synced operations

## Architecture

```
Claude Code (Opus 4.5)
        |
        | MCP over HTTP
        v
Gilligan (Controller Extension + MCP Server)
        |
   +---------+---------+
   |         |         |
   v         v         v
Skipper   Skipper   Skipper
(Track 1) (Track 2) (Track 3)
   |         |         |
   v         v         v
 Synth     Drums     Lead
```

See [docs/architecture.excalidraw](docs/architecture.excalidraw) for a visual diagram (open with [Excalidraw](https://excalidraw.com)).

## Quick Start

### Build Skipper

```bash
cargo xtask bundle skipper
```

Bitwig loads plugins directly from `target/bundled/`. No installation step needed.

### Build Gilligan

```bash
cd gilligan
mvn install
```

This builds and copies `Gilligan.bwextension` to `~/Documents/Bitwig Studio/Extensions/`.

### Enable in Bitwig

1. Open Bitwig Studio
2. Go to Settings > Controllers > Add
3. Select Audio Forge RS > Gilligan

### Configure Claude Code

Add to `~/.claude/mcp.json`:

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

Restart Claude Code to pick up the configuration.

## MCP Tools

| Tool | Description |
|------|-------------|
| `transport_play` | Start playback |
| `transport_stop` | Stop playback |
| `transport_record` | Toggle recording |
| `get_transport` | Get tempo, position, playing status |
| `list_tracks` | List all tracks with names and colors |
| `get_selected_track` | Get currently selected track |
| `create_track` | Create a new track |
| `rename_track` | Rename a track |
| `get_selected_device` | Get selected device info |

## Documentation

- [Understanding Bitwig Development](docs/UNDERSTANDING-BITWIG-DEVELOPMENT.md) - Human-readable essay on plugins, extensions, and AI integration
- [Bitwig Development Reference](docs/BITWIG-DEVELOPMENT.md) - Technical reference for developers

## Requirements

- Bitwig Studio 5.2.7+ (API version 19)
- Java 21+ LTS
- Rust (latest stable)
- Maven 3.8+

## Forked Dependencies

This project uses forked versions of several libraries:

- **[audio-forge-rs/nih-plug](https://github.com/audio-forge-rs/nih-plug)** - Added CLAP track-info extension support
- **[audio-forge-rs/baseview](https://github.com/audio-forge-rs/baseview)** - Fixed null pointer crash in macOS view initialization
- **[audio-forge-rs/egui-baseview](https://github.com/audio-forge-rs/egui-baseview)** - Updated to use forked baseview

## Related Projects

- [DrivenByMoss](https://github.com/git-moss/DrivenByMoss) - The premier Bitwig controller extension
- [WigAI](https://github.com/fabb/WigAI) - MCP server for Bitwig that inspired Gilligan
- [nih-plug](https://github.com/robbert-vdh/nih-plug) - Rust audio plugin framework
- [CLAP](https://github.com/free-audio/clap) - CLever Audio Plug-in specification

## License

- Skipper: AGPL-3.0
- Gilligan: AGPL-3.0
- nih-plug fork: ISC (upstream license)

## Author

Brian Edwards
[Audio Forge RS](https://audio-forge-rs.github.io/)
brian.mabry.edwards@gmail.com
