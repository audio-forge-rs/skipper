#!/bin/bash
# DJ Script - Cycles through songs, playing each for its full duration
# Usage: ./dj.sh [--shuffle]

SONGS_DIR="/Users/bedwards/skipper/songs"
CLI="python3 /Users/bedwards/skipper/tools/gilligan.py"
TRACKS="Piano Bass Guitar Kick Snare Violin"

# Get list of songs (directories only, exclude markdown files)
SONGS=()
for dir in "$SONGS_DIR"/*/; do
    [ -d "$dir" ] && SONGS+=("$(basename "$dir")")
done

if [ ${#SONGS[@]} -eq 0 ]; then
    echo "No songs found in $SONGS_DIR"
    exit 1
fi

# Shuffle if requested
if [ "$1" = "--shuffle" ]; then
    SONGS=($(printf '%s\n' "${SONGS[@]}" | shuf))
    echo "=== Skipper DJ (Shuffle Mode) ==="
else
    echo "=== Skipper DJ ==="
fi

echo "Songs: ${SONGS[*]}"
echo "Ctrl+C to stop."
echo ""

# Function to get max bars from a song's JSON files
get_song_bars() {
    local song_path="$1"
    local max_bars=8

    for track in $TRACKS; do
        json_file="$song_path/$track.json"
        if [ -f "$json_file" ]; then
            bars=$(python3 -c "import json; print(int(json.load(open('$json_file')).get('lengthBars', 8)))" 2>/dev/null)
            if [ -n "$bars" ] && [ "$bars" -gt "$max_bars" ]; then
                max_bars=$bars
            fi
        fi
    done
    echo $max_bars
}

# Function to get tempo from song
get_song_tempo() {
    local song_path="$1"

    # First check tempo.txt
    if [ -f "$song_path/tempo.txt" ]; then
        cat "$song_path/tempo.txt"
        return
    fi

    # Default
    echo "120"
}

# Start playback
$CLI play >/dev/null 2>&1

song_index=0
while true; do
    song="${SONGS[$song_index]}"
    song_path="$SONGS_DIR/$song"

    tempo=$(get_song_tempo "$song_path")
    bars=$(get_song_bars "$song_path")

    # Calculate wait time: bars * 4 beats/bar * 60 / tempo
    wait_seconds=$(echo "scale=2; $bars * 4 * 60 / $tempo" | bc)

    echo ">>> $song"
    echo "    Tempo: ${tempo} BPM | Bars: ${bars} | Duration: ${wait_seconds}s"

    # Set tempo in Bitwig
    curl -s -X POST "http://localhost:61170/api/tempo" \
        -H "Content-Type: application/json" \
        -d "{\"bpm\": $tempo}" >/dev/null 2>&1

    # Load all tracks from song directory
    loaded=0
    for track in $TRACKS; do
        json_file="$song_path/$track.json"
        if [ -f "$json_file" ]; then
            cp "$json_file" "/tmp/skipper/$track.json"
            ((loaded++))
        fi
    done
    echo "    Loaded $loaded tracks"

    # Retry: touch files to ensure hot-reload triggers
    sleep 0.2
    for retry in 1 2 3; do
        for track in $TRACKS; do
            staging_file="/tmp/skipper/$track.json"
            if [ -f "$staging_file" ]; then
                touch "$staging_file"
            fi
        done
        sleep 0.1
    done

    # Wait for full song duration
    sleep "$wait_seconds"

    # Next song
    song_index=$(( (song_index + 1) % ${#SONGS[@]} ))

    # If we've played all songs, reshuffle if in shuffle mode
    if [ "$song_index" -eq 0 ] && [ "$1" = "--shuffle" ]; then
        SONGS=($(printf '%s\n' "${SONGS[@]}" | shuf))
        echo ""
        echo "--- Reshuffled playlist ---"
        echo ""
    fi
done
