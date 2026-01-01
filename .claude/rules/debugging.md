# Debugging Guidelines

## Core Principles

**NEVER delete code to debug.** Instead:
1. Add more `nih_log!()` calls to pinpoint exact crash location
2. Handle more exceptions/edge cases
3. Wrap suspicious code in error handling
4. Stay laser focused on the specific issue

## Rust Plugin Logging (nih_log!)

Use `nih_plug::nih_log!()` for ALL plugin logging:

```rust
nih_plug::nih_log!("Message here");
nih_plug::nih_log!("With args: {} {:?}", value, struct);
```

**NEVER use file I/O, String allocation, or format!() in `process()`**

### NIH_LOG Environment Variable

- `NIH_LOG=stderr` → output to stderr (default)
- `NIH_LOG=/path/to/file.log` → output to file
- `NIH_LOG=windbg` → Windows debugger (Windows only)

### Starting Bitwig with Logging

```bash
# macOS
export NIH_LOG=~/skipper-nih.log
/Applications/Bitwig\ Studio.app/Contents/MacOS/BitwigStudio

# Or one-liner
NIH_LOG=~/skipper-nih.log /Applications/Bitwig\ Studio.app/Contents/MacOS/BitwigStudio
```

## Java Extension Logging (host.println)

Output goes to Controller Script Console:
**Commander:** Cmd+Enter → type "console" → select "Show Control Script Console"

```java
host.println("Gilligan: message");
host.errorln("Gilligan: error message");
```

## Bitwig Log Files

```bash
# Main Bitwig log (extension loading, errors)
~/Library/Logs/Bitwig/BitwigStudio.log

# Previous session log
~/Library/Logs/Bitwig/BitwigStudio-previous-run.log

# Engine log
~/Library/Logs/Bitwig/engine.log

# Crash reports
~/Library/Logs/DiagnosticReports/BitwigPluginHost-*.ips
```

## Forking Dependencies

- Use the `https://github.com/audio-forge-rs` org
- Use `gh` CLI for GitHub operations
- Commit and push changes frequently
