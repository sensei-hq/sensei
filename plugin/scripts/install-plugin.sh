#!/usr/bin/env bash
set -euo pipefail

# Sensei Plugin Installer
# Installs sensei MCP server, hooks, and configures Claude Code

PLUGIN_NAME="sensei"
CLAUDE_DIR="${HOME}/.claude"
PLUGIN_DIR="${CLAUDE_DIR}/plugins/${PLUGIN_NAME}"
HOOKS_FILE="${CLAUDE_DIR}/hooks.json"
CLAUDE_CONFIG="${CLAUDE_DIR}/.claude.json"
SENSEI_DIR="${HOME}/.sensei"

echo "Installing sensei plugin for Claude Code..."

# ── 1. Find or build sensei-mcp binary ────────────────────────────────────────

SCRIPT_DIR="$(cd "$(dirname "$0")/.." && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"

# Check if release binary exists
MCP_BIN="${REPO_ROOT}/crates/sensei-mcp/target/release/sensei-mcp"
DAEMON_BIN="${REPO_ROOT}/crates/senseid/target/release/senseid"

if [ ! -f "$MCP_BIN" ]; then
  echo "Building sensei-mcp..."
  cd "${REPO_ROOT}/crates/sensei-mcp" && cargo build --release
fi

if [ ! -f "$DAEMON_BIN" ]; then
  echo "Building senseid..."
  cd "${REPO_ROOT}/crates/senseid" && cargo build --release
fi

# ── 2. Install plugin files ───────────────────────────────────────────────────

mkdir -p "${PLUGIN_DIR}/bin"
mkdir -p "${PLUGIN_DIR}/hooks"

# Copy binaries
cp "$MCP_BIN" "${PLUGIN_DIR}/bin/sensei-mcp"
cp "$DAEMON_BIN" "${PLUGIN_DIR}/bin/senseid"

# Copy hooks
cp "${SCRIPT_DIR}/hooks/session-start" "${PLUGIN_DIR}/hooks/"
cp "${SCRIPT_DIR}/hooks/pre-tool" "${PLUGIN_DIR}/hooks/"
cp "${SCRIPT_DIR}/hooks/post-tool" "${PLUGIN_DIR}/hooks/"
cp "${SCRIPT_DIR}/hooks/run-hook.cmd" "${PLUGIN_DIR}/hooks/"
chmod +x "${PLUGIN_DIR}/hooks/"*
chmod +x "${PLUGIN_DIR}/bin/"*

echo "  Installed to ${PLUGIN_DIR}"

# ── 3. Configure MCP server ──────────────────────────────────────────────────

# Update ~/.claude.json to add sensei MCP server
if [ -f "${HOME}/.claude.json" ]; then
  python3 -c "
import json
with open('${HOME}/.claude.json', 'r') as f:
    config = json.load(f)
config.setdefault('mcpServers', {})
config['mcpServers']['sensei'] = {
    'command': '${PLUGIN_DIR}/bin/sensei-mcp',
    'args': []
}
with open('${HOME}/.claude.json', 'w') as f:
    json.dump(config, f, indent=2)
print('  Configured MCP server in ~/.claude.json')
"
fi

# ── 4. Configure hooks ───────────────────────────────────────────────────────

python3 -c "
import json, os

hooks_file = '${HOOKS_FILE}'
plugin_dir = '${PLUGIN_DIR}'

# Load existing or create new
if os.path.exists(hooks_file):
    with open(hooks_file) as f:
        config = json.load(f)
else:
    config = {'hooks': {}}

hooks = config.setdefault('hooks', {})

# SessionStart
hooks['SessionStart'] = [{
    'matcher': 'startup|resume|clear|compact',
    'hooks': [{
        'type': 'command',
        'command': f'{plugin_dir}/hooks/run-hook.cmd session-start'
    }]
}]

# PreToolExecution
hooks['PreToolExecution'] = [{
    'matcher': '',
    'hooks': [{
        'type': 'command',
        'command': f'{plugin_dir}/hooks/run-hook.cmd pre-tool'
    }]
}]

# PostToolExecution
hooks['PostToolExecution'] = [{
    'matcher': '',
    'hooks': [{
        'type': 'command',
        'command': f'{plugin_dir}/hooks/run-hook.cmd post-tool'
    }]
}]

with open(hooks_file, 'w') as f:
    json.dump(config, f, indent=2)

print('  Configured hooks in ~/.claude/hooks.json')
"

# ── 5. Ensure daemon data directory exists ────────────────────────────────────

mkdir -p "${SENSEI_DIR}"

echo ""
echo "Sensei plugin installed successfully!"
echo ""
echo "  MCP server: ${PLUGIN_DIR}/bin/sensei-mcp"
echo "  Daemon:     ${PLUGIN_DIR}/bin/senseid"
echo "  Hooks:      ${PLUGIN_DIR}/hooks/"
echo ""
echo "Start the daemon:"
echo "  ${PLUGIN_DIR}/bin/senseid start"
echo ""
echo "Or add to your shell profile:"
echo "  alias senseid='${PLUGIN_DIR}/bin/senseid'"
