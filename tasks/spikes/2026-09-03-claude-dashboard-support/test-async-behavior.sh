#!/bin/bash
set -e

# Create isolated temporary directories
BASE_DIR="/private/var/folders/7v/0z9xp1957554w5lkb4k0vk3m0000gn/T/opencode"
WORK_DIR=$(mktemp -d "$BASE_DIR/claude-async-test-XXXXXX")
HOME_DIR="$WORK_DIR/home"
CLAUDE_CONFIG_DIR="$WORK_DIR/.claude"
CWD_DIR="$WORK_DIR/project"
mkdir -p "$HOME_DIR" "$CLAUDE_CONFIG_DIR" "$CWD_DIR"

# Create a hook script that logs event names with timestamps (metadata only)
cat > "$WORK_DIR/hook-observer.sh" << 'EOF'
#!/bin/bash
# Async hook observer - metadata-only event log (field presence only)
LOGFILE="$(dirname "$0")/hook-events.log"
INPUT=$(cat)

# Extract event name only
EVENT=$(echo "$INPUT" | python3 -c "import sys, json; d=json.load(sys.stdin); print(d.get('hook_event_name', 'unknown'))")

# Log with timestamp
TIMESTAMP=$(date +%s.%N)
echo "$TIMESTAMP EVENT=$EVENT" >> "$LOGFILE"

exit 0
EOF
chmod +x "$WORK_DIR/hook-observer.sh"

# Create settings.json with ASYNC hooks
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

echo "=== Hook events log ==="
if [ -f "$WORK_DIR/hook-events.log" ]; then
    cat "$WORK_DIR/hook-events.log"
else
    echo "No hook events log found"
fi