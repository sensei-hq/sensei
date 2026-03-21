---
name: Sensei Claude Plugin
description: Design spec for the sensei plugin — packages cross-project and project-specific skills into a properly registered Claude Code plugin so they are auto-discovered at session start.
date: 2026-03-20
status: approved
---

# Sensei Plugin Design

## Problem

Standalone skills in `~/.claude/skills/` are not surfaced to Claude at session start. Only skills inside registered plugins (in `~/.claude/plugins/`) are auto-discovered. This means skills like `working-smarter`, `building-app-mockups`, and `zero-errors-policy` never fire — Claude re-offers HTML mockups, skips error checks, and doesn't know about framework-first mockup discipline.

## Solution

Package all skills into a single `sensei` plugin that lives in this repo and installs into the Claude plugin system. Globally useful skills are always-on defaults. Project-specific skills are opt-in via a per-project config file.

---

## Architecture

**Approach:** Plugin lives at `plugin/` in the sensei repo. A `bun run plugin:install` script copies it into `~/.claude/plugins/repos/sensei/` and registers it in `installed_plugins.json`. Idempotent — re-run after editing skills to push updates.

```
plugin/
  .claude-plugin/
    plugin.json                      ← manifest
  skills/
    working-smarter/                 ← default (always-on)
    context-efficiency/              ← default
    decomposing-broad-tasks/         ← default
    pattern-based-development/       ← default
    session-management/              ← opt-in
    codebase-indexing/               ← opt-in
    identifying-patterns/            ← opt-in
    guiding-doc-creation/            ← opt-in
    running-benchmarks/              ← opt-in
    auditing-skill-descriptions/     ← opt-in
  commands/
    mockup.md
    session.md
    checkpoint.md
    backlog.md
    commit.md
    pattern-extract.md
    pattern-use.md
    product.md
    feature.md
    audit.md
    enable.md
    disable.md
  hooks/
    hooks.json                       ← SessionStart hook
    session-start                    ← reads sensei.config.json, emits opt-in skill list
  scripts/
    install.ts
    uninstall.ts
```

---

## Skills

### Default Skills (all projects, always-on)

| Skill | Trigger | Merges |
|-------|---------|--------|
| `working-smarter` | Designing UI, starting new features, impl checkpoints | `working-smarter` + `building-app-mockups` + `zero-errors-policy` |
| `context-efficiency` | Before loading code into session | `managing-context` + `compressing-content` |
| `decomposing-broad-tasks` | "refactor all", "update all", 5+ file tasks | unchanged |
| `pattern-based-development` | Before implementing any feature or component | moved from opt-in — required everywhere |

**`working-smarter` coverage:**
- Commit all work before starting a new feature (no exceptions)
- Build mockups/UI directly in the target framework — never standalone HTML
- Zero-errors policy: run full test + tsc check before coding AND after coding

### Opt-in Skills (activated per project)

| Skill | Covers | Merges |
|-------|--------|--------|
| `session-management` | `get_session_context()`, `take_snapshot()`, `recommend_next()` | `managing-project-sessions` + `running-agentic-sessions` |
| `codebase-indexing` | Run sensei indexer, populate llmspec.yaml | `indexing-codebase` + `populating-llmspec` |
| `identifying-patterns` | Discover + document recurring structures | unchanged |
| `guiding-doc-creation` | Doc naming, frontmatter, traceability | unchanged |
| `running-benchmarks` | A/B skill evaluation, token/turn metrics | unchanged |
| `auditing-skill-descriptions` | Skill library quality review | unchanged |

---

## Commands

| Command | Description |
|---------|-------------|
| `/mockup` | Triggers working-smarter mockup flow — commits first, builds in framework |
| `/session` | Calls `get_session_context()`, loads interrupted work and open decisions |
| `/checkpoint` | Calls `take_snapshot()`, records current progress for interruption recovery |
| `/backlog` | Lists open tasks and decisions from session store |
| `/commit` | Runs zero-errors checks (test + tsc), then commits clean |
| `/pattern-extract` | Extracts a reusable pattern from existing code → writes to PATTERNS.md |
| `/pattern-use [name]` | Looks up named pattern → applies it to current task |
| `/product` | Reverse-engineers full product → `openspec/product/` docs (from reverse-engineer.md `mode=product`) |
| `/feature [capability]` | Deep-dives a feature → `openspec/specs/<capability>/` (from `mode=feature`) |
| `/audit [capability]` | Audits a capability: OWASP, NFR, code quality (from `mode=audit`) |
| `/enable [skill]` | Adds a skill to `~/.claude/projects/<slug>/sensei.config.json` |
| `/disable [skill]` | Removes a skill from the project config |

---

## Opt-in Mechanism

**Per-project config:** `~/.claude/projects/<slug>/sensei.config.json`

```json
{
  "skills": ["session-management", "codebase-indexing", "identifying-patterns"]
}
```

**SessionStart hook** reads this file. If it exists, injects a system reminder listing active opt-in skills. If absent, only default skills fire. The `<slug>` matches the Claude Code project identifier derived from the repo path.

**`/enable` and `/disable`** commands edit this file directly — no manual file hunting required.

---

## Installation

```bash
bun run plugin:install   # copy plugin/ → ~/.claude/plugins/repos/sensei/ + register
bun run plugin:uninstall # deregister and remove
```

The install script:
1. Copies `plugin/` to `~/.claude/plugins/repos/sensei/`
2. Upserts entry in `~/.claude/plugins/installed_plugins.json` at user scope
3. Prints confirmation with skill count and command list

Re-running after editing skills pushes updates live immediately (no Claude restart needed for skills; commands take effect on next session).

---

## Source Mapping

Skills in `plugin/skills/` are the canonical source — they replace the scattered files in:
- `~/.claude/skills/` (working-smarter.md, orientation.md, workflow.md, etc.)
- `/sensei/skills/` (all project-specific SKILL.md files)

After the plugin is installed and verified, the originals in those locations can be removed to avoid confusion.
