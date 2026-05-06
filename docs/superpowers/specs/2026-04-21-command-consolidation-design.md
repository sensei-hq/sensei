# Command Consolidation — Design Spec

**Date:** 2026-04-21
**Status:** Approved
**Scope:** Reduce 28 marketplace commands to 20 by removing redundancy, merging related commands, and improving documentation.

---

## Problem

28 slash commands are too many to remember. Many have narrow scope, several overlap, and some are redundant with MCP tools that auto-detect the same information.

## Decision Summary

| Action | Commands | Rationale |
|--------|----------|-----------|
| **Drop** | `enable`, `disable` | Thin config wrappers with no current use |
| **Drop** | `pattern-extract`, `pattern-use` | MCP `get_patterns()` / `match_pattern()` handles this automatically |
| **Merge into `session`** | `status`, `refocus`, `backlog` | All are "where am I" queries — sub-actions of session management |
| **Merge into `spec`** | `product`, `feature`, `audit` | All invoke the same `sensei:reverse-engineering` skill with different `mode=` |
| **Rename** | `get-api-docs` → `docs` | Shorter, easier to type |
| **Create** | `agent` | List and invoke mindset-based agents |

---

## Final Command Map (20 commands)

### Phase Commands (7) — The workflow pipeline

| Command | Arguments | Description | Example |
|---------|-----------|-------------|---------|
| `idea` | `<topic>` | Explore a concept — ask questions, document problem space. No code. | `/sensei:idea task scheduler` |
| `analyze` | `[topic]` | Assess feasibility — produce 2-3 options with tradeoffs. No code. | `/sensei:analyze` |
| `blueprint` | `[topic]` | Design architecture — components, interfaces, data flow. No code. | `/sensei:blueprint caching layer` |
| `experiment` | `<topic>` | Prototype/spike — build minimal, document findings. Code is discardable. | `/sensei:experiment RxJS for real-time` |
| `plan` | `[topic]` | Decompose blueprint into features with acceptance criteria. Creates GitHub issues. | `/sensei:plan` |
| `build` | `#issue` or `<desc>` | Implement — locate code, TDD, review. The core coding command. | `/sensei:build #42` |
| `validate` | `[#issue]` | Verify — tests pass, acceptance criteria met, no doc drift. | `/sensei:validate` |

### Cross-cutting Commands (4) — Use at any phase

| Command | Arguments | Description | Example |
|---------|-----------|-------------|---------|
| `brainstorm` | `[topic]` | Open creative exploration — routes artifacts by depth. | `/sensei:brainstorm` |
| `review` | `[scope]` | Quality check — patterns, duplicates, test coverage, doc drift. | `/sensei:review modified files` |
| `persona` | `list\|add <name>\|switch <name>\|validate` | Manage project personas for design validation. | `/sensei:persona add end-user` |
| `agent` | `list\|use <name> [task]` | List available agents or invoke one by name. | `/sensei:agent use security-reviewer` |

### Utility Commands (9)

| Command | Arguments | Description | Example |
|---------|-----------|-------------|---------|
| `session` | (none)\|`status`\|`refocus`\|`backlog` | Session management — resume, show state, re-anchor, list open work. | `/sensei:session status` |
| `spec` | `product [path]\|feature <name>\|audit [name]` | Reverse-engineer docs — product overview, feature deep-dive, or security audit. | `/sensei:spec feature auth` |
| `rules` | `[rule to add]` | View, create, or add project rules. | `/sensei:rules use adapter pattern for parsers` |
| `patterns` | `[query]` | Show detected patterns and project conventions. | `/sensei:patterns adapter` |
| `checkpoint` | `[summary]` | Snapshot current progress for interruption recovery. | `/sensei:checkpoint auth flow done` |
| `commit` | `[message]` | Run zero-errors checks, then commit. | `/sensei:commit` |
| `mockup` | `<description>` | Start a UI mockup — enforces framework-native build. | `/sensei:mockup dashboard layout` |
| `docs` | `<lib> [component]` | Fetch library documentation before writing code. | `/sensei:docs sveltekit hooks` |
| `help` | — | Show all commands, agents, and examples. | `/sensei:help` |

---

## New Command: `agent`

**File:** `marketplace/plugins/sensei/commands/agent.md`

