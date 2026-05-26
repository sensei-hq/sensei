#!/usr/bin/env bash
# PostToolUse hook: detect intentional-avoidance patterns in written/edited files.
#
# Flags comments that document a deliberate workaround of a shared pattern without
# user approval (e.g. "X does not depend on Y to avoid Z", "keep in sync with Z").
# These decisions must be recorded in backlog.md or an ADR, never in code comments.
#
# Exit 0 always (PostToolUse cannot block), but print a loud warning so Claude
# sees it in the tool feedback and corrects itself before the next action.

set -euo pipefail

INPUT=$(cat)
TOOL=$(echo "$INPUT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('tool_name',''))" 2>/dev/null || true)

# Only scan after Write or Edit
if [[ "$TOOL" != "Write" && "$TOOL" != "Edit" ]]; then
  exit 0
fi

FILE=$(echo "$INPUT" | python3 -c "import sys,json; d=json.load(sys.stdin); print(d.get('tool_input',{}).get('file_path',''))" 2>/dev/null || true)

if [[ -z "$FILE" || ! -f "$FILE" ]]; then
  exit 0
fi

# Only scan source files
EXT="${FILE##*.}"
if [[ "$EXT" != "rs" && "$EXT" != "ts" && "$EXT" != "js" && "$EXT" != "svelte" && "$EXT" != "py" ]]; then
  exit 0
fi

# Patterns that indicate intentional avoidance documented inline in code
PATTERNS=(
  'does not depend on.*to avoid'
  'intentionally (avoid|not using|avoids)'
  'avoid (pulling|dragging|importing|depending on|using)'
  'keep .* in sync'
  'keep in sync with'
  'do not (depend|import|use) .* to avoid'
  'without.*dep(endency|endencies)'
)

PATTERN_ARGS=()
for p in "${PATTERNS[@]}"; do
  PATTERN_ARGS+=(-e "$p")
done

MATCHES=$(grep -niE "${PATTERN_ARGS[@]}" "$FILE" 2>/dev/null || true)

if [[ -n "$MATCHES" ]]; then
  echo ""
  echo "RULE VIOLATION — intentional avoidance documented in code:"
  echo "  File: $FILE"
  echo ""
  echo "$MATCHES" | while IFS= read -r line; do
    echo "  $line"
  done
  echo ""
  echo "Do NOT document architecture decisions as inline code comments."
  echo "If avoidance is approved by the user, record it in docs/backlog.md or an ADR."
  echo "Remove the comment and either use the shared pattern or raise the decision with the user."
fi

exit 0
