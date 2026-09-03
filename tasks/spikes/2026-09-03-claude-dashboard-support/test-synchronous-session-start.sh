#!/bin/bash
set -e

# Create isolated temporary directories
BASE_DIR="/private/var/folders/7v/0z9xp1957554w5lkb4k0vk3m0000gn/T/opencode"
WORK_DIR=$(mktemp -d "$BASE_DIR/claude-sync-test-XXXXXX")
HOME_DIR="$WORK_DIR/home"
CLAUDE_CONFIG_DIR="$WORK_DIR/.claude"
CWD_DIR="$WORK_DIR/project"
mkdir -p "$HOME_DIR" "$CLAUDE_CONFIG_DIR" "$CWD_DIR"

# Create a minimal hook script that logs ONLY event name and presence of fields
# This script never writes values, only records that fields exist
cat > "$WORK_DIR/hook-observer.sh" << 'EOF'
#!/bin/bash
# Metadata-only observer: records only field presence, never values
LOGFILE="$(dirname "$0")/hook-events.log"
INPUT=$(cat)

# Use python3 to extract ONLY field names (keys), not values
EVENT=$(echo "$INPUT" | python3 -c "import sys, json; d=json.load(sys.stdin); print(d.get('hook_event_name', 'unknown'))")
SESSION=$(echo "$INPUT" | python3 -c "import sys, json; d=json.load(sys.stdin); print('present' if 'session_id' in d else 'absent')")
CWD=$(echo "$INPUT" | python3 -c "import sys, json; d=json.load(sys.stdin); print('present' if 'cwd' in d else 'absent')")
SOURCE=$(echo "$INPUT" | python3 -c "import sys, json; d=json.load(sys.stdin); print('present' if 'source' in d else 'absent')")
TRANSCRIPT=$(echo "$INPUT" | python3 -c "import sys, json; d=json.load(sys.stdin); print('present' if 'transcript_path' in d else 'absent')")

# Log only metadata
TIMESTAMP=$(date +%s)
echo "$TIMESTAMP EVENT=$EVENT SESSION=$SESSION CWD=$CWD SOURCE=$SOURCE TRANSCRIPT=$TRANSCRIPT" >> "$LOGFILE"

exit 0
EOF
chmod +x "$WORK_DIR/hook-observer.sh"

# Create settings.json with SYNCHRONOUS hooks (no async: true)
cat > "$CLAUDE_CONFIG_DIR/settings.json" <<EOF
{
  "hooks": {
    "SessionStart": [
      {
        "matcher": "*",
        "hooks": [
          {
            "type": "command",
            "command": "$WORK_DIR/hook-observer.sh"
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
            "command": "$WORK_DIR/hook-observer.sh"
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
            "command": "$WORK_DIR/hook-observer.sh"
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

# Change to disposable working directory (FINDING 4 fix)
cd "$CWD_DIR"

echo "=== Running claude --print with SYNCHRONOUS hooks from disposable cwd ==="
# Capture exit status (CLI output is discarded; never retained in artifacts)
set +e
claude --print "" >/dev/null 2>&1
CLI_EXIT_STATUS=$?
set -e
echo "CLI exit status: $CLI_EXIT_STATUS"
echo ""

# Wait for synchronous hooks to complete
sleep 2

echo "=== Hook events log ==="
if [ -f "$WORK_DIR/hook-events.log" ]; then
    cat "$WORK_DIR/hook-events.log"
else
    echo "No hook events log found"
fi

echo ""
echo "=== Directory structure ==="
find "$WORK_DIR" -type f -name "*.log" 2>/dev/null