# Marketplace — Commands, Skills, Hooks, Plugins

**See also:** [ideas/03](../ideas/03-marketplace.md), [ideas/04](../ideas/04-plugin-system.md)

## Overview

The marketplace is the AI-facing layer of sensei — markdown commands that instruct the agent, hooks that capture events and maintain context, skills that auto-trigger behaviors, and agents that perform specialized roles. It lives in `marketplace/` as a git subtree synced to `sensei-hq/marketplace` via `make marketplace-push`.

The plugin ships as a Claude Code plugin installable via `claude plugin add`. Users do not configure hooks, skills, or commands manually — the plugin handles registration. Per-repo configuration (MCP server binding, repo-specific skills) remains in `sensei init`.

---

## Architecture

A sensei plugin is a directory with a `plugin.json` manifest and four component types:

```
marketplace/plugins/sensei/
  plugin.json               Manifest — declares skills, commands, hooks, agents
  commands/                 Slash commands (markdown)
  skills/                   Auto-triggering skill files (markdown)
  hooks/                    Event-driven scripts
  agents/                   Specialist sub-agents (markdown)
```

`plugin.json` declares the entry points:

```json
{
  "name": "sensei",
  "version": "0.2.2",
  "description": "Codebase intelligence for AI coding agents",
  "skills": ["skills/*.md"],
  "commands": ["commands/*.md"],
  "hooks": { "SessionStart": [{ "type": "command", "command": "bash hooks/session-start.sh" }] }
}
```

The MCP server is not registered in the plugin — it requires `SENSEI_REPO_PATH` which is repo-specific. `sensei init --mcp` writes that entry to the project's MCP configuration.

---

## Commands

Commands are markdown files in `commands/` that instruct the AI agent when invoked as slash commands (e.g., `/sensei:build`). Each command specifies key behavior, MCP tools to call, and expected outputs.

### Phase commands (7)

These map to the development lifecycle. Each calls `update_phase` and `log_event` on the daemon.

| Command | Purpose | Key MCP tools |
|---------|---------|---------------|
| `brainstorm` | Open creative conversation. Routes content to docs/ based on depth. | `update_phase`, `log_event`, `match_pattern` |
| `idea` | Structured brainstorm. Clarifying questions. Output to docs/ideas/. No code. | `update_phase`, `log_event` |
| `analyze` | Read idea + scan codebase. Produce feasibility with 2-3 options. | `update_phase`, `search`, `get_patterns`, `get_project_summary` |
| `blueprint` | Architecture from chosen approach. Components, interfaces, data flow. No code. | `update_phase`, `log_event` |
| `experiment` | Create branch. Build throwaway code. Produce findings doc. | `update_phase`, `log_event` |
| `plan` | Decompose blueprint into features. Create GitHub issues. | `update_phase`, `log_event` |
| `build` | TDD cycle with locate step, decomposition, test approval, pattern enforcement. | `update_phase`, `search`, `get_callers`, `get_patterns`, `match_pattern` |

### Cross-cutting commands (2)

| Command | Purpose | Key MCP tools |
|---------|---------|---------------|
| `review` | Pattern conformance, duplicate detection, quality checks. Auto-triggers after build. | `get_patterns`, `get_pattern_for`, `get_duplicates` |
| `validate` | E2E tests, integration check, doc drift detection, acceptance criteria. | `get_workflow_state` |

### Refocus commands (4)

These re-anchor the agent when context has drifted:

| Command | Purpose | Key MCP tools |
|---------|---------|---------------|
| `rules` | Re-read .sensei/rules.md, output compact summary. | `get_workflow_state` |
| `patterns` | Re-read PATTERNS.md + detected patterns. Show catalog. | `get_patterns`, `get_project_conventions` |
| `refocus` | Re-read state, plan, current task. Flush tangential context. | `get_workflow_state` |
| `tools` | Re-read available MCP tools and preference hierarchy. | `get_workflow_state` |

### Command format

Every command is a markdown file with YAML frontmatter (`name`, `description`) and a structured body. The body defines:
- **When to invoke** — triggering conditions
- **Steps** — numbered sequence the agent must follow
- **MCP tools** — which daemon tools to call and in what order
- **Output expectations** — what files or artifacts the agent should produce

---

## Hooks

Hooks fire on assistant events and post data to the sensei daemon. They are assistant-agnostic — Claude Code is the first integration; the architecture supports Cursor, Zed, Kiro, and others.

### Registered events

| Hook | Event | Action |
|------|-------|--------|
| `session-start` | SessionStart | Load guardrails, inject command awareness, tool preference reminder |
| `user-prompt` | UserPromptSubmit | Classify prompt (correction/continuation/clarification/new), log turn event, detect revision |
| `pre-compact` | PreCompact | Read guardrails + state, inject compact reminder into context |

### Event capture pipeline

Hook scripts are thin wrappers that read the event payload from stdin, enrich it with `assistant_family` and `event_type`, and POST to the daemon on the appropriate port (7744 release, 7745 dev). If the daemon is unreachable, the event falls back to a local JSONL file.

Claude Code registers hooks in `~/.claude/settings.json`. Each event type gets one entry per installed mode (release and dev can coexist). The daemon's `POST /hook/event` handler always returns 200 — hook scripts must never block.

### Captured event types

SessionStart, InstructionsLoaded, UserPromptSubmit, PreToolUse, PostToolUse, Stop, SubagentStart, SubagentStop, Notification, PreCompact, PostCompact.

---

## Skills

Skills are markdown files that teach agents a protocol, technique, or reference. They are not documentation of what was built — they are guidance for how to work.

### Directory structure

```
skills/
  skill-name/
    SKILL.md              Required. Main skill content.
    supporting-file.*     Optional. Heavy reference (100+ lines) or reusable tools.
```

### Frontmatter

Two fields only: `name` and `description`. Max 1024 characters total. `description` starts with "Use when..." and describes triggering conditions — never the workflow or process.

### Token efficiency

Skills load into every conversation. Every token matters.

| Type | Target |
|------|--------|
| Orientation skills (loaded every session) | < 150 words |
| Frequently-used skills | < 300 words |
| Technique/reference skills | < 500 words |

Cross-reference other skills instead of repeating. Put API details in separate files. One excellent code example, not multiple mediocre ones. Use tables for reference material, not prose.

### Static vs generated skills

**Static skills** ship in the plugin — session management, zero-errors policy, indexing, benchmarks, doc drift. **Generated skills** are produced by `sensei init` per-repo — stack-specific patterns, library usage, project-specific architectural guidelines. Generated skills live alongside plugin skills in the user's skill directory.

---

## Agents

Eight specialist agents ship in the plugin:

| Agent | Role |
|-------|------|
| `analyst` | Feasibility analysis and option evaluation |
| `developer` | TDD implementation with pattern enforcement |
| `security-reviewer` | Security audit and vulnerability detection |
| `acceptance-tester` | E2E test design and acceptance criteria |
| `devops-sre` | Infrastructure and deployment concerns |
| `performance-engineer` | Performance analysis and optimization |
| `persona-reviewer` | UX review from a user persona perspective |
| `ux-designer` | Interface design and interaction patterns |

Each agent is a markdown file defining a system prompt, tool access list, and triggering conditions.

---

## Auto-triggering

Skills and commands are triggered based on workflow state, not unsolicited interruptions. The daemon's `get_workflow_state` returns the current phase (brainstorm, idea, analyze, blueprint, experiment, plan, build) and active task. Commands and skills declare their relevant phases in frontmatter — the agent loads only what applies to the current context.

The `session-start` hook injects phase awareness so the agent knows which commands are relevant without loading all of them. Cross-cutting commands (review, validate) trigger automatically after build features complete. Refocus commands are user-initiated — the agent never self-refocuses without being asked.
