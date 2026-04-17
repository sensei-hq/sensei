---
name: Workflow Engine Architecture
description: How commands, phases, templates, guardrails, hooks, and configuration interact as a system
date: 2026-04-17
status: blueprint
origin: docs/ideas/01-workflow-system.md
analysis: docs/analysis/01-skill-command-mapping.md
---

# Workflow Engine Architecture

## Overview

The workflow engine is not a runtime — it's a set of **markdown commands, templates, a guardrails file, and hooks** that shape how the AI assistant behaves. There is no new code to run. The "engine" is the plugin structure itself: commands set intent, templates define output contracts, guardrails encode rules, and hooks maintain context across compaction.

---

## System Architecture

```
┌─────────────────────────────────────────────────────────────────────┐
│  User                                                               │
│  Types: /sensei:brainstorm, /sensei:build, /sensei:refocus, etc.   │
└──────────────────┬──────────────────────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│  Claude Code Plugin System                                          │
│                                                                     │
│  marketplace/                                                       │
│  ├── commands/         ← slash commands (markdown, executed by AI)  │
│  │   ├── brainstorm.md                                              │
│  │   ├── build.md                                                   │
│  │   ├── refocus.md                                                 │
│  │   └── ...                                                        │
│  ├── skills/           ← auto-triggered behaviors (kept skills)     │
│  │   ├── codebase-indexing/                                         │
│  │   ├── test-gen/                                                  │
│  │   ├── refactor/                                                  │
│  │   └── extract-docs/                                              │
│  ├── hooks/            ← event-driven automation                    │
│  │   ├── hooks.json                                                 │
│  │   ├── session-start ← inject context + guardrails on start       │
│  │   ├── pre-compact   ← auto-refocus before compaction (NEW)       │
│  │   ├── pre-tool      ← analytics capture                         │
│  │   └── post-tool     ← analytics capture                         │
│  └── plugins/          ← MCP server configs                         │
│      └── sensei-mcp/                                                │
│                                                                     │
└──────────────────┬──────────────────────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│  Project Files                                                      │
│                                                                     │
│  .sensei/                                                           │
│  ├── config.yaml       ← per-project config (command overrides)     │
│  └── guardrails.md     ← living rules document                     │
│                                                                     │
│  docs/                                                              │
│  ├── ideas/            ← phase 01 artifacts                         │
│  ├── analysis/         ← phase 02 artifacts                         │
│  ├── blueprints/       ← phase 03 artifacts                         │
│  ├── experiments/      ← phase 04 artifacts                         │
│  ├── plans/            ← phase 05 artifacts                         │
│  └── templates/        ← doc templates per phase                    │
│                                                                     │
│  PATTERNS.md           ← pattern registry                           │
│                                                                     │
└──────────────────┬──────────────────────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│  senseid (Rust daemon on :7744)                                     │
│  ├── Graph DB (symbols, calls, patterns)                            │
│  ├── Session store                                                  │
│  ├── Project registry                                               │
│  └── HTTP API                                                       │
│                                                                     │
│  sensei-mcp (Rust MCP binary)                                       │
│  └── Exposes daemon capabilities as MCP tools                       │
└─────────────────────────────────────────────────────────────────────┘
```

---

## Component Design

### 1. Commands (markdown files)

Each command is a markdown file in `marketplace/commands/`. Claude Code loads it when the user types the slash command. The file contains instructions that the AI follows — there is no runtime code.

**Structure of a phase command:**

```markdown
---
description: <one-line for command picker>
argument-hint: <what arguments are accepted>
---

## Intent
<what this phase is for, what constraints apply>

## Prior artifacts
<which docs to read before starting — load from configured output_dir>

## Procedure
<step-by-step instructions for the AI>

## Output
<where to write the artifact, which template to use>

## Nudges
<when to suggest moving to next phase, when to suggest /sensei:experiment>
```

