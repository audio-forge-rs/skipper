# Build Commands

## Skipper (Rust CLAP/VST3 Plugin)

```bash
# Bundle as CLAP/VST3 (DEBUG - always use this)
cargo xtask bundle skipper

# Output: target/bundled/skipper.clap, target/bundled/skipper.vst3
```

**IMPORTANT:**
- Bitwig loads plugins directly from `target/bundled/`
- ALWAYS use debug builds (no --release flag)
- NEVER install to `~/Library/Audio/Plug-Ins/CLAP/`

```bash
# Run tests
cargo test test_plugin_receives_track_name -- --nocapture
```

## Gilligan (Java Bitwig Extension + MCP Server)

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

## MCP Server

**Claude Code Configuration** (`~/.claude/mcp.json`):
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

**Available Tools:**

| Tool | Description |
|------|-------------|
| `transport_play` | Start playback |
| `transport_stop` | Stop playback |
| `transport_record` | Toggle recording |
| `get_transport` | Get tempo, position, status |
| `list_tracks` | List all tracks |
| `get_selected_track` | Get selected track |
| `get_selected_device` | Get selected device |
| `create_track` | Create new track |
| `rename_track` | Rename a track |
