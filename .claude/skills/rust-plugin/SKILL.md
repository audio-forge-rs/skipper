---
name: rust-plugin
description: Build, test, and bundle Rust nih-plug audio plugins
triggers:
  - build plugin
  - bundle clap
  - run tests
  - cargo build
  - nih-plug
---

# Rust Plugin Build & Test Skill

This skill handles building, testing, and bundling nih-plug audio plugins.

## Build Commands

### Debug Build
```bash
cargo build
```

### Release Build
```bash
cargo build --release
```

### Bundle CLAP Plugin
```bash
cargo xtask bundle skipper --release
```
Output: `target/bundled/skipper.clap`

### Run Tests
```bash
cargo test
```

### Format & Lint
```bash
cargo fmt
cargo clippy -- -D warnings
```

## Bundle Locations

After bundling, the plugin is available at:
- **macOS**: `target/bundled/skipper.clap/` (directory bundle)
- **Windows**: `target/bundled/skipper.clap` (single file)
- **Linux**: `target/bundled/skipper.clap` (single file)

Also creates VST3: `target/bundled/skipper.vst3`

## Troubleshooting

### Build Errors
1. Check Rust version: `rustc --version` (minimum 1.70)
2. Update dependencies: `cargo update`
3. Clean build: `cargo clean && cargo build`

### Plugin Not Loading
1. Verify bundle format matches platform
2. Check DAW plugin scan log
3. Use `nih_log!()` for debug output
