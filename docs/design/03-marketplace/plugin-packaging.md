---
id: plugin-packaging
type: design
status: active
---

# Sensei as a Claude Code Plugin

> How to package sensei's Claude integration as a first-class plugin — distributable, installable, and composable with the existing `sensei init` workflow.

---

## Background

Claude Code plugins are packages that bundle skills, hooks, agents, slash commands, and MCP server registrations into a single installable unit. Users install them with `claude plugin add <name>`.

Sensei already has all the ingredients:

| Sensei component | Plugin primitive |
|---|---|
| Skills in `.claude/skills/` | `skills/` directory |
| Pre/PostToolUse hook scripts | Plugin hooks |
| MCP server (`packages/server`) | `.mcp.json` |
| CLI commands like `reverse-engineer` | `commands/` (slash commands) |
| Session protocol, zero-errors, etc. | Static skills |

Packaging as a plugin means:
- Users don't need to manually configure hooks, settings, or mcp.json
- The Claude integration can be updated independently of the repo setup
- Skills ship with the tool, not generated per-repo at init time

---

## What Stays Outside the Plugin

The plugin handles the **Claude integration layer**. These remain CLI responsibilities:

| Concern | Why it stays in CLI |
|---|---|
| `sensei init` (Supabase setup, repo indexing) | Requires interactive setup, writes `.sensei/config.yaml` |
| Collector daemon + launchd | System-level process management |
| Repo-specific skill generation (`install_skills`) | Depends on indexed codebase |
| `SENSEI_REPO_PATH` in mcp.json | Per-repo config, written by `sensei init --mcp` |

The plugin is the **static, shared** layer. `sensei init` is the **per-repo, dynamic** layer.

---

## Plugin Structure

```
sensei-plugin/
  plugin.json                          ← manifest
  skills/
    managing-project-sessions.md       ← static session protocol skill
    zero-errors-policy.md              ← zero errors enforcement
    indexing-codebase.md               ← how to trigger reindex
    running-benchmarks.md              ← benchmark workflow
    detecting-doc-drift.md             ← doc drift detection
  commands/
    reverse-engineer.md                ← /sensei:reverse-engineer
    sensei-status.md                   ← /sensei:status (shows index age, session info)
  hooks/
    session-start.sh                   ← injects get_session_context reminder
  agents/
    sensei-indexer.md                  ← background agent for watch/reindex
```

`plugin.json`:
```json
{
  "name": "sensei",
  "version": "1.0.0",
  "description": "Codebase intelligence for AI coding agents",
  "skills": ["skills/*.md"],
  "commands": ["commands/*.md"],
  "hooks": {
    "SessionStart": [{ "type": "command", "command": "bash hooks/session-start.sh" }]
  }
}
```

Note: the MCP server is **not** registered in the plugin — it requires `SENSEI_REPO_PATH` which is repo-specific. `sensei init --mcp` handles that separately and writes to `~/.claude/mcp.json`.

---

## Skill Split: Static vs Generated

Not all skills should be in the plugin. Some are generic; others are codebase-specific.

**Static skills (ship in plugin):**
- `managing-project-sessions` — call `get_session_context`, `take_snapshot`, `checkpoint`
- `zero-errors-policy` — run tests + tsc before and after, fix all errors
- `indexing-codebase` — when and how to trigger reindex
- `running-benchmarks` — benchmark workflow
- `detecting-doc-drift` — doc drift workflow

**Generated skills (produced by `install_skills`, repo-specific):**
- Stack-specific implementation patterns (e.g. "how tests are structured in this repo")
- Library usage patterns for registered custom_libs
- Project-specific architectural guidelines

Generated skills live in `~/.claude/skills/` (created by `sensei init --agent` or `install_skills` MCP tool). They coexist with the plugin skills.

---

## SessionStart Hook in the Plugin

The current `SessionStart` hook in `.claude/settings.local.json` injects the `get_session_context` reminder. In the plugin, this becomes a `SessionStart` hook entry in `plugin.json`.

The hook script checks whether the sensei MCP server is registered before injecting the instruction — no point instructing the agent to call a tool that isn't available:

```bash
#!/bin/bash
# Only inject if sensei MCP server is configured
if grep -q '"sensei"' ~/.claude/mcp.json 2>/dev/null; then
  echo 'SESSION PROTOCOL REQUIRED: Call get_session_context(task_description="session startup") as your FIRST tool call before responding.'
fi
```

This makes the plugin safe to install globally — it silently no-ops on repos where `sensei init` hasn't been run.

---

## Slash Commands

Two commands make sense as plugin slash commands:

### `/sensei:reverse-engineer`

Given a file or URL, reverse engineer it into a structured spec. Already documented in `docs/reverse-engineer.md`. Move to `commands/reverse-engineer.md` with `$ARGUMENTS` for the target.

### `/sensei:status`

Quick health check:
- Is the MCP server running? (checks if `get_session_context` tool is available)
- When was the repo last indexed?
- How many symbols are indexed?
- Is the collector daemon running?
- Any OTLP events in the last hour?

Implemented as a prompt that calls `get_session_context` and formats the result.

---

## Distribution

Two options:

**Option A: npm package + Claude Code marketplace**
```
npm publish sensei-claude-plugin
claude plugin add sensei-claude-plugin
```
Requires sensei to be on the Claude Code plugin marketplace.

**Option B: Git URL install**
```
claude plugin add https://github.com/your-org/sensei --source git
```
Available today, no marketplace required.

**Recommended:** Start with Option B (git install). Register on marketplace once stable.

---

## Migration from Current Setup

Users who ran `sensei init` before the plugin existed have hooks + skills installed manually. Migration:

1. `claude plugin add sensei` — installs plugin (skills + commands + SessionStart hook)
2. `sensei doctor` detects duplicate hooks and removes the manually-installed ones from `~/.claude/settings.json`
3. The MCP server entry in `~/.claude/mcp.json` stays (written by `sensei init --mcp`, not by the plugin)

---

## What This Enables

| Feature | Before plugin | After plugin |
|---|---|---|
| Install Claude integration | Manual: run `sensei init`, configure hooks, copy skills | `claude plugin add sensei` |
| Update skills | Re-run `sensei init --agent` or copy files | `claude plugin update sensei` |
| Zero-errors skill | Manually ensure it's in `.claude/skills/` | Ships in plugin |
| SessionStart reminder | Per-project `.claude/settings.local.json` | Plugin hook (global, repo-aware) |
| Slash commands | Not available | `/sensei:reverse-engineer`, `/sensei:status` |
