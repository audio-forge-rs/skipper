# Build Plugin

Build and bundle the Skipper CLAP plugin for use in Bitwig or other DAWs.

## Steps

1. Run release build:
```bash
cargo build --release
```

2. Bundle as CLAP:
```bash
cargo xtask bundle skipper --release
```

3. Report the bundle location to the user:
   - macOS: `target/bundled/skipper.clap/`
   - Windows/Linux: `target/bundled/skipper.clap`
   - VST3: `target/bundled/skipper.vst3`

The plugin can be loaded directly from this location in Bitwig via:
Settings > Plug-ins > Add Plugin Location > select the `target/bundled/` folder
