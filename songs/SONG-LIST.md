# Skipper Song Library

19 songs across multiple genres. Use `./tools/dj.sh` to cycle through them.

## Songs

| # | Song | Genre | BPM | Key | Features |
|---|------|-------|-----|-----|----------|
| 1 | bittersweet-violin | Ballad | 70 | Am | Emotional violin, sparse arrangement |
| 2 | chicago-blues | Blues | 90 | E | 12-bar, walking bass, blues riffs |
| 3 | bebop-jazz | Jazz | 128 | Dm/C | ii-V-I, walking bass, comping |
| 4 | big-band-swing | Big Band | 150 | Bb | Brass stabs, driving swing |
| 5 | west-coast-swing | Swing | 115 | C | Triple-step feel, bluesy |
| 6 | 90s-boom-bap | Hip Hop | 92 | Am | Boom bap, sampled keys |
| 7 | seattle-grunge | Grunge | 75 | Em | Heavy power chords, sludgy |
| 8 | classical-mozart | Classical | 120 | C | Arpeggios, elegant |
| 9 | surfabilly | Surf/Rockabilly | 165 | E | Twangy, fast |
| 10 | tight-funk | Funk | 105 | E | Syncopated, tight groove |
| 11 | roots-reggae | Reggae | 80 | C | One drop, skank |
| 12 | disco-fever | Disco | 120 | Am | Four on floor, strings |
| 13 | speed-metal | Speed Metal | 200 | Em | Blast beats, thrash |
| 14 | post-punk | Post-Punk | 140 | Am | Angular, driving |
| 15 | synthwave | Synthwave | 118 | Am | 80s retro, arpeggios |
| 16 | honky-tonk-country | Country | 110 | G | Chicken pickin, fiddle |
| 17 | afrobeat | Afrobeat | 100 | Em | Polyrhythmic |
| 18 | bossa-nova | Bossa Nova | 130 | Am | Brazilian, smooth |
| 19 | deep-house | House | 126 | Am | Four on floor, offbeat |

## Quick Commands

```bash
# Start DJ mode
./tools/dj.sh

# Load specific song
for f in songs/chicago-blues/*.json; do
  track=$(basename "$f" .json)
  python3 tools/gilligan.py workflow --track "$track" --json "$f"
done

# Play/Stop
python3 tools/gilligan.py play
python3 tools/gilligan.py stop
```

## Known Issues

- Some songs may sound similar due to uniform rhythmic patterns
- Half-time feel on some patterns - need more varied note durations
- Need more syncopation and rhythmic variety

---
*19 songs created by Claude Code for Skipper/Gilligan*
