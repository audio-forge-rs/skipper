#!/bin/bash
# DJ Script - Cycles through songs endlessly, changing every 16 bars
# Usage: ./dj.sh

SONGS_DIR="/Users/bedwards/skipper/songs"
CLI="python3 /Users/bedwards/skipper/tools/gilligan.py"
TRACKS="Piano Bass Guitar Kick Snare Violin"

# Get list of songs
SONGS=($(ls -d "$SONGS_DIR"/*/ 2>/dev/null | xargs -n1 basename))

if [ ${#SONGS[@]} -eq 0 ]; then
    echo "No songs found in $SONGS_DIR"
    exit 1
fi

echo "=== Skipper DJ ==="
echo "Songs: ${SONGS[*]}"
echo "Cycling every 16 bars. Ctrl+C to stop."
echo ""

# Start playback
$CLI play >/dev/null 2>&1

song_index=0
while true; do
    song="${SONGS[$song_index]}"
    song_path="$SONGS_DIR/$song"
    tempo=$(cat "$song_path/tempo.txt" 2>/dev/null || echo "120")

    echo ">>> Loading: $song (${tempo} BPM)"

    # Set tempo
    $CLI transport >/dev/null 2>&1  # Just to trigger connection

    # Load all tracks from song directory
    for track in $TRACKS; do
        json_file="$song_path/$track.json"
        if [ -f "$json_file" ]; then
            cp "$json_file" "/tmp/skipper/$track.json"
            # Notify Gilligan to trigger reload
            $CLI stage --track "$track" --json "$json_file" >/dev/null 2>&1
        fi
    done

    echo "    Playing for 16 bars..."

    # Wait for 16 bars at current tempo
    # 16 bars * 4 beats/bar = 64 beats
    # seconds = 64 beats / (tempo/60) = 64 * 60 / tempo
    wait_seconds=$(echo "scale=2; 64 * 60 / $tempo" | bc)
    sleep "$wait_seconds"

    # Next song
    song_index=$(( (song_index + 1) % ${#SONGS[@]} ))
done
