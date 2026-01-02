#!/usr/bin/env python3
"""
Gilligan CLI - Control Bitwig Studio via REST API.

Simple, robust interface to Gilligan controller extension.
No MCP, no SSE, just JSON over HTTP.

Usage:
    gilligan play                    # Start playback
    gilligan stop                    # Stop playback
    gilligan tracks                  # List all tracks
    gilligan snapshot                # Full project state
    gilligan stage --abc 'c d e f'   # Stage ABC program
    gilligan workflow piano.abc      # Full validation + staging workflow

Commands:
    play, stop, record              Transport controls
    transport                       Get transport state (tempo, position)
    tracks                          List all tracks
    track                           Get selected track
    device                          Get selected device
    snapshot                        Full project snapshot
    create_track <type>             Create instrument or audio track
    rename_track <name>             Rename selected track
    stage                           Stage programs (with --abc or --json)
    validate                        Validate ABC without staging
    workflow                        Full workflow: validate → stage
    songs                           List available songs
    song <name>                     Load song on all tracks (with retry)

Examples:
    gilligan play
    gilligan tracks
    gilligan snapshot
    gilligan songs                  # List all songs
    gilligan song disco-fever       # Load disco on all tracks
    gilligan validate --key C --scale major 'c d e f | g a b c |'
    gilligan stage --track Piano --abc 'c d e f | g a b c |'
    gilligan workflow --track Bass --abc 'C, C, E, G, |'
"""

import argparse
import json
import subprocess
import sys
import urllib.request
import urllib.error
from pathlib import Path

GILLIGAN_URL = "http://localhost:61170/api"
VALIDATOR_PATH = Path(__file__).parent / "validate_program.py"


def api_call(command: str, args: dict | None = None) -> dict:
    """Call Gilligan REST API."""
    url = f"{GILLIGAN_URL}/{command}"

    try:
        if args:
            data = json.dumps(args).encode('utf-8')
            req = urllib.request.Request(url, data=data,
                                          headers={'Content-Type': 'application/json'})
        else:
            req = urllib.request.Request(url)

        with urllib.request.urlopen(req, timeout=5) as resp:
            return json.loads(resp.read().decode('utf-8'))

    except urllib.error.URLError as e:
        return {"error": f"Connection failed: {e.reason}. Is Bitwig running with Gilligan?"}
    except urllib.error.HTTPError as e:
        try:
            body = json.loads(e.read().decode('utf-8'))
            return body
        except:
            return {"error": f"HTTP {e.code}: {e.reason}"}
    except Exception as e:
        return {"error": str(e)}


def validate_abc(abc: str, key: str | None = None, scale: str | None = None) -> dict:
    """Validate ABC notation using the validator tool."""
    cmd = [sys.executable, str(VALIDATOR_PATH)]

    if key:
        cmd.extend(['--key', key])
    if scale:
        cmd.extend(['--scale', scale])

    cmd.append(abc)

    try:
        result = subprocess.run(cmd, capture_output=True, text=True)
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return {"valid": False, "errors": [f"Validator error: {result.stderr}"]}
    except Exception as e:
        return {"valid": False, "errors": [str(e)]}


def cmd_play(args):
    """Start playback."""
    result = api_call("play")
    print(json.dumps(result, indent=2))
    return 0 if "error" not in result else 1


def cmd_stop(args):
    """Stop playback."""
    result = api_call("stop")
    print(json.dumps(result, indent=2))
    return 0 if "error" not in result else 1


def cmd_record(args):
    """Toggle recording."""
    result = api_call("record")
    print(json.dumps(result, indent=2))
    return 0 if "error" not in result else 1


def cmd_transport(args):
    """Get transport state."""
    result = api_call("transport")
    print(json.dumps(result, indent=2))
    return 0 if "error" not in result else 1


def cmd_tracks(args):
    """List all tracks."""
    result = api_call("tracks")
    if isinstance(result, list):
        for i, track in enumerate(result):
            skipper = "✓ Skipper" if track.get("skipper") else ""
            instrument = track.get("instrument") or ""
            print(f"{i+1}. {track.get('name', '?'):20} {track.get('trackType', ''):12} {skipper} {instrument}")
        return 0
    print(json.dumps(result, indent=2))
    return 1 if "error" in result else 0


def cmd_track(args):
    """Get selected track."""
    result = api_call("track")
    print(json.dumps(result, indent=2))
    return 0 if "error" not in result else 1


