# Programs Directory

MIDI programs in ABC notation for Skipper.

## Directory Structure

Each Bitwig project gets its own directory:

```
programs/
  template/                 # Example template (checked in)
    current/               # Active programs (what's playing now)
      bass.abc
      guitar.abc
      piano.abc
    versions/              # Version history
      v1/
        bass.abc
        guitar.abc
        piano.abc

  my-song/                 # Your project (git-ignored)
    current/
      bass.abc
      drums.abc
    versions/
      v1/
      v2/
```

## Workflow

1. Copy `template/` to `<your-project-name>/`
2. Edit programs in `current/`
3. When happy, copy to `versions/vN/`
4. Stage to Bitwig via gilligan CLI:
   ```bash
   python tools/gilligan.py workflow --track Bass --file programs/my-song/current/bass.abc
   ```

## ABC Notation

```
Notes:     C D E F G A B (octave 4)    c d e f g a b (octave 5)
Octave:    C, = C3    C,, = C2    c' = C6    c'' = C7
Sharps:    ^C = C#    Flats: _D = Db
Duration:  C2 = half   C4 = whole   C/2 = 8th   C/4 = 16th
Rest:      z = quarter   z2 = half   z4 = whole
Chords:    [CEG] = simultaneous
Bar:       |
```

## Constraints

- Bar length: power-of-2 (1, 2, 4, 8, 16 bars)
- Drums: use C, (MIDI 48 / C3) as trigger
