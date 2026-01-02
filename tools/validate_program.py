#!/usr/bin/env python3
"""
ABC-first music program validator for Skipper/Gilligan workflow.

Validates ABC notation for musical correctness, converts to MIDI program format,
and validates the result. Fast-fails on any error.

Usage:
    python validate_program.py 'C D E F | G A B c |'
    python validate_program.py --key C --scale major 'c d e f | g a b c |'
    python validate_program.py --file melody.abc
    python validate_program.py --json '{"lengthBars": 1, "notes": [...]}'

ABC Notation Reference:
- Notes: C D E F G A B c d e f g a b (lowercase = octave 5, uppercase = octave 4)
- Middle C (C4, MIDI 60) = c (lowercase c)
- Octave up: ' (c' = C6, c'' = C7)
- Octave down: , (C, = C3, C,, = C2)
- Sharps/flats: ^C = C#, _D = Db
- Duration: number suffix (C2 = half note, C/2 = eighth note)
- Rest: z (z2 = half rest)
- Bar line: |
- Chords: [CEG] = C major chord (simultaneous notes)
- Ties: C-C = tied notes

Drum track (single note on C3 = MIDI 48):
    C, C, C, C, | z C, z C, |

Complex melody example:
    c d e f | g2 a b | c'4 |

Chord progression example:
    [CEG] [FAc] | [GBd] [CEG] |
"""

import json
import re
import sys
from dataclasses import dataclass
from typing import Any

# Valid bar lengths: powers of 2 from 1/8 to 16 bars
VALID_BAR_LENGTHS = [0.125, 0.25, 0.5, 1.0, 2.0, 4.0, 8.0, 16.0]

# ABC note mapping (C4 = middle C = MIDI 60 = lowercase c in ABC)
# Note: ABC standard has middle C as C (uppercase), but we follow the common
# convention where lowercase c = C4 = MIDI 60
ABC_BASE_NOTES = {
    'C': 48, 'D': 50, 'E': 52, 'F': 53, 'G': 55, 'A': 57, 'B': 59,
    'c': 60, 'd': 62, 'e': 64, 'f': 65, 'g': 67, 'a': 69, 'b': 71,
}

NOTE_NAMES = ['C', 'C#', 'D', 'D#', 'E', 'F', 'F#', 'G', 'G#', 'A', 'A#', 'B']

SCALES = {
    'major': [0, 2, 4, 5, 7, 9, 11],
    'minor': [0, 2, 3, 5, 7, 8, 10],
    'dorian': [0, 2, 3, 5, 7, 9, 10],
    'mixolydian': [0, 2, 4, 5, 7, 9, 10],
    'pentatonic_major': [0, 2, 4, 7, 9],
    'pentatonic_minor': [0, 3, 5, 7, 10],
    'blues': [0, 3, 5, 6, 7, 10],
    'chromatic': list(range(12)),
}


@dataclass
class ValidationResult:
    valid: bool
    abc_input: str
    program: dict | None
    errors: list[str]
    warnings: list[str]
    abc_output: str | None  # Normalized ABC

    def to_dict(self) -> dict:
        return {
            'valid': self.valid,
            'abc_input': self.abc_input,
            'program': self.program,
            'errors': self.errors,
            'warnings': self.warnings,
            'abc_output': self.abc_output,
        }


