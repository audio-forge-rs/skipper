# Architecture

## System Overview

Skipper + Gilligan is a multi-component system for AI-assisted music production:

- **Skipper** - Rust CLAP/VST3 plugin (nih-plug) with egui GUI - one instance per track
- **Gilligan** - Java Bitwig Controller Extension with MCP Server - acts as central hub

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

## Beat-Synced Multi-Track Commit Flow

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
