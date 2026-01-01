# Common Issues

## Skipper Plugin

**"Track info not available"**
- Ensure using CLAP format (not VST3)
- Host must support CLAP track-info extension (Bitwig 4.4+)

**"Host info empty"**
- Some hosts don't populate all fields
- Bitwig populates name/version; other hosts vary

## Gilligan Extension

**"Extension not loading" / "No extensions found"**
1. Check Bitwig log file for errors:
   ```bash
   cat ~/Library/Logs/Bitwig/BitwigStudio.log | grep -i "gilligan\|extension\|error"
   ```
2. Common causes:
   - Bitwig API bundled in JAR (must use `<scope>provided</scope>`)
   - Java version mismatch (21+ required)
   - Missing SPI service file

**"Extension not in Add Controller menu"**
- Check log: `~/Library/Logs/Bitwig/BitwigStudio.log`
- Look for: `[extension-registry error] Error scanning extension file`

**Verify JAR contents:**
```bash
jar tf target/gilligan-*.jar | grep "^com/" | head -20
# Should only show: com/bedwards/gilligan/...
# Should NOT show: com/bitwig/...
```

**"MCP server not responding"**
- Check port 61170 is not in use: `lsof -i :61170`
- Verify Gilligan is enabled in Bitwig Settings > Controllers
- Check firewall settings

**"Values not updating"**
- Call `markInterested()` on values you want to observe
- Bitwig only sends updates for interested values

## Forked Dependencies

### nih-plug (audio-forge-rs/nih-plug)
- **Why:** Added CLAP track-info extension support, Arc-based track info caching
- **Key additions:**
  - `InitContext::track_info()` - CLAP track-info extension
  - `ProcessContext::track_info()` - Arc<TrackInfo> for audio thread safety

### baseview (audio-forge-rs/baseview)
- **Why:** Fixed null pointer crash in macOS view initialization
- **Branch:** `fix-null-window-crash`

### egui-baseview (audio-forge-rs/egui-baseview)
- **Why:** Updated to use our forked baseview
- **Branch:** `fix-null-window-crash`