def midi_to_note_name(midi: int) -> str:
    """Convert MIDI pitch to note name (e.g., 60 -> 'C4')."""
    octave = (midi // 12) - 1
    note = NOTE_NAMES[midi % 12]
    return f"{note}{octave}"


def midi_to_abc(pitch: int, duration_beats: float = 1.0) -> str:
    """Convert MIDI pitch and duration to ABC notation."""
    octave = pitch // 12
    note_in_octave = pitch % 12

    natural_notes = [0, 2, 4, 5, 7, 9, 11]  # C D E F G A B
    note_names_list = ['C', 'D', 'E', 'F', 'G', 'A', 'B']

    is_sharp = note_in_octave not in natural_notes
    if is_sharp:
        base_note = note_in_octave - 1
    else:
        base_note = note_in_octave

    note_idx = natural_notes.index(base_note)
    note_char = note_names_list[note_idx]

    # Octave 5 (MIDI 60-71) uses lowercase
    if octave >= 5:
        note_char = note_char.lower()
        result = '^' + note_char if is_sharp else note_char
        if octave > 5:
            result += "'" * (octave - 5)
    else:
        result = '^' + note_char if is_sharp else note_char
        if octave < 4:
            result += "," * (4 - octave)

    # Duration
    if abs(duration_beats - 0.25) < 0.01:
        result += '/4'
    elif abs(duration_beats - 0.5) < 0.01:
        result += '/2'
    elif abs(duration_beats - 1.0) < 0.01:
        pass  # Default, no suffix
    elif abs(duration_beats - 2.0) < 0.01:
        result += '2'
    elif abs(duration_beats - 3.0) < 0.01:
        result += '3'
    elif abs(duration_beats - 4.0) < 0.01:
        result += '4'
    elif duration_beats == int(duration_beats):
        result += str(int(duration_beats))
    else:
        # Express as fraction of quarter note
        numerator = int(duration_beats * 4)
        if numerator > 0:
            result += f'{numerator}/4'

    return result


def parse_abc_note(token: str) -> tuple[int, float]:
    """
    Parse a single ABC note token to (MIDI pitch, duration in beats).

    Returns (-1, duration) for rests.
    Raises ValueError for invalid tokens.
    """
    if not token:
        raise ValueError("Empty note token")

    note = token.strip()

    # Handle rest
    if note.startswith('z') or note.startswith('Z'):
        duration = 1.0
        rest_part = note[1:]
        if rest_part:
            if '/' in rest_part:
                parts = rest_part.split('/')
                if parts[0]:
                    duration = float(parts[0]) / float(parts[1])
                else:
                    duration = 1.0 / float(parts[1])
            else:
                try:
                    duration = float(rest_part)
                except ValueError:
                    pass
        return (-1, duration)

    accidental = 0

    # Check for accidental prefix
    if note.startswith('^^'):
        accidental = 2
        note = note[2:]
    elif note.startswith('^'):
        accidental = 1
        note = note[1:]
    elif note.startswith('__'):
        accidental = -2
        note = note[2:]
    elif note.startswith('_'):
        accidental = -1
        note = note[1:]
    elif note.startswith('='):
        accidental = 0
        note = note[1:]

    if not note:
        raise ValueError(f"Invalid ABC note: {token}")

    # Get base note
    base_char = note[0]
    if base_char not in ABC_BASE_NOTES:
        raise ValueError(f"Invalid note letter '{base_char}' in: {token}")

    midi = ABC_BASE_NOTES[base_char] + accidental
    note = note[1:]

    # Count octave modifiers
    while note and note[0] == "'":
        midi += 12
        note = note[1:]
    while note and note[0] == ",":
        midi -= 12
        note = note[1:]

    # Validate MIDI range
    if midi < 0 or midi > 127:
        raise ValueError(f"Note out of MIDI range (0-127): {token} -> {midi}")

    # Parse duration
    duration = 1.0
    if note:
        if '/' in note:
            parts = note.split('/')
            if parts[0]:
                duration = float(parts[0]) / float(parts[1])
            else:
                duration = 1.0 / float(parts[1])
        else:
            try:
                duration = float(note)
            except ValueError:
                raise ValueError(f"Invalid duration in: {token}")

    return (midi, duration)


def parse_abc_chord(chord_str: str) -> list[tuple[int, float]]:
    """
    Parse ABC chord notation [CEG] to list of (MIDI, duration) tuples.

    All notes in a chord share the same duration (from the last note or suffix).
    """
    # Remove brackets
    inner = chord_str.strip('[]')
    if not inner:
        return []

    # Check for duration suffix after the chord content
    # e.g., [CEG]2 means all notes are half notes
    chord_duration = 1.0
    duration_match = re.search(r'(\d+(?:/\d+)?|\d*/\d+)$', chord_str.rstrip(']'))

    notes = []
    # Split into individual notes - be careful with accidentals
    note_pattern = re.compile(r"(\^{1,2}|_{1,2}|=)?([A-Ga-g])([',]*)")

    for match in note_pattern.finditer(inner):
        accidental_str = match.group(1) or ''
        note_letter = match.group(2)
        octave_mod = match.group(3) or ''

        note_token = accidental_str + note_letter + octave_mod
        try:
            midi, _ = parse_abc_note(note_token)
            notes.append((midi, chord_duration))
        except ValueError:
            continue

    return notes


def tokenize_abc(abc_str: str) -> list[str]:
    """
    Tokenize ABC notation string into note/chord/rest tokens.

    Handles:
    - Single notes: C, c, ^C, _D, C', C,, C2, C/2
    - Chords: [CEG], [CEG]2
    - Rests: z, z2, z/2
    - Ignores bar lines, headers, and whitespace
    """
    tokens = []

    # Remove header lines (X:, K:, M:, etc.)
    lines = abc_str.strip().split('\n')
    note_lines = [l for l in lines if not re.match(r'^[A-Z]:', l)]
    content = ' '.join(note_lines)

    # Remove bar lines but preserve structure
    content = re.sub(r'\|+', ' ', content)
    content = re.sub(r'\s+', ' ', content).strip()

    # Tokenize
    i = 0
    while i < len(content):
        c = content[i]

        # Skip whitespace
        if c.isspace():
            i += 1
            continue

        # Chord
        if c == '[':
            end = content.find(']', i)
            if end == -1:
                i += 1
                continue
            # Include duration suffix if present
            chord = content[i:end + 1]
            j = end + 1
            while j < len(content) and (content[j].isdigit() or content[j] == '/'):
                chord += content[j]
                j += 1
            tokens.append(chord)
            i = j
            continue

        # Note or rest
        if c in 'ABCDEFGabcdefgzZ^_=':
            token = ''
            # Accidentals
            while i < len(content) and content[i] in '^_=':
                token += content[i]
                i += 1
            # Note letter
            if i < len(content) and content[i] in 'ABCDEFGabcdefgzZ':
                token += content[i]
                i += 1
            # Octave modifiers
            while i < len(content) and content[i] in "',":
                token += content[i]
                i += 1
            # Duration
            while i < len(content) and (content[i].isdigit() or content[i] == '/'):
                token += content[i]
                i += 1
            if token:
                tokens.append(token)
            continue

        # Skip unknown characters
        i += 1

    return tokens


def abc_to_program(abc_str: str, beats_per_bar: int = 4,
                   default_velocity: float = 0.8) -> tuple[dict, list[str]]:
    """
    Convert ABC notation to MIDI program format.

    Returns:
        Tuple of (program dict, list of parse errors)
    """
    errors = []
    notes = []
    current_beat = 0.0

    tokens = tokenize_abc(abc_str)

    for token in tokens:
        try:
            if token.startswith('['):
                # Chord - all notes start at same beat
                chord_notes = parse_abc_chord(token)

                # Get duration from last character if present
                duration = 1.0
                duration_match = re.search(r'(\d+(?:/\d+)?|\d*/\d+)$', token.rstrip(']'))
                if duration_match:
                    dur_str = duration_match.group(1)
                    if '/' in dur_str:
                        parts = dur_str.split('/')
                        if parts[0]:
                            duration = float(parts[0]) / float(parts[1])
                        else:
                            duration = 1.0 / float(parts[1])
                    else:
                        duration = float(dur_str)

                for midi, _ in chord_notes:
                    notes.append({
                        'pitch': midi,
                        'startBeat': current_beat,
                        'lengthBeats': duration,
                        'velocity': default_velocity
                    })
                current_beat += duration

            else:
                # Single note or rest
                midi, duration = parse_abc_note(token)
                if midi >= 0:  # Not a rest
                    notes.append({
                        'pitch': midi,
                        'startBeat': current_beat,
                        'lengthBeats': duration,
                        'velocity': default_velocity
                    })
                current_beat += duration

        except ValueError as e:
            errors.append(str(e))

    # Calculate bar length
    total_beats = current_beat
    bars = total_beats / beats_per_bar if beats_per_bar > 0 else total_beats

    # Find next valid bar length (round up)
    length_bars = 1.0
    for valid in VALID_BAR_LENGTHS:
        if valid >= bars - 0.001:  # Small epsilon for float comparison
            length_bars = valid
            break
    else:
        length_bars = VALID_BAR_LENGTHS[-1]

    program = {
        'lengthBars': length_bars,
        'notes': notes
    }

    return program, errors


def program_to_abc(program: dict, beats_per_bar: int = 4) -> str:
    """Convert MIDI program to ABC notation."""
    notes = program.get('notes', [])
    if not notes:
        return "z4 |"

    # Group notes by start time for chords
    by_time: dict[float, list[dict]] = {}
    for note in notes:
        start = note.get('startBeat', 0)
        if start not in by_time:
            by_time[start] = []
        by_time[start].append(note)

    # Sort by start time
    sorted_times = sorted(by_time.keys())

    abc_parts = []
    notes_in_bar = 0

    for start_time in sorted_times:
        group = by_time[start_time]

        # Check if new bar
        bar_num = int(start_time // beats_per_bar)
        if bar_num > 0 and notes_in_bar > 0 and start_time % beats_per_bar < 0.01:
            abc_parts.append('|')
            notes_in_bar = 0

        if len(group) == 1:
            # Single note
            n = group[0]
            abc_parts.append(midi_to_abc(n['pitch'], n.get('lengthBeats', 1.0)))
        else:
            # Chord
            duration = group[0].get('lengthBeats', 1.0)
            chord_notes = [midi_to_abc(n['pitch'], 1.0) for n in group]
            chord_str = '[' + ''.join(chord_notes) + ']'
            if abs(duration - 1.0) > 0.01:
                if duration == int(duration):
                    chord_str += str(int(duration))
                elif abs(duration - 0.5) < 0.01:
                    chord_str += '/2'
                elif abs(duration - 0.25) < 0.01:
                    chord_str += '/4'
            abc_parts.append(chord_str)

        notes_in_bar += 1

    # Add final bar line
    abc_parts.append('|')

    return ' '.join(abc_parts)


def get_scale_notes(root: int, scale_name: str) -> set[int]:
    """Get all MIDI notes in a scale across all octaves."""
    if scale_name not in SCALES:
        return set()
    intervals = SCALES[scale_name]
    notes = set()
    for octave in range(11):
        base = octave * 12
        for interval in intervals:
            note = base + (root % 12) + interval
            if 0 <= note <= 127:
                notes.add(note)
    return notes


def validate_program(program: dict, key: int | None = None,
                     scale: str | None = None,
                     beats_per_bar: int = 4) -> tuple[list[str], list[str]]:
    """
    Validate a MIDI program.

    Returns:
        Tuple of (errors, warnings)
    """
    errors = []
    warnings = []

    length_bars = program.get('lengthBars')
    if length_bars is None:
        errors.append("Missing 'lengthBars'")
    elif not any(abs(length_bars - v) < 0.001 for v in VALID_BAR_LENGTHS):
        errors.append(f"Invalid bar length {length_bars}. Must be: {VALID_BAR_LENGTHS}")

    notes = program.get('notes', [])
    if not notes:
        warnings.append("Empty program (will silence track)")
        return errors, warnings

    total_beats = (length_bars or 0) * beats_per_bar
    scale_notes = get_scale_notes(key, scale) if key is not None and scale else None

    for i, note in enumerate(notes):
        prefix = f"Note {i}"

        pitch = note.get('pitch')
        if pitch is None:
            errors.append(f"{prefix}: Missing 'pitch'")
        elif not isinstance(pitch, int) or pitch < 0 or pitch > 127:
            errors.append(f"{prefix}: Invalid pitch {pitch}")
        elif scale_notes and pitch not in scale_notes:
            warnings.append(f"{prefix} ({midi_to_note_name(pitch)}): Outside {NOTE_NAMES[key % 12]} {scale} scale")

        start = note.get('startBeat')
        if start is None:
            errors.append(f"{prefix}: Missing 'startBeat'")
        elif start < 0:
            errors.append(f"{prefix}: Negative startBeat")
        elif total_beats > 0 and start >= total_beats:
            errors.append(f"{prefix}: Starts past program end")

        length = note.get('lengthBeats')
        if length is None:
            errors.append(f"{prefix}: Missing 'lengthBeats'")
        elif length <= 0:
            errors.append(f"{prefix}: Non-positive duration")

        velocity = note.get('velocity')
        if velocity is None:
            errors.append(f"{prefix}: Missing 'velocity'")
        elif velocity < 0 or velocity > 1:
            errors.append(f"{prefix}: Velocity must be 0.0-1.0")

    return errors, warnings


def validate_abc(abc_str: str, key: int | None = None,
                 scale: str | None = None,
                 beats_per_bar: int = 4) -> ValidationResult:
    """
    Full validation pipeline: ABC -> parse -> validate -> normalize.

    Fast-fails on critical errors.
    """
    errors = []
    warnings = []

    # Step 1: Parse ABC to program
    program, parse_errors = abc_to_program(abc_str, beats_per_bar)
    if parse_errors:
        return ValidationResult(
            valid=False,
            abc_input=abc_str,
            program=None,
            errors=parse_errors,
            warnings=[],
            abc_output=None
        )

    # Step 2: Validate program
    val_errors, val_warnings = validate_program(program, key, scale, beats_per_bar)
    errors.extend(val_errors)
    warnings.extend(val_warnings)

    if errors:
        return ValidationResult(
            valid=False,
            abc_input=abc_str,
            program=program,
            errors=errors,
            warnings=warnings,
            abc_output=None
        )

    # Step 3: Generate normalized ABC
    abc_output = program_to_abc(program, beats_per_bar)

    return ValidationResult(
        valid=True,
        abc_input=abc_str,
        program=program,
        errors=[],
        warnings=warnings,
        abc_output=abc_output
    )


def main():
    """CLI entry point."""
    import argparse

    parser = argparse.ArgumentParser(
        description='Validate ABC notation and convert to MIDI program',
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  %(prog)s 'C D E F | G A B c |'
  %(prog)s --key C --scale major 'c d e f g'
  %(prog)s '[CEG] [FAc] | [GBd] [CEG] |'
  %(prog)s --json '{"lengthBars": 1, "notes": [{"pitch": 60, ...}]}'
        """
    )
    parser.add_argument('input', nargs='?', help='ABC notation or JSON program')
    parser.add_argument('--file', '-f', help='Read ABC from file')
    parser.add_argument('--json', action='store_true', help='Input is JSON program format')
    parser.add_argument('--key', '-k', help='Key for scale validation (C, D, E, F, G, A, B)')
    parser.add_argument('--scale', '-s', help='Scale for validation (major, minor, pentatonic_major, etc.)')
    parser.add_argument('--output', '-o', choices=['json', 'abc', 'both'], default='json',
                        help='Output format')

    args = parser.parse_args()

    # Get input
    if args.file:
        with open(args.file) as f:
            input_str = f.read()
    elif args.input:
        input_str = args.input
    else:
        input_str = sys.stdin.read()

    # Parse key
    key = None
    if args.key:
        key_map = {'C': 0, 'D': 2, 'E': 4, 'F': 5, 'G': 7, 'A': 9, 'B': 11}
        key_upper = args.key.upper().replace('#', '').replace('B', '')
        if key_upper in key_map:
            key = key_map[key_upper]
            if '#' in args.key:
                key += 1
            elif 'b' in args.key.lower():
                key -= 1
            key = key % 12

    # Process
    if args.json:
        try:
            program = json.loads(input_str)
            val_errors, val_warnings = validate_program(program, key, args.scale)
            result = ValidationResult(
                valid=len(val_errors) == 0,
                abc_input='',
                program=program,
                errors=val_errors,
                warnings=val_warnings,
                abc_output=program_to_abc(program) if len(val_errors) == 0 else None
            )
        except json.JSONDecodeError as e:
            result = ValidationResult(
                valid=False,
                abc_input=input_str,
                program=None,
                errors=[f"Invalid JSON: {e}"],
                warnings=[],
                abc_output=None
            )
    else:
        result = validate_abc(input_str, key, args.scale)

    # Output
    if args.output == 'abc' and result.abc_output:
        print(result.abc_output)
    elif args.output == 'both':
        output = result.to_dict()
        print(json.dumps(output, indent=2))
    else:
        output = result.to_dict()
        print(json.dumps(output, indent=2))

    sys.exit(0 if result.valid else 1)


if __name__ == '__main__':
    main()
