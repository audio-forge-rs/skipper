# DJ Mode - Skipper/Gilligan

Endless music generation with automatic song cycling using Claude Code as the DJ.

## Quick Start

```bash
# Start playback
python3 tools/gilligan.py play

# Run DJ script to cycle through songs
./tools/dj.sh
```

## Manual DJing with Claude

Ask Claude to:
- "Give me blues" → Stages blues programs
- "Switch to funk" → Changes to funk
- "Play reggae at 85 BPM" → Creates and stages reggae
- "Keep the music going" → Claude keeps creating/switching

## Creating Songs

```bash
# Stage a single track
python3 tools/gilligan.py workflow \
  --track Piano \
  --tempo 120 \
  --abc '[CEG]2 [FAc]2 | [GBd]2 [CEG]2 |' \
  --name "Jazz Chords"

# Stage all tracks for a genre
for track in Piano Bass Guitar Kick Snare Violin; do
  python3 tools/gilligan.py workflow \
    --track "$track" \
    --tempo 90 \
    --abc '<abc notation>' \
    --name "Blues $track"
done
```

## Song Library

Songs are stored in `/Users/bedwards/skipper/songs/`:

| Genre | Tempo | Vibe |
|-------|-------|------|
| chicago-blues | 90 | Walking bass, blues riffs |
| bebop-jazz | 128 | ii-V-I, walking bass |
| tight-funk | 105 | Syncopated, tight |
| roots-reggae | 80 | One drop, skank |
| speed-metal | 200 | Blast beats, thrash |
| synthwave | 118 | 80s retro |
| surfabilly | 165 | Twangy, fast |

See full list: `songs/SONG-LIST.md`

## Loading Saved Songs

```bash
# Load a specific song
SONG="chicago-blues"
for f in songs/$SONG/*.json; do
  track=$(basename "$f" .json)
  python3 tools/gilligan.py stage --track "$track" --json "$f"
done
```

## ABC Notation Cheat Sheet

```
Notes: C D E F G A B (octave 3), c d e f g a b (octave 4)
Octave: C, = C2, C,, = C1, c' = C5
Sharps/Flats: ^C = C#, _D = Db
Duration: C2 = half note, C/2 = eighth
Chords: [CEG] = C major triad
Rest: z (z2 = half rest)
Bar: |
```

## Tips

1. **Tempo changes** apply immediately - use `--tempo` flag
2. **Half-time feel?** Use shorter durations (C/2 instead of C)
3. **More variety** - Different rhythms per track
4. **Silent track** - Use `z4 |` for empty program

## Troubleshooting

**Programs not loading:**
- Check Gilligan is running: `lsof -i :61170`
- Check Skipper is on tracks: `python3 tools/gilligan.py tracks`
- Restart Bitwig to reload plugins

**No sound:**
- `python3 tools/gilligan.py play`
- Check track has instrument after Skipper
