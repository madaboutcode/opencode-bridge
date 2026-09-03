#!/bin/bash
# Metadata-only observer: records only field presence, never values
# This script is invoked by Claude CLI hooks.
# It receives JSON on stdin describing the event.
# It should NOT read or store prompt text, assistant text, tool I/O, etc.

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