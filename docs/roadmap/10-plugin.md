# Sensei Plugin — Commands, Skills, and Hooks

> **Status:** Shipped (`plugin/`) — install with `sensei plugin install`, upgrade via `sensei init` or `sensei doctor`.

---

## Why a Plugin

Sensei ships as a Claude Code plugin — a directory of markdown files that teach Claude workflows and expose slash commands. This is the right model because:

- **No process boundary** — skills load into Claude's context; no IPC or HTTP
- **Model-agnostic** — the same markdown works with any ACP that supports the plugin format
- **Composable** — commands are thin wrappers that delegate to skills; skills are independent units
- **Auditable** — every behaviour is readable markdown, not compiled code

The CLI (`sensei` binary) handles indexing, MCP, daemon, and setup. The plugin handles Claude's behaviour during sessions.

---

## Plugin Structure

```
plugin/
  .claude-plugin/
    plugin.json          ← name, version, description, author
  commands/              ← slash commands (/session, /commit, etc.)
    *.md
  skills/                ← workflow skills (loaded by trigger, not by command)
    <name>/
      SKILL.md           ← trigger description + workflow instructions
      test-baseline.md   ← expected output before skill (for regression testing)
      test-pressure-1.md ← edge case 1
      test-pressure-2.md ← edge case 2
  hooks/                 ← session lifecycle hooks
    hooks.json
    session-start
  scripts/
    install.ts           ← post-install hook
    uninstall.ts         ← pre-uninstall hook
```

---

## Commands vs Skills

| | Commands | Skills |
|---|---|---|
| **Invoked** | Explicitly, via `/command` | Automatically, when trigger condition is met |
| **Syntax** | `/session`, `/commit auth` | No invocation — Claude applies them |
| **Scope** | One-shot action | Persistent behaviour shaping |
| **Location** | `commands/*.md` | `skills/<name>/SKILL.md` |
| **Can call skills** | Yes — via `Skill tool` | N/A |

**Rule of thumb:** if the user has to ask for it, it's a command. If Claude should always do it when a certain situation arises, it's a skill.

---

## Command Catalog

| Command | Description |
|---------|-------------|
| `/session` | Resume session — calls `get_session_context()`, surfaces interrupted work |
| `/checkpoint` | Snapshot progress for interruption recovery — calls `take_snapshot()` |
| `/backlog` | List open tasks, decisions, and pending questions from the session store |
| `/commit` | Run zero-errors check (`bun run --filter '*' test && bunx tsc --noEmit`) then commit |
| `/mockup` | Build a mockup — enforces commit-first, framework-native, no standalone HTML |
| `/pattern-extract` | Extract a recurring structure from code and write it to `PATTERNS.md` |
| `/pattern-use` | Look up a pattern by name and apply it to the current task |
| `/feature` | Deep-dive a feature → generates `openspec/specs/<capability>/` via `reverse-engineering` skill |
| `/product` | Reverse-engineer the full product → generates `openspec/product/` via `reverse-engineering` skill |
| `/audit` | Audit a capability for OWASP, NFR, and code quality → via `reverse-engineering` skill |
| `/enable` | Enable an opt-in skill for this project (writes to project sensei config) |
| `/disable` | Disable an opt-in skill for this project |
| `/help` | Show all commands and skills |

Commands that drive the reverse-engineering workflow (`/feature`, `/product`, `/audit`) delegate to the `sensei:reverse-engineering` skill via the Skill tool — the workflow spec lives in the skill, not duplicated across commands.

---

## Skill Catalog

Skills are always-on: Claude applies them automatically when the described trigger condition is met. They are not invoked manually.

### Core Session Skills

| Skill | Trigger |
|-------|---------|
| `session-management` | Start of every session with sensei MCP available — calls `get_session_context()` before anything else |
| `context-efficiency` | Before loading any code — calls `recommend_next(task)` to get minimal file scope |
| `working-smarter` | Designing UI, building features, or starting/completing any implementation task |

### Code Intelligence Skills