**Key design decisions:**
- Commands read config from `.sensei/config.yaml` to determine output directories, naming, templates
- Commands load guardrails from `.sensei/guardrails.md` at the start
- Phase commands load prior phase artifacts (e.g., `/sensei:blueprint` reads from `docs/analysis/`)
- `/sensei:brainstorm` is the open container — it routes content to the right folder based on depth (D11)

### 2. Guardrails file

**Location:** `.sensei/guardrails.md`

A living document at project level that captures enforceable rules. Different from CLAUDE.md (project setup) and memory (user facts). Guardrails store **how to build in this project**.

```markdown
---
name: Project Guardrails
updated: 2026-04-17
---

# Guardrails

## Patterns
- Use adapter pattern for all language parsers (see PATTERNS.md → adapter)
- Task workers wrap testable pure functions — never put logic in the worker itself

## Quality
- TDD: write test first, then implement
- Zero errors policy: `cargo test` must pass before and after every change

## Architecture
- All new MCP tools go through the Rust daemon HTTP API, never direct DB access
- File paths in the graph use repo-relative paths, never absolute

## Tools
- Prefer sensei MCP `search()` over grep for symbol lookup
- Use `get_callers()`/`get_callees()` for dependency analysis, not manual file reading
```

**Lifecycle:**
1. Created during project setup (or first `/sensei:guardrails` call creates a template)
2. Grows from feedback: user corrects AI → AI asks clarifying questions → adds rule to guardrails
3. Loaded automatically by session-start hook
4. Reloaded on demand by `/sensei:guardrails`
5. Reloaded automatically by pre-compact hook (lightweight summary)

### 3. Hooks

#### session-start (revised)

Fires on: startup, resume, clear, compact.

**What changes from current:**
- Adds guardrails loading: reads `.sensei/guardrails.md` and injects a compact summary
- Adds command awareness: lists available workflow commands grouped by category
- Adds tool preference reminder: "prefer sensei MCP tools over grep/sed"
- Keeps: session creation on daemon, MCP tool list, package manager rules

#### pre-compact (new)

Fires on: PreCompact event.

**Purpose:** Before context is compressed, save critical state so it survives compaction.

```bash
#!/usr/bin/env bash
# Reads current guardrails and outputs a compact reminder
GUARDRAILS=$(cat "${CLAUDE_PROJECT_ROOT}/.sensei/guardrails.md" 2>/dev/null | head -50)
# Output compact context that survives compaction
cat <<EOF
{
  "additional_context": "<sensei-refocus>\n## Active guardrails (auto-loaded)\n${GUARDRAILS}\n\n## Reminder\n- Use sensei MCP tools before grep/sed\n- Check PATTERNS.md before implementing\n- Content goes to its natural depth folder\n</sensei-refocus>"
}
EOF
```

**Key:** This is lightweight — just guardrails + tool reminder. Full refocus (phase doc, plan, task) is the manual `/sensei:refocus` command.

#### pre-tool / post-tool (wire existing)

Already implemented but not registered in hooks.json. Wire them:

```json
{
  "hooks": {
    "SessionStart": [ ... ],
    "PreCompact": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/run-hook.cmd pre-compact"
          }
        ]
      }
    ],
    "PreToolUse": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/run-hook.cmd pre-tool"
          }
        ]
      }
    ],
    "PostToolUse": [
      {
        "matcher": "",
        "hooks": [
          {
            "type": "command",
            "command": "${CLAUDE_PLUGIN_ROOT}/hooks/run-hook.cmd post-tool"
          }
        ]
      }
    ]
  }
}
```

### 4. Configuration

**Global:** `~/.sensei/config.yaml`
**Project:** `.sensei/config.yaml` (overrides global)

```yaml
# .sensei/config.yaml
workflow:
  recipe: full                    # full | lean | exploratory | maintenance | custom

commands:
  idea:
    output_dir: docs/ideas
    naming: "{name}.md"
  analysis:
    output_dir: docs/analysis
    naming: "{name}.md"
  blueprint:
    output_dir: docs/blueprints
    naming: "{name}.md"
  experiment:
    output_dir: docs/experiments
    naming: "{name}.md"
  plan:
    output_dir: docs/plans
    naming: "{name}.md"
  build:
    tdd: true
    review_on_complete: true
  review:
    strictness: advisory          # strict | advisory | minimal
```