**Sub-actions (parsed from first word of $ARGUMENTS):**

- **(no args) or `list`** — List all 8 agents with name, description, model, and when to use.
- **`use <name> [task]`** — Dispatch the named agent as a subagent. If task is provided, pass it. If not, use current session context.

**Available agents:**

| Agent | Mindset | When to use |
|-------|---------|-------------|
| `analyst` | Problem analysis | Before designing — requirements, constraints, scope |
| `developer` | Implementation review | Verify approach before coding |
| `acceptance-tester` | End-to-end testing | After implementation — acceptance criteria, regressions |
| `security-reviewer` | Security audit | User input, auth, data storage, external comms |
| `performance-engineer` | Performance analysis | Data processing, queries, loops, latency |
| `ux-designer` | UX review | Commands, UI components, output formatting |
| `devops-sre` | Ops readiness | Deployment, infra, config, reliability |
| `persona-reviewer` | Persona validation | Validate work against persona goals |

---

## New Command: `spec`

**File:** `marketplace/plugins/sensei/commands/spec.md`

**Sub-actions (parsed from first word of $ARGUMENTS):**

- **`product [root]`** — Reverse-engineer the full product. Generates `openspec/product/` docs.
- **`feature <name>`** — Deep-dive a feature. Generates `openspec/specs/<name>/` docs.
- **`audit [name]`** — OWASP, NFR, and code quality audit.

All delegate to the `sensei:reverse-engineering` skill with `mode=` set accordingly.

---

## Modified Command: `session`

**File:** `marketplace/plugins/sensei/commands/session.md`

**Sub-actions (parsed from first word of $ARGUMENTS):**

- **(no args)** — Resume session: call `get_session_context()`, surface open decisions, report state. (Current behavior.)
- **`status`** — Full orientation: phase, task, issue, rules count, patterns, docs. (Absorbs `status.md`.)
- **`refocus`** — Re-anchor on current task: re-read state, plan, rules, active issue. (Absorbs `refocus.md`.)
- **`backlog`** — List open tasks, decisions, pending questions from session store. (Absorbs `backlog.md`.)

---

## Renamed Command: `docs`

**File:** `marketplace/plugins/sensei/commands/docs.md` (rename from `get-api-docs.md`)

Content is identical — only the filename and description change.

---

## Modified Command: `help`

**File:** `marketplace/plugins/sensei/commands/help.md`

Complete rewrite to include:
- Grouped command tables with descriptions
- Sub-action reference for `session`, `spec`, `persona`, `agent`
- 1-2 usage examples per command
- Agent reference table
- Quick-start section for new users

---

## Documentation Updates

### marketplace/README.md — Comprehensive reference

Rewrite to serve as the full reference doc:
- Command reference with descriptions, args, examples
- Agent reference with descriptions, model, trigger conditions
- Skill reference with trigger conditions
- Typical workflow walkthrough

### Hook updates

| File | Change |
|------|--------|
| `hooks/session-start` line 150-153 | Update `## Workflow Commands` block — remove dropped commands, add new ones, rename `get-api-docs` → `docs` |
| `hooks/pre-compact` line 55 | Change `/sensei:refocus` → `/sensei:session refocus` |

### Design & ideas docs

These are historical documents. Update references to point readers to the current commands:

| File | References to update |
|------|---------------------|
| `docs/README.md` | `/sensei:refocus` → `session refocus`, `/sensei:status` → `session status`, `/sensei:pattern-extract` → removed |
| `docs/features/02-rules-context.md` | `/sensei:refocus` → `session refocus`, `/sensei:status` → `session status` |
| `docs/analysis/01-skill-command-mapping.md` | Update command table: drop enable/disable/pattern-extract/pattern-use/get-api-docs, show final 20-command map |
| `docs/ideas/01-workflow-system.md` | `/sensei:refocus` → `session refocus` |
| `docs/ideas/02-commands.md` | Major update — this is the command design doc. Update to reflect final 20 commands. |
| `docs/ideas/04-cross-cutting.md` | `/sensei:refocus` → `session refocus` |
| `docs/ideas/05-decisions.md` | `/sensei:refocus` → `session refocus`, `/sensei:status` → `session status` |
| `docs/ideas/06-docs-disposition.md` | `/sensei:refocus` → `session refocus` |
| `docs/ideas/14-context-delivery.md` | `/sensei:refocus` → `session refocus` |
| `docs/ideas/15-pattern-store.md` | Remove `/sensei:pattern-extract` references, note MCP auto-detection |
| `docs/ideas/17-pattern-knowledge.md` | Remove `/sensei:pattern-extract` references |
| `docs/blueprints/01-workflow-engine.md` | `/sensei:refocus` → `session refocus`, `/sensei:status` → `session status` |
| `docs/design/02-mcp/workflow-tools.md` | `/sensei:status` → `session status`, `/sensei:refocus` → `session refocus` |
| `docs/design/03-marketplace/plugin-packaging.md` | `/sensei:status` → `session status` |
| `docs/design/03-marketplace/commands.md` | `/sensei:refocus` → `session refocus` |
| `docs/design/roadmap.md` | `/sensei:refocus` → `session refocus`, `/sensei:status` → `session status` |

---

## Catalog & Installer Updates

### `marketplace/catalog.json`

Must be updated to reflect the new 20-command set:
- **Remove entries** for: `enable`, `disable`, `pattern-extract`, `pattern-use`, `status`, `refocus`, `backlog`, `product`, `feature`, `audit`
- **Add entries** for: `agent`, `spec`
- **Rename** `get-api-docs` → `docs`
- **Update** `session` description

### `install_marketplace()` — Stale file cleanup

**Problem:** The fallback installer (`crates/senseid/src/installer.rs:install_marketplace()`) writes commands to `~/.claude/commands/<name>.md` but never removes files from previous versions. On upgrade, dropped commands (like `enable.md`, `status.md`) remain as stale files.

**Fix:** After writing all current catalog items, scan `~/.claude/commands/` and `~/.claude/skills/` for `.md` files whose names don't match any item in the current catalog. Delete those stale files. Log the count as `stale_commands_removed` / `stale_skills_removed` in the install result.

**Note:** This only affects the fallback path (non-plugin install). When `claude plugin install sensei` succeeds, Claude Code manages the command files directly and handles cleanup via its own plugin lifecycle.

---

## Files Summary

### Delete (10 files)
```
marketplace/plugins/sensei/commands/enable.md
marketplace/plugins/sensei/commands/disable.md
marketplace/plugins/sensei/commands/pattern-extract.md
marketplace/plugins/sensei/commands/pattern-use.md
marketplace/plugins/sensei/commands/status.md
marketplace/plugins/sensei/commands/refocus.md
marketplace/plugins/sensei/commands/backlog.md
marketplace/plugins/sensei/commands/product.md
marketplace/plugins/sensei/commands/feature.md
marketplace/plugins/sensei/commands/audit.md
```

### Create (2 files)
```
marketplace/plugins/sensei/commands/agent.md
marketplace/plugins/sensei/commands/spec.md
```

### Rename (1 file)
```
marketplace/plugins/sensei/commands/get-api-docs.md → docs.md
```

### Modify — commands (2 files)
```
marketplace/plugins/sensei/commands/session.md
marketplace/plugins/sensei/commands/help.md
```

### Modify — hooks (2 files)
```
marketplace/plugins/sensei/hooks/session-start
marketplace/plugins/sensei/hooks/pre-compact
```

### Modify — catalog & installer (2 files)
```
marketplace/catalog.json
crates/senseid/src/installer.rs
```

### Modify — docs (16 files)
```
marketplace/README.md
docs/README.md
docs/features/02-rules-context.md
docs/analysis/01-skill-command-mapping.md
docs/ideas/01-workflow-system.md
docs/ideas/02-commands.md
docs/ideas/04-cross-cutting.md
docs/ideas/05-decisions.md
docs/ideas/06-docs-disposition.md
docs/ideas/14-context-delivery.md
docs/ideas/15-pattern-store.md
docs/ideas/17-pattern-knowledge.md
docs/blueprints/01-workflow-engine.md
docs/design/02-mcp/workflow-tools.md
docs/design/03-marketplace/plugin-packaging.md
docs/design/03-marketplace/commands.md
docs/design/roadmap.md
```

### Total: 10 deleted + 2 created + 1 renamed + 22 modified = 35 file operations
