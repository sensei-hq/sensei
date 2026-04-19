#!/usr/bin/env bash
# Install/upgrade the sensei plugin for Claude Code.
# Wires: hooks into .claude/settings.local.json, MCP server into .mcp.json.
# Re-run after any plugin update to pick up changes.
#
# Usage: ./scripts/install-plugin.sh

set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PLUGIN_ROOT="$REPO_ROOT/marketplace"
CLAUDE_DIR="$REPO_ROOT/.claude"
SETTINGS_FILE="$CLAUDE_DIR/settings.local.json"
MCP_FILE="$REPO_ROOT/.mcp.json"

echo "=== sensei plugin install ==="
echo "Repo:   $REPO_ROOT"
echo "Plugin: $PLUGIN_ROOT"
echo ""

# ── 1. Ensure .claude directory exists ───────────────────────────────────────
mkdir -p "$CLAUDE_DIR"

# ── 2. Write .mcp.json (MCP server registration) ────────────────────────────
cat > "$MCP_FILE" <<'MCPEOF'
{
  "mcpServers": {
    "sensei": {
      "command": "senseid",
      "args": ["--mcp"]
    }
  }
}
MCPEOF
echo "[ok] .mcp.json — MCP server registered"

# ── 2b. Copy mindsets into .sensei/mindsets/ ─────────────────────────────────
SENSEI_DIR="$REPO_ROOT/.sensei"
mkdir -p "$SENSEI_DIR/mindsets"
if [ -d "$PLUGIN_ROOT/mindsets" ]; then
  cp "$PLUGIN_ROOT/mindsets/"*.md "$SENSEI_DIR/mindsets/" 2>/dev/null
  MCOUNT=$(ls "$SENSEI_DIR/mindsets/"*.md 2>/dev/null | wc -l | tr -d ' ')
  echo "[ok] .sensei/mindsets/ — $MCOUNT mindsets copied from plugin"
elif [ -f "$PLUGIN_ROOT/templates/mindsets.md" ]; then
  cp "$PLUGIN_ROOT/templates/mindsets.md" "$SENSEI_DIR/mindsets/all.md"
  echo "[ok] .sensei/mindsets/ — single file copied from plugin/templates"
fi

# ── 3. Write settings.local.json with hooks ──────────────────────────────────
# Preserve existing permissions if present
PERMISSIONS='{}'
if [ -f "$SETTINGS_FILE" ]; then
  PERMISSIONS=$(python3 -c "
import json, sys
with open('$SETTINGS_FILE') as f:
    d = json.load(f)
print(json.dumps(d.get('permissions', {})))
" 2>/dev/null || echo '{}')
fi

# Build the hooks section pointing to the plugin
cat > "$SETTINGS_FILE" <<SETTINGSEOF
{
  "permissions": $PERMISSIONS,
  "hooks": {
    "SessionStart": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "$PLUGIN_ROOT/hooks/run-hook.cmd session-start"
          }
        ]
      }
    ],
    "PreCompact": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "$PLUGIN_ROOT/hooks/run-hook.cmd pre-compact"
          }
        ]
      }
    ],
    "UserPromptSubmit": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "$PLUGIN_ROOT/hooks/run-hook.cmd user-prompt"
          }
        ]
      }
    ]
  }
}
SETTINGSEOF
echo "[ok] settings.local.json — hooks wired (SessionStart, PreCompact, UserPromptSubmit)"

# ── 4. Verify gate check files exist ────────────────────────────────────────
echo ""
echo "=== gate check ==="

PASS=true

if [ -d "$SENSEI_DIR/mindsets" ]; then
  MINDSET_COUNT=$(ls "$SENSEI_DIR/mindsets/"*.md 2>/dev/null | wc -l | tr -d ' ')
  echo "[ok] mindsets/ — $MINDSET_COUNT mindsets"
else
  echo "[FAIL] .sensei/mindsets/ — NOT FOUND"
  PASS=false
fi

if [ -f "$REPO_ROOT/.sensei/rules.md" ]; then
  echo "[ok] rules.md — project rules loaded"
else
  echo "[WARN] .sensei/rules.md — not found (create with /sensei:rules)"
fi

if [ -d "$REPO_ROOT/.sensei/personas" ]; then
  PERSONA_COUNT=$(ls "$REPO_ROOT/.sensei/personas/"*.md 2>/dev/null | wc -l | tr -d ' ')
  echo "[ok] personas/ — $PERSONA_COUNT personas defined"
else
  echo "[info] .sensei/personas/ — not found (optional, create with /sensei:persona add)"
fi

# Verify every command on disk is in catalog
CMDS_ON_DISK=$(ls "$PLUGIN_ROOT/commands/"*.md 2>/dev/null | xargs -I{} basename {} .md | sort)
CMDS_IN_CATALOG=$(python3 -c "import json; items=json.load(open('$PLUGIN_ROOT/catalog.json'))['items']; print('\n'.join(sorted(i['name'] for i in items if i['kind']=='command')))" 2>/dev/null)
MISSING_CMDS=$(comm -23 <(echo "$CMDS_ON_DISK") <(echo "$CMDS_IN_CATALOG"))
if [ -n "$MISSING_CMDS" ]; then
  echo "[FAIL] commands on disk but NOT in catalog.json:"
  echo "$MISSING_CMDS" | sed 's/^/       /'
  PASS=false
else
  CMD_COUNT=$(echo "$CMDS_ON_DISK" | wc -l | tr -d ' ')
  echo "[ok] catalog — all $CMD_COUNT commands registered"
fi

if [ -f "$REPO_ROOT/CLAUDE.md" ]; then
  if grep -q "Gate Check" "$REPO_ROOT/CLAUDE.md"; then
    echo "[ok] CLAUDE.md — gate check reference present"
  else
    echo "[WARN] CLAUDE.md — missing gate check section"
  fi
else
  echo "[WARN] CLAUDE.md — not found"
fi

if command -v senseid &>/dev/null; then
  echo "[ok] senseid — binary found on PATH"
else
  echo "[WARN] senseid — not on PATH (run: cargo build --release && ./scripts/link.sh)"
fi

echo ""
if [ "$PASS" = true ]; then
  echo "=== install complete ==="
  echo "Start a new Claude Code session to pick up the changes."
  echo "Re-run this script after any plugin update."
else
  echo "=== install incomplete — fix FAIL items above ==="
  exit 1
fi