**How commands read config:** Each command's markdown includes instructions to read `.sensei/config.yaml` for its section. If the file doesn't exist, use defaults. This is pure AI instruction — no runtime code needed.

### 5. Templates

**Location:** `docs/templates/`

Each phase gets a template that defines the output contract:

| Template | Phase | Key sections |
|----------|-------|-------------|
| `idea.md` | 01 Ideate | Problem, goals, constraints, open questions |
| `analysis.md` | 02 Analyze | Current state, feasibility, approaches (2-3 options with tradeoffs), recommendation |
| `blueprint.md` | 03 Blueprint | Overview, architecture diagram, components, interfaces, data flow, integration points |
| `experiment.md` | 04 Experiment | Hypothesis, approach, findings, recommendation (incorporate or discard) |
| `plan.md` | 05 Plan | Ordered features, acceptance criteria per feature, test scenarios, dependencies |
| `design.md` | (existing) | Component internals — schema, algorithm, API contracts, error handling, testing |
| `feature.md` | (existing) | User-facing needs — Gherkin scenarios, status tracking |

All templates use frontmatter with `name`, `description`, `date`, `status`, `origin` (link to parent doc).

### 6. Content routing (brainstorm behavior)

`/sensei:brainstorm` is the open creative command. It routes content based on depth:

| Depth signal | Target folder | Example |
|-------------|---------------|---------|
| Problem statement, vague concept, "what if" | `docs/ideas/` | "What if we had a way to track library versions?" |
| Feasibility assessment, existing code analysis, tradeoffs | `docs/analysis/` | "Here are 3 approaches with pros/cons based on the current codebase" |
| Architecture, components, interfaces, data flow | `docs/blueprints/` | "The system has 3 components connected like this" |
| Findings from trying something | `docs/experiments/` | "We tried approach A and B, here's what worked" |
| Task breakdown with acceptance criteria | `docs/plans/` | "Feature 1: do X, test Y. Feature 2: do Z, test W." |

The AI uses judgment to route. When in doubt, it asks: "This is getting into blueprint territory — should I put this in `docs/blueprints/`?"

### 7. Workflow state file

**Location:** `.sensei/state.yaml`

The single source of truth for "where am I." Updated by commands, read by hooks and refocus. This is the **testable artifact** — after running a command, check if state was updated correctly.

```yaml
# .sensei/state.yaml (managed by commands and hooks, not edited by hand)
active_phase: build
active_plan: docs/plans/workflow-engine.md
active_task: "Wave 1, item 3: /sensei:guardrails command"
active_issue: 42                    # GitHub issue number, if tracking via issues
last_checkpoint: "2026-04-17T12:00:00Z"
guardrails_hash: "a3f2..."         # quick check if guardrails changed since last load
```

**Lifecycle:**
1. Phase commands update `active_phase` when invoked
2. `/sensei:plan` sets `active_plan`; `/sensei:build` advances `active_task`
3. `/sensei:checkpoint` updates `last_checkpoint`
4. Session-start hook reads this to orient the AI
5. Pre-compact hook reads this to preserve state across compaction
6. `/sensei:refocus` reads this to re-anchor

**Testability:** After any command, you can `cat .sensei/state.yaml` and verify the state is correct. Hooks can be tested by running the script and checking JSON output against state.yaml contents.

### 8. `/sensei:status` command

The "where am I" query. Reads state file + project structure and returns full orientation:

```
/sensei:status

Phase:      build
Plan:       docs/plans/workflow-engine.md
Task:       Wave 1, item 3: /sensei:guardrails command
Issue:      #42 (open)
Checkpoint: 2026-04-17T12:00:00Z

Guardrails: .sensei/guardrails.md (12 rules loaded)
Patterns:   PATTERNS.md (3 patterns)

Docs:
  ideas/       7 files
  analysis/    1 file
  blueprints/  1 file
  plans/       0 files

Tools: sensei-mcp connected (14 tools)
```

This is also callable by the AI internally when it feels lost — not just a user-facing command. The pre-compact hook can remind the AI: "call `/sensei:status` if you need orientation after compaction."

### 9. Backlog management — GitHub issues as source of truth

Markdown backlogs go stale because the AI forgets to update them. GitHub issues are persistent, queryable, and manageable independently. For repos on GitHub, issues ARE the backlog.

**Flow:**

```
Brainstorm/Ideate
  → Feature doc (docs/ideas/ or docs/features/)
    → Push as GitHub issue (with labels)
      → AI picks issue → works on it
        → Closes issue referencing commit
```

**Issue labels:**

| Label type | Values | Purpose |
|-----------|--------|---------|
| `concept:` | workflow, qualitative, quantitative, tooling, integrity, platform | Which core concept |
| `depth:` | idea, analysis, blueprint, experiment, build | How deep to go |
| `wave:` | 1, 2, 3 | Implementation wave |
| `priority:` | critical, high, medium, low | Ordering |
| `type:` | feature, bug, enhancement, cleanup | Nature of work |

**GitHub issue template:**

```markdown
---
name: Feature
about: New capability or enhancement
labels: type:feature
---

## Summary
<!-- One paragraph: what and why -->

## Depth
<!-- How far should this go? -->
- [ ] Idea — explore the concept, document in docs/ideas/
- [ ] Analysis — assess feasibility, map against existing code
- [ ] Blueprint — design the architecture
- [ ] Build — implement with TDD

## Acceptance criteria
<!-- How do we know this is done? -->
- [ ] ...

## Related docs
<!-- Links to idea/analysis/blueprint docs if they exist -->

## Context
<!-- Feature doc, design doc, or conversation that led to this -->
```

**Commands integration:**

| Command | GitHub interaction |
|---------|-------------------|
| `/sensei:plan` | Reads approved blueprint → creates GitHub issues for each feature (or links to existing ones) |
| `/sensei:build` | Reads open issues → picks highest priority → sets `active_issue` in state.yaml → works on it |
| `/sensei:refocus` | Shows current issue, remaining issues in milestone |
| `/sensei:status` | Shows active issue number and title |
| `/sensei:commit` | Includes `Closes #42` or `Refs #42` in commit message |
| `/sensei:validate` | Verifies acceptance criteria from issue are met before closing |

**Roadmap decomposition:**
The entire roadmap (all 3 waves, 22 components) should be pushed as GitHub issues with wave milestones. Then implementation becomes: pick the next open issue, work on it, close it. One at a time, verified.

**For non-GitHub repos:** Fall back to `docs/backlog.md` with a simple task list. The commands detect whether `gh` is available and the repo has a remote.

---

## Data Architecture — What lives where

### The three layers

