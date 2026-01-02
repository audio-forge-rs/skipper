# Music Program Workflow

## Overview

When generating music programs for Bitwig, use this workflow:

1. **Human describes** music in natural language
2. **Claude writes ABC notation** (text-based, token-efficient)
3. **Python validator** checks musical correctness
4. **Claude refines** based on validation feedback (repeat until valid)
5. **REST API stages** program to Gilligan → Skipper plugins
6. **Batch commit** on next bar

## Tools

### Python CLI (`tools/gilligan.py`)

```bash
# Transport
gilligan.py play
gilligan.py stop
gilligan.py transport

# Tracks
gilligan.py tracks
gilligan.py snapshot

# Validate ABC (no staging)
gilligan.py validate 'c d e f | g a b c |'
gilligan.py validate --key C --scale major 'c e g'

# Full workflow: validate → stage
gilligan.py workflow --track Piano --abc 'c d e f |'
gilligan.py workflow --track Bass --abc 'C, E, G, C |'
```

### ABC Validator (`tools/validate_program.py`)

```bash
python validate_program.py 'c d e f |'
python validate_program.py --key C --scale major '[CEG] [FAc] |'
```

## ABC Notation Quick Reference

```
Notes:     C D E F G A B (octave 4)    c d e f g a b (octave 5 = middle C octave)
Octave up: c' (C6)  c'' (C7)
Octave down: C, (C3)  C,, (C2)
Sharps:    ^C = C#    ^^C = Cx (double sharp)
Flats:     _D = Db    __D = Dbb
Duration:  C2 = half   C4 = whole   C/2 = eighth   C/4 = sixteenth
Rest:      z (quarter)  z2 (half)  z4 (whole)
Chords:    [CEG] = C major chord (simultaneous)
Bar line:  |
```

## Program Constraints

### Bar Length
Must be power-of-2 bars:
- Valid: 0.125, 0.25, 0.5, 1, 2, 4, 8, 16 bars
- Invalid: 1.5, 3, 0.75 bars

### Drums
Single track, C3 note (MIDI 48 = `C,` in ABC):
```
C, C, C, C, |     # Four on the floor
C, z C, z |       # Kick on 1 and 3
z C, z C, |       # Snare on 2 and 4
```

### MIDI/Bitwig Note Names
- MIDI 60 = C4 (standard) = C3 (Bitwig display)
- ABC lowercase `c` = MIDI 60 = middle C

## Batch Updates

- Only include tracks with changes
- Empty 1-bar program to clear/silence track
- No staged program + commit = no-op

```python
# Stage multiple tracks
gilligan.py workflow --track Bass --abc 'C, E, G, C |'
gilligan.py workflow --track Piano --abc '[CEG]4 |'
```

## REST API (Direct)

```bash
# Simple commands
curl http://localhost:61170/api/play
curl http://localhost:61170/api/tracks
curl http://localhost:61170/api/snapshot

# With arguments
curl -X POST http://localhost:61170/api/stage \
  -H "Content-Type: application/json" \
  -d '{"stages": [{"track": "Piano", "program": {...}}]}'
```

## Error Handling

Validation errors fast-fail before staging:
- Invalid ABC syntax → parse error
- Invalid bar length → constraint error
- Notes outside scale → warning (not error)
- Empty program → warning (silences track)

## Sources

- [gRPC vs REST Benchmarks 2025](https://markaicode.com/grpc-vs-rest-benchmarks-2025/)
- [API Design Comparison](https://www.javacodegeeks.com/2025/12/api-design-in-java-rest-graphql-grpc-comparison.html)