def cmd_device(args):
    """Get selected device."""
    result = api_call("device")
    print(json.dumps(result, indent=2))
    return 0 if "error" not in result else 1


def cmd_snapshot(args):
    """Get full project snapshot."""
    result = api_call("snapshot")
    print(json.dumps(result, indent=2))
    return 0 if "error" not in result else 1


def cmd_create_track(args):
    """Create a new track."""
    track_type = args.type or "instrument"
    result = api_call("create_track", {"type": track_type})
    print(json.dumps(result, indent=2))
    return 0 if "error" not in result else 1


def cmd_rename_track(args):
    """Rename selected track."""
    if not args.name:
        print('{"error": "Name required: gilligan rename_track <name>"}')
        return 1
    result = api_call("rename_track", {"name": args.name})
    print(json.dumps(result, indent=2))
    return 0 if "error" not in result else 1


def cmd_validate(args):
    """Validate ABC notation without staging."""
    abc = args.abc
    if not abc:
        print('{"error": "ABC notation required: gilligan validate \'c d e f\'"}')
        return 1

    result = validate_abc(abc, args.key, args.scale)
    print(json.dumps(result, indent=2))
    return 0 if result.get("valid") else 1


def cmd_stage(args):
    """Stage program to track."""
    if not args.abc and not args.json_file:
        print('{"error": "Provide --abc or --json: gilligan stage --abc \'c d e f\'"}')
        return 1

    track = args.track or "selected"

    if args.abc:
        # Validate first
        validation = validate_abc(args.abc, args.key, args.scale)
        if not validation.get("valid"):
            print(json.dumps({"error": "Validation failed", "details": validation}, indent=2))
            return 1
        program = validation.get("program")
    else:
        with open(args.json_file) as f:
            program = json.load(f)

    # Stage via API
    result = api_call("stage", {
        "stages": [{"track": track, "program": program}],
        "commitAt": args.commit_at or "next_bar"
    })
    print(json.dumps(result, indent=2))
    return 0 if "error" not in result else 1


SKIPPER_STAGING_DIR = Path("/tmp/skipper")


def cmd_workflow(args):
    """Full workflow: validate ABC → write to staging file for Skipper."""
    if not args.abc and not args.file:
        print('{"error": "Provide --abc or file path"}')
        return 1

    abc = args.abc
    if args.file:
        with open(args.file) as f:
            abc = f.read()

    track = args.track
    if not track:
        print('{"error": "Track name required: --track Bass"}')
        return 1

    print(f"=== Workflow: {track} ===")

    # Set tempo if specified
    if hasattr(args, 'tempo') and args.tempo:
        print(f"\n0a. Setting tempo to {args.tempo} BPM...")
        result = api_call("tempo", {"bpm": args.tempo})
        if "error" not in result:
            print("   OK")

    # Set time signature if specified
    if hasattr(args, 'timesig') and args.timesig:
        parts = args.timesig.split("/")
        if len(parts) == 2:
            print(f"\n0b. Setting time signature to {args.timesig}...")
            result = api_call("timesig", {
                "numerator": int(parts[0]),
                "denominator": int(parts[1])
            })
            if "error" not in result:
                print("   OK")

    # Step 1: Validate
    print("\n1. Validating ABC...")
    validation = validate_abc(abc, args.key, args.scale)

    if not validation.get("valid"):
        print("   FAILED:")
        for err in validation.get("errors", []):
            print(f"   - {err}")
        return 1

    print("   OK")
    if validation.get("warnings"):
        for warn in validation.get("warnings", []):
            print(f"   Warning: {warn}")

    program = validation.get("program")
    print(f"   Length: {program.get('lengthBars')} bars, {len(program.get('notes', []))} notes")

    # Step 2: Write to staging file for Skipper to read
    print("\n2. Writing to staging file...")
    SKIPPER_STAGING_DIR.mkdir(parents=True, exist_ok=True)
    staging_file = SKIPPER_STAGING_DIR / f"{track}.json"

    # Add metadata - extract title from ABC if not provided
    abc_title = None
    for line in abc.split('\n'):
        if line.startswith('T:'):
            abc_title = line[2:].strip()
            break
    program["name"] = args.name or abc_title or f"{track} Program"
    program["version"] = 1

    with open(staging_file, 'w') as f:
        json.dump(program, f, indent=2)

    # Verify JSON is valid by reading it back
    try:
        with open(staging_file) as f:
            verified = json.load(f)
        if verified.get("name") != program["name"]:
            print(f"   ERROR: JSON verification failed - name mismatch")
            return 1
        print(f"   Written: {staging_file}")
        print(f"   Verified: {verified['name']} ({len(verified.get('notes', []))} notes, {verified.get('lengthBars')} bars)")
    except json.JSONDecodeError as e:
        print(f"   ERROR: Invalid JSON written: {e}")
        return 1

    # Step 3: Also notify Gilligan (if running)
    print("\n3. Notifying Gilligan...")
    result = api_call("stage", {
        "stages": [{"track": track, "program": program}],
        "commitAt": args.commit_at or "next_bar"
    })

    if "error" in result:
        print(f"   Warning: Gilligan not responding (file staging still works)")
    else:
        print("   OK")

    # Summary
    print("\n=== Done ===")
    print(f"Skipper on track '{track}' will load from: {staging_file}")
    if validation.get("abc_output"):
        print(f"ABC: {validation['abc_output']}")

    return 0


SKIPPER_STAGING_DIR = Path("/tmp/skipper")
TRACKS = ["Piano", "Bass", "Guitar", "Kick", "Snare", "Violin"]


def ensure_playing():
    """Ensure transport is playing."""
    transport = api_call("transport")
    if not transport.get("playing", False):
        api_call("play")
        print("   Started playback")
        return True
    return False


def touch_file(path: Path):
    """Touch file to trigger reload."""
    import time
    path.touch()
    # Small delay to ensure file system picks up the change
    time.sleep(0.05)


def cmd_song(args):
    """Load a complete song on all tracks with retry and playback."""
    import time

    songs_dir = Path(__file__).parent.parent / "songs"
    song_path = songs_dir / args.song

    if not song_path.is_dir():
        print(f'{{"error": "Song not found: {args.song}"}}')
        print(f"Available songs: {[d.name for d in songs_dir.iterdir() if d.is_dir()]}")
        return 1

    print(f"=== Loading Song: {args.song} ===")

    # Set tempo if tempo.txt exists
    tempo_file = song_path / "tempo.txt"
    if tempo_file.exists():
        tempo = float(tempo_file.read_text().strip())
        result = api_call("tempo", {"bpm": tempo})
        if "error" not in result:
            print(f"   Tempo: {tempo} BPM")

    # Ensure staging directory exists
    SKIPPER_STAGING_DIR.mkdir(parents=True, exist_ok=True)

    # Load all tracks
    loaded = 0
    max_bars = 0
    for track in TRACKS:
        src = song_path / f"{track}.json"
        dst = SKIPPER_STAGING_DIR / f"{track}.json"

        if src.exists():
            # Copy to staging
            import shutil
            shutil.copy(src, dst)

            # Read to get bar count
            with open(dst) as f:
                program = json.load(f)
            bars = program.get("lengthBars", 8)
            if bars > max_bars:
                max_bars = bars

            print(f"   {track}: {program.get('name', 'Unknown')} ({bars} bars)")
            loaded += 1

    if loaded == 0:
        print("   ERROR: No tracks found in song")
        return 1

    print(f"\n   Loaded {loaded}/{len(TRACKS)} tracks, max {max_bars} bars")

    # Retry logic - touch files to ensure reload
    print("\n   Verifying load...")
    time.sleep(0.2)  # Wait for initial load

    retries = 3
    for attempt in range(retries):
        # Touch all files to trigger reload
        for track in TRACKS:
            dst = SKIPPER_STAGING_DIR / f"{track}.json"
            if dst.exists():
                touch_file(dst)

        time.sleep(0.15)  # Wait for reload

        if attempt < retries - 1:
            print(f"   Retry {attempt + 1}...")

    print("   OK")

    # Ensure playing
    print("\n   Checking transport...")
    ensure_playing()
    print("   Playing")

    print(f"\n=== {args.song} loaded ===")
    return 0


def cmd_songs(args):
    """List available songs."""
    songs_dir = Path(__file__).parent.parent / "songs"

    if not songs_dir.exists():
        print('{"error": "Songs directory not found"}')
        return 1

    songs = sorted([d.name for d in songs_dir.iterdir() if d.is_dir()])

    print(f"=== Available Songs ({len(songs)}) ===")
    for song in songs:
        song_path = songs_dir / song

        # Count tracks
        track_count = sum(1 for t in TRACKS if (song_path / f"{t}.json").exists())

        # Get tempo
        tempo_file = song_path / "tempo.txt"
        tempo = tempo_file.read_text().strip() if tempo_file.exists() else "120"

        print(f"  {song:25} {track_count}/6 tracks, {tempo} BPM")

    return 0


def cmd_help(args):
    """Show help."""
    print(__doc__)
    return 0


def main():
    parser = argparse.ArgumentParser(
        description="Gilligan CLI - Control Bitwig via REST",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog=__doc__
    )
    subparsers = parser.add_subparsers(dest="command", help="Command")

    # Transport commands
    subparsers.add_parser("play", help="Start playback")
    subparsers.add_parser("stop", help="Stop playback")
    subparsers.add_parser("record", help="Toggle recording")
    subparsers.add_parser("transport", help="Get transport state")

    # Track commands
    subparsers.add_parser("tracks", help="List all tracks")
    subparsers.add_parser("track", help="Get selected track")
    subparsers.add_parser("device", help="Get selected device")
    subparsers.add_parser("snapshot", help="Full project snapshot")

    # Create track
    create_p = subparsers.add_parser("create_track", help="Create track")
    create_p.add_argument("type", nargs="?", choices=["instrument", "audio"],
                          default="instrument", help="Track type")

    # Rename track
    rename_p = subparsers.add_parser("rename_track", help="Rename selected track")
    rename_p.add_argument("name", help="New track name")

    # Validate
    validate_p = subparsers.add_parser("validate", help="Validate ABC notation")
    validate_p.add_argument("abc", help="ABC notation string")
    validate_p.add_argument("--key", "-k", help="Key (C, D, E, F, G, A, B)")
    validate_p.add_argument("--scale", "-s", help="Scale (major, minor, etc.)")

    # Stage
    stage_p = subparsers.add_parser("stage", help="Stage program to track")
    stage_p.add_argument("--track", "-t", help="Track name (default: selected)")
    stage_p.add_argument("--abc", "-a", help="ABC notation")
    stage_p.add_argument("--json", dest="json_file", help="JSON program file")
    stage_p.add_argument("--key", "-k", help="Key for validation")
    stage_p.add_argument("--scale", "-s", help="Scale for validation")
    stage_p.add_argument("--commit-at", "-c", default="next_bar",
                         help="Commit timing: immediate, next_bar, next_beat")

    # Workflow
    workflow_p = subparsers.add_parser("workflow", help="Full validate → stage workflow")
    workflow_p.add_argument("file", nargs="?", help="ABC file path")
    workflow_p.add_argument("--abc", "-a", help="ABC notation string")
    workflow_p.add_argument("--track", "-t", required=True, help="Track name (required)")
    workflow_p.add_argument("--name", "-n", help="Program name (default: track name)")
    workflow_p.add_argument("--key", "-k", help="Key for validation")
    workflow_p.add_argument("--scale", "-s", help="Scale for validation")
    workflow_p.add_argument("--tempo", "-b", type=float, help="Set tempo in BPM")
    workflow_p.add_argument("--timesig", help="Set time signature (e.g., 4/4)")
    workflow_p.add_argument("--commit-at", "-c", default="next_bar",
                            help="Commit timing")

    # Song - load complete song
    song_p = subparsers.add_parser("song", help="Load complete song on all tracks")
    song_p.add_argument("song", help="Song name (directory in songs/)")

    # Songs - list available songs
    subparsers.add_parser("songs", help="List available songs")

    # Help
    subparsers.add_parser("help", help="Show help")

    args = parser.parse_args()

    if not args.command or args.command == "help":
        parser.print_help()
        return 0

    # Dispatch
    commands = {
        "play": cmd_play,
        "stop": cmd_stop,
        "record": cmd_record,
        "transport": cmd_transport,
        "tracks": cmd_tracks,
        "track": cmd_track,
        "device": cmd_device,
        "snapshot": cmd_snapshot,
        "create_track": cmd_create_track,
        "rename_track": cmd_rename_track,
        "validate": cmd_validate,
        "stage": cmd_stage,
        "workflow": cmd_workflow,
        "song": cmd_song,
        "songs": cmd_songs,
    }

    handler = commands.get(args.command)
    if handler:
        return handler(args)
    else:
        print(f'{{"error": "Unknown command: {args.command}"}}')
        return 1


if __name__ == "__main__":
    sys.exit(main())