```
┌─────────────────────────────────────────────────────────────────────┐
│  Desktop (Tauri + SvelteKit)                                        │
│  Sees, configures, analyzes                                         │
│  ├── Phase progression timeline                                     │
│  ├── Quality metrics charts (FTR, turn count, rework rate)          │
│  ├── Event log viewer                                               │
│  ├── Issue/backlog status                                           │
│  ├── Guardrails editor                                              │
│  └── Configuration UI                                               │
│                                                                     │
│  Reads from daemon HTTP API. Does NOT write workflow state.         │
└──────────────────┬──────────────────────────────────────────────────┘
                   │ HTTP (read)
                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│  Daemon (senseid :7744) — persistent store + compute                │
│                                                                     │
│  Stores (SQLite):                                                   │
│  ├── events        — append-only log of everything that happens     │
│  ├── sessions      — session lifecycle (start, checkpoint, end)     │
│  ├── metrics       — computed from events (FTR, turn count, etc.)   │
│  └── phase_history — timeline of phase transitions per project      │
│                                                                     │
│  Computes:                                                          │
│  ├── FTR score (from rework events / total tasks)                   │
│  ├── Turn efficiency (turns per task)                               │
│  ├── Tool preference adherence (MCP vs grep usage ratio)            │
│  ├── Pattern adherence (guardrail violations over time)             │
│  └── Phase velocity (time spent per phase)                          │
│                                                                     │
│  HTTP API:                                                          │
│  ├── POST /api/events         — log an event                       │
│  ├── GET  /api/metrics/:proj  — computed metrics for a project     │
│  ├── GET  /api/phases/:proj   — phase transition history           │
│  └── GET  /api/state/:proj    — current workflow state              │
└──────────────────┬──────────────────────────────────────────────────┘
                   │ MCP (tools)
                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│  MCP (sensei-mcp) — exposes daemon to AI                            │
│                                                                     │
│  Existing tools (code intelligence):                                │
│  ├── search(), get_callers(), get_callees()                         │
│  ├── get_patterns(), get_communities()                              │
│  ├── get_lib_docs(), search_lib_docs(), add_library()               │
│  └── get_project_summary()                                          │
│                                                                     │
│  New tools (workflow intelligence):                                  │
│  ├── log_event(type, data)     — record workflow event              │
│  ├── get_workflow_state()      — current phase, task, issue         │
│  ├── get_metrics(range?)       — FTR, turns, adherence scores       │
│  └── update_phase(phase, task?, issue?)  — transition phase         │
└──────────────────┬──────────────────────────────────────────────────┘
                   │
                   ▼
┌─────────────────────────────────────────────────────────────────────┐
│  State file (.sensei/state.yaml) — local fast-read for hooks        │
│                                                                     │
│  Written by: commands (via MCP or directly)                         │
│  Read by: hooks (bash — no MCP access), /sensei:status              │
│                                                                     │
│  This is a CACHE of daemon state, not the source of truth.          │
│  If state.yaml is missing, commands recreate it from daemon.        │
│  If daemon is down, commands work from state.yaml (degraded mode).  │
└─────────────────────────────────────────────────────────────────────┘
```

### Event types

The daemon's event log captures everything needed for analysis. Events are appended by hooks and commands via `POST /api/events` (or MCP `log_event()`).

| Event type | Source | Data captured | Used for |
|-----------|--------|---------------|----------|
| `phase_transition` | Phase commands | from_phase, to_phase, trigger (command or nudge) | Phase velocity, workflow patterns |
| `command_invoked` | All commands | command_name, arguments, phase_context | Command usage analytics |
| `tool_used` | pre-tool hook | tool_name, is_mcp (bool) | Tool preference adherence |
| `tool_result` | post-tool hook | tool_name, exit_code, duration_ms | Tool reliability metrics |
| `checkpoint` | `/sensei:checkpoint` | summary, phase, task | Session continuity |
| `issue_started` | `/sensei:build` | issue_number, title | Task tracking |
| `issue_completed` | `/sensei:commit`, `/sensei:validate` | issue_number, turns_taken | FTR, turn efficiency |
| `review_finding` | `/sensei:review` | finding_type, severity, file | Quality trends |
| `guardrail_added` | `/sensei:guardrails` | rule_text, triggered_by | Guardrail growth tracking |
| `rework` | AI detects it's redoing prior work | original_task, reason | Rework rate, coaching |
| `compaction` | pre-compact hook | context_preserved (summary) | Context decay measurement |

### How capture happens

**The AI only calls MCP tools. It never calls daemon HTTP endpoints directly.**

Three callers, three paths:

```
AI (Claude)  → MCP tools (sensei-mcp)  → Daemon HTTP API (senseid)
Hooks (bash) → Daemon HTTP API (curl)   directly
Desktop      → Daemon HTTP API (fetch)  directly
```

| Capture point | Mechanism | Notes |
|--------------|-----------|-------|
| **Hooks** (pre-tool, post-tool, session-start, pre-compact) | `curl` to daemon HTTP | Bash scripts, no MCP access. Already implemented for pre-tool/post-tool. Add phase context from state.yaml. |
| **Commands** (phase transitions, checkpoints, issue lifecycle) | MCP `log_event()` tool | Commands include AI instructions: "call `log_event(type='phase_transition', data={...})`" |
| **Rework detection** | AI calls MCP `log_event()` | Command instructions: "if you notice you're redoing work from a prior session, call `log_event(type='rework', ...)`" |
| **State file updates** | Commands call MCP `update_phase()` | MCP tool writes to daemon AND updates `.sensei/state.yaml` for hook access |

### What hooks capture vs. what commands capture

```
Hook (bash → curl to daemon)             Command (AI → MCP tool → daemon)
├── tool_used (every tool call)           ├── phase_transition
├── tool_result (every tool call)         ├── command_invoked
├── compaction (pre-compact)              ├── checkpoint
└── session lifecycle                     ├── issue_started / issue_completed
                                          ├── review_finding
                                          ├── guardrail_added
                                          └── rework
```

Hooks handle high-frequency, automatic capture (bash → curl). Commands handle semantic, intent-driven capture (AI → MCP).

### Desktop reads, daemon computes

The desktop app reads computed metrics from the daemon HTTP API. It does NOT process raw events — the daemon does the aggregation.

| Desktop view | Daemon endpoint | What it shows |
|-------------|-----------------|---------------|
| Phase timeline | `GET /api/phases/:proj` | Visual timeline of phase transitions with duration |
| Quality dashboard | `GET /api/metrics/:proj` | FTR trend, turn count trend, rework rate, tool adherence |
| Event log | `GET /api/events/:proj?limit=50` | Recent events with filtering |
| Active work | `GET /api/state/:proj` | Current phase, task, issue |

### MCP tools (what the AI calls)

| Tool | Purpose | Called by |
|------|---------|----------|
| `log_event(type, data)` | Record a workflow event | Commands (AI instructions say "call this tool") |
| `get_workflow_state()` | Return current phase, task, issue, guardrails status | `/sensei:status`, `/sensei:refocus`, AI when lost |
| `get_metrics(range?)` | Return computed FTR, turn count, rework rate | `/sensei:analyze` when reviewing interaction quality |
| `update_phase(phase, task?, issue?)` | Transition phase, update state.yaml + daemon | Phase commands |

### Daemon HTTP API (internal — never called by AI)

Called by hooks (curl), desktop (fetch), and sensei-mcp (internal). The AI never sees these.

| Endpoint | Method | Caller | Purpose |
|----------|--------|--------|---------|
| `/api/events` | POST | hooks, sensei-mcp | Log an event |
| `/api/events/:proj` | GET | desktop, sensei-mcp | List events for a project |
| `/api/state/:proj` | GET | desktop, sensei-mcp, hooks | Current workflow state |
| `/api/state/:proj` | PUT | sensei-mcp | Update workflow state |
| `/api/metrics/:proj` | GET | desktop, sensei-mcp | Computed metrics |
| `/api/phases/:proj` | GET | desktop, sensei-mcp | Phase transition history |

---

## Verification & Testability

How each component type can be tested:

| Component | Test method | What to verify |
|-----------|------------|----------------|
| **Hooks** (bash scripts) | Run script, check JSON output | Output contains expected context, state.yaml values, guardrails |
| **State file** | `cat .sensei/state.yaml` after command | Phase, task, issue, checkpoint updated correctly |
| **Commands** (markdown) | E2E conversation test OR manual walkthrough | Correct artifact created in correct folder with correct frontmatter |
| **Guardrails** | `/sensei:status` shows loaded count | File exists, rules parsed, hash matches |
| **GitHub issues** | `gh issue list --label "wave:1"` | Issues exist, labels correct, milestone set |
| **Pre-compact hook** | Trigger compaction, check if AI retains guardrails | AI can still reference guardrails and current task after compaction |
| **Content routing** | Check file location after brainstorm | Design-depth content in blueprints/, idea-depth in ideas/ |

**E2E test approach (using Playwright MCP or scripted sessions):**
1. Start a session → verify session-start hook injected context
2. Run `/sensei:idea "test concept"` → verify file created in `docs/ideas/` with correct template
3. Run `/sensei:status` → verify state shows phase: ideate
4. Trigger compaction → verify pre-compact hook fires, AI retains context
5. Run `/sensei:refocus` → verify AI re-anchors to current task
6. Run `/sensei:build` on an issue → verify state.yaml updates, tests written first, issue closed on completion

---

## Implementation Order

Based on analysis priority actions and dependency chain:

### Wave 1: Foundation (unblocks everything)

| # | Component | Type | Depends on |
|---|-----------|------|-----------|
| 1 | Phase doc templates (idea, analysis, blueprint, experiment, plan) | Templates | Nothing |
| 2 | Guardrails file template | Template | Nothing |
| 3 | State file schema (`.sensei/state.yaml`) | Schema | Nothing |
| 4 | Daemon: event log + workflow state endpoints | Rust (senseid) | Nothing |
| 5 | MCP: `log_event`, `get_workflow_state`, `update_phase` tools | Rust (sensei-mcp) | Daemon endpoints |
| 6 | `/sensei:guardrails` command | Command | Guardrails template |
| 7 | `/sensei:refocus` command | Command | State file, MCP tools |
| 8 | `/sensei:status` command | Command | State file, MCP tools |
| 9 | Pre-compact hook | Hook | Guardrails template, state file |
| 10 | Wire pre-tool/post-tool hooks (add phase context) | Hook config | State file |
| 11 | Update session-start hook | Hook | Guardrails template, state file |
| 12 | GitHub issue template + labels | GitHub config | Nothing |

### Wave 2: Phase commands (core workflow)

| # | Component | Type | Depends on |
|---|-----------|------|-----------|
| 13 | `/sensei:brainstorm` command | Command | Templates, config, MCP tools |
| 14 | `/sensei:idea` command | Command | Idea template, MCP tools |
| 15 | `/sensei:analyze` command | Command | Analysis template, MCP tools |
| 16 | `/sensei:blueprint` command | Command | Blueprint template, MCP tools |
| 17 | `/sensei:experiment` command | Command | Experiment template, MCP tools |
| 18 | `/sensei:plan` command | Command | Plan template, MCP tools, GitHub issues |
| 19 | `/sensei:build` command | Command | Plan template, guardrails, MCP tools, GitHub issues |
| 20 | `/sensei:validate` command | Command | MCP tools, GitHub issues |

### Wave 3: Cross-cutting and polish

| # | Component | Type | Depends on |
|---|-----------|------|-----------|
| 21 | `/sensei:review` command | Command | MCP tools |
| 22 | `/sensei:tools` command | Command | Nothing |
| 23 | `/sensei:patterns` command | Command | PATTERNS.md |
| 24 | Daemon: `get_metrics` endpoint | Rust (senseid) | Event log |
| 25 | MCP: `get_metrics` tool | Rust (sensei-mcp) | Daemon endpoint |
| 26 | Update `/sensei:help` | Command | All commands exist |
| 27 | Update catalog.json | Config | All commands exist |
| 28 | Retire absorbed skills | Cleanup | Replacement commands tested |
| 29 | Archive `docs/superpowers/` | Cleanup | Nothing |
| 30 | Push roadmap as GitHub issues | Backlog | All waves defined |

---

## What this blueprint does NOT cover

- **MCP tool contracts for code intelligence**: Stale references marked (D13). New commands will use current Rust API tools directly during build phase.
- **Dashboard UI implementation**: Separate idea (10). The data architecture above defines what the desktop reads — building the actual views is a separate effort.
- **Multi-coordinator**: Separate idea (12). Build for Claude Code first, abstract later.
- **Metric computation algorithms**: The daemon endpoints and event types are defined above. The actual FTR/rework/adherence computation logic is implementation detail for the daemon build.

---

## Next step

`/sensei:plan` — decompose Wave 1 into implementable features with acceptance criteria and test scenarios.