| Skill | Trigger |
|-------|---------|
| `codebase-indexing` | First working on a repo, after major refactor, or when `llmspec.yaml` has TODOs |
| `decomposing-broad-tasks` | Request touches 5+ files or uses "all", "every", "refactor all" |
| `design` | Before implementing a new feature — maps existing patterns, picks approach |
| `identifying-patterns` | Before implementing anything that might follow a recurring structure |
| `pattern-based-development` | Before implementing any new feature/component — checks `PATTERNS.md` first |
| `refactor` | When improving code structure without changing behaviour |
| `context-efficiency` | Before loading code — gets minimal file scope from `recommend_next()` |
| `analyze` | Starting work on an unfamiliar repo, or after significant changes |

### Documentation Skills

| Skill | Trigger |
|-------|---------|
| `guiding-doc-creation` | Before creating or updating any doc — enforces naming and frontmatter conventions |
| `detecting-doc-drift` | Docs may be out of sync with code, or before committing API changes |
| `reformatting-docs` | A doc's structure diverges from the canonical template |
| `extract-docs` | Adding doc coverage to untested or under-documented code |

### Quality & Reverse-Engineering Skills

| Skill | Trigger |
|-------|---------|
| `reverse-engineering` | Invoked by `/product`, `/feature`, `/audit` commands — full OWASP + NFR + DB analysis workflow |
| `running-benchmarks` | Evaluating whether a new skill or workflow change reduces token usage |
| `auditing-skill-descriptions` | Adding new skills or reviewing the skill library for trigger quality |
| `test-gen` | Adding test coverage to untested or under-tested code |

### Library Intelligence Skills

| Skill | Trigger |
|-------|---------|
| `identify-unknown-libs` | `get_lib_docs` returns empty sections for a library you're about to use |

### UI Skills

| Skill | Trigger |
|-------|---------|
| `building-app-mockups` | Invoked by `/mockup` command — framework-native mockups at real app routes |

---

## Hooks

| Hook | File | Purpose |
|------|------|---------|
| `SessionStart` | `hooks/session-start` | Injects a reminder to call `get_session_context()` as first tool |

The `session-start` hook outputs text that becomes a `<system-reminder>` before the first user message. Since MCP tools can't be called from a shell hook, it works by instructing Claude to call the tool when it sees the reminder.

---

## Installation

```bash
# Install from the sensei repo (dev / first time)
sensei plugin install

# The plugin is re-applied automatically during:
sensei init       # on first setup and on re-init
sensei doctor     # detects and fixes outdated plugin installs
```

`sensei plugin install` copies `plugin/` to `~/.claude/plugins/marketplaces/local/plugins/sensei/` and registers it in `installed_plugins.json`. Restart Claude Code after installing.

---

## Versioning

The plugin version lives in `plugin/.claude-plugin/plugin.json`. Bump it when adding or significantly changing skills or commands:

```json
{
  "name": "sensei",
  "version": "1.1.0",
  "description": "..."
}
```

`sensei doctor` compares the installed version against the current repo version and prompts to upgrade.

---

## Adding a New Skill

1. Create `plugin/skills/<name>/SKILL.md` with frontmatter:
   ```markdown
   ---
   name: <name>
   description: Use when <precise trigger condition> — <one sentence on what it does>.
   ---
   ```
2. Write the workflow body — structured, specific, no vague instructions
3. Add 3 test files: `test-baseline.md`, `test-pressure-1.md`, `test-pressure-2.md`
4. Run `/auditing-skill-descriptions` to score and improve the trigger description
5. Add to `/help` command's auto-applied skills table
6. Bump plugin version, run `sensei plugin install`

## Adding a New Command

1. Create `plugin/commands/<name>.md` with frontmatter:
   ```markdown
   ---
   description: One sentence on what this command does
   argument-hint: What $ARGUMENTS expects (optional)
   ---
   ```
2. Write the command body — reference skills via `Skill tool` where applicable
3. Add to `/help` command's slash commands table
4. Bump plugin version, run `sensei plugin install`

---

## Future

| Area | Plan |
|------|------|
| **Plugin marketplace** | Publish to Claude Code's plugin registry when available |
| **Per-ACP skill sets** | Skills with ACP-specific variants (Cursor, opencode, Kiro) |
| **Skill TDD** | Automated pressure tests for all skills to catch regressions |
| **Version check in doctor** | `sensei doctor` detects stale plugin install and auto-upgrades |
