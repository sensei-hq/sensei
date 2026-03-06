#!/usr/bin/env bash
set -e

REPO_ROOT="$(cd "$(dirname "$0")" && pwd)"
SKILLS_DIR="$REPO_ROOT/skills"
SENSEI_DIR="$REPO_ROOT/packages/sensei"
CLAUDE_SKILLS_DIR="$HOME/.claude/skills"
CLAUDE_MCP_CONFIG="$HOME/.claude/mcp.json"

usage() {
  echo "Usage: ./install.sh [--claude] [--all] [--uninstall]"
  echo ""
  echo "  --claude     Install skills for Claude Code + register MCP server"
  echo "  --all        Install for all supported agents"
  echo "  --uninstall  Remove installed skills and MCP registration"
  exit 1
}

install_skills() {
  local target="$1"
  echo "Installing skills to $target..."
  mkdir -p "$target"
  for skill_dir in "$SKILLS_DIR"/*/; do
    skill_name=$(basename "$skill_dir")
    link_path="$target/$skill_name"
    if [ -L "$link_path" ]; then
      rm "$link_path"
    fi
    ln -s "$skill_dir" "$link_path"
    echo "  Linked: $skill_name"
  done
}

install_mcp() {
  echo "Building sensei MCP server..."
  cd "$REPO_ROOT"
  bun install --silent
  bun run build
  cd - > /dev/null

  local dist="$SENSEI_DIR/dist/index.js"

  echo "Registering MCP server in $CLAUDE_MCP_CONFIG..."
  mkdir -p "$(dirname "$CLAUDE_MCP_CONFIG")"

  if [ ! -f "$CLAUDE_MCP_CONFIG" ]; then
    echo '{"mcpServers":{}}' > "$CLAUDE_MCP_CONFIG"
  fi

  node -e "
    const fs = require('fs');
    const config = JSON.parse(fs.readFileSync('$CLAUDE_MCP_CONFIG', 'utf-8'));
    config.mcpServers = config.mcpServers ?? {};
    config.mcpServers['sensei'] = {
      command: 'node',
      args: ['$dist'],
      env: { REPO_PATH: process.cwd() }
    };
    fs.writeFileSync('$CLAUDE_MCP_CONFIG', JSON.stringify(config, null, 2));
  "
  echo "  MCP server registered as 'sensei'."
}

uninstall() {
  echo "Uninstalling..."
  for skill_dir in "$SKILLS_DIR"/*/; do
    skill_name=$(basename "$skill_dir")
    rm -f "$CLAUDE_SKILLS_DIR/$skill_name"
    echo "  Removed: $skill_name"
  done
  echo "  Skills removed."
}

if [ $# -eq 0 ]; then usage; fi

case "$1" in
  --claude|--all)
    install_skills "$CLAUDE_SKILLS_DIR"
    install_mcp
    echo ""
    echo "Done. Restart Claude Code to pick up new skills and MCP server."
    ;;
  --uninstall)
    uninstall
    ;;
  *)
    usage
    ;;
esac
