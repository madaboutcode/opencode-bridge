#!/bin/bash
set -e

# Create isolated temporary directories
BASE_DIR="/private/var/folders/7v/0z9xp1957554w5lkb4k0vk3m0000gn/T/opencode"
WORK_DIR=$(mktemp -d "$BASE_DIR/claude-async-obs-XXXXXX")
HOME_DIR="$WORK_DIR/home"
CLAUDE_CONFIG_DIR="$WORK_DIR/.claude"
CWD_DIR="$WORK_DIR/project"
mkdir -p "$HOME_DIR" "$CLAUDE_CONFIG_DIR" "$CWD_DIR"

# Create a hook script that logs to separate files per event
cat > "$WORK_DIR/hook-observer.sh" << 'EOF'
#!/bin/bash
# Metadata-only observer: records only field presence, never values
BASE_DIR="$(dirname "$0")"
INPUT=$(cat)

# Extract event name
EVENT=$(echo "$INPUT" | python3 -c "import sys, json; d=json.load(sys.stdin); print(d.get('hook_event_name', 'unknown'))")

# Create per-event log file
EVENT_LOG="$BASE_DIR/events/$EVENT.log"
mkdir -p "$BASE_DIR/events"

# Log timestamp and field presence
TIMESTAMP=$(date +%s.%N)
SESSION=$(echo "$INPUT" | python3 -c "import sys, json; d=json.load(sys.stdin); print('present' if 'session_id' in d else 'absent')")
CWD=$(echo "$INPUT" | python3 -c "import sys, json; d=json.load(sys.stdin); print('present' if 'cwd' in d else 'absent')")
SOURCE=$(echo "$INPUT" | python3 -c "import sys, json; d=json.load(sys.stdin); print('present' if 'source' in d else 'absent')")
TRANSCRIPT=$(echo "$INPUT" | python3 -c "import sys, json; d=json.load(sys.stdin); print('present' if 'transcript_path' in d else 'absent')")

echo "$TIMESTAMP SESSION=$SESSION CWD=$CWD SOURCE=$SOURCE TRANSCRIPT=$TRANSCRIPT" >> "$EVENT_LOG"

# Also append to combined events list
echo "$TIMESTAMP $EVENT" >> "$BASE_DIR/events-list.log"

exit 0
EOF
chmod +x "$WORK_DIR/hook-observer.sh"

# Create settings.json with ASYNC hooks for all events
cat > "$CLAUDE_CONFIG_DIR/settings.json" <<EOF
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "$WORK_DIR/hook-observer.sh",
            "async": true
          }
        ]
      }
    ],
    "StopFailure": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "$WORK_DIR/hook-observer.sh",
            "async": true
          }
        ]
      }
    ],
    "SessionEnd": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "$WORK_DIR/hook-observer.sh",
            "async": true
          }
        ]
      }
    ]
  }
}
EOF

# Set environment variables for isolated Claude
export HOME="$HOME_DIR"
export CLAUDE_CONFIG_DIR="$CLAUDE_CONFIG_DIR"

echo "=== Test Configuration ==="
echo "WORK_DIR=$WORK_DIR"
echo "HOME=$HOME"
echo "CLAUDE_CONFIG_DIR=$CLAUDE_CONFIG_DIR"
echo "CWD=$CWD_DIR"
echo ""

# Change to disposable working directory
cd "$CWD_DIR"

echo "=== Running claude --print with ASYNC hooks from disposable cwd ==="
# Capture exit status (CLI output is discarded; never retained in artifacts)
set +e
claude --print "" >/dev/null 2>&1
CLI_EXIT_STATUS=$?
set -e
echo "CLI exit status: $CLI_EXIT_STATUS"
echo ""

# Wait for async hooks to potentially complete
echo "Waiting 5 seconds for async hooks to complete..."
sleep 5

echo "=== Events list ==="
if [ -f "$WORK_DIR/events-list.log" ]; then
    cat "$WORK_DIR/events-list.log"
else
    echo "No events"
fi

echo ""
echo "=== Per-event logs ==="
for event in SessionStart StopFailure SessionEnd; do
    echo "--- $event ---"
    if [ -f "$WORK_DIR/events/$event.log" ]; then
        cat "$WORK_DIR/events/$event.log"
    else
        echo "Not observed"
    fi
done

echo ""
echo "=== Directory structure ==="
find "$WORK_DIR/events" -type f 2>/dev/null | head -10