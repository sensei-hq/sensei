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
| 4 | `/sensei:guardrails` command | Command | Guardrails template |
| 5 | `/sensei:refocus` command | Command | State file |
| 6 | `/sensei:status` command | Command | State file |
| 7 | Pre-compact hook | Hook | Guardrails template, state file |
| 8 | Wire pre-tool/post-tool hooks | Hook config | Nothing |
| 9 | Update session-start hook | Hook | Guardrails template, state file |
| 10 | GitHub issue template + labels | GitHub config | Nothing |

### Wave 2: Phase commands (core workflow)

| # | Component | Type | Depends on |
|---|-----------|------|-----------|
| 11 | `/sensei:brainstorm` command | Command | Templates, config, state file |
| 12 | `/sensei:idea` command | Command | Idea template, state file |
| 13 | `/sensei:analyze` command | Command | Analysis template, state file |
| 14 | `/sensei:blueprint` command | Command | Blueprint template, state file |
| 15 | `/sensei:experiment` command | Command | Experiment template, state file |
| 16 | `/sensei:plan` command | Command | Plan template, state file, GitHub issues |
| 17 | `/sensei:build` command | Command | Plan template, guardrails, state file, GitHub issues |
| 18 | `/sensei:validate` command | Command | State file, GitHub issues |

### Wave 3: Cross-cutting and polish

| # | Component | Type | Depends on |
|---|-----------|------|-----------|
| 19 | `/sensei:review` command | Command | Nothing |
| 20 | `/sensei:tools` command | Command | Nothing |
| 21 | `/sensei:patterns` command | Command | PATTERNS.md |
| 22 | Update `/sensei:help` | Command | All commands exist |
| 23 | Update catalog.json | Config | All commands exist |
| 24 | Retire absorbed skills | Cleanup | Replacement commands tested |
| 25 | Archive `docs/superpowers/` | Cleanup | Nothing |
| 26 | Push roadmap as GitHub issues | Backlog | All waves defined |

---

## What this blueprint does NOT cover

- **MCP tool contracts**: Stale references marked (D13). New commands will use current Rust API tools directly during build phase.
- **Dashboard/visualization**: Separate idea (10). Orthogonal to workflow engine.
- **Multi-coordinator**: Separate idea (12). Build for Claude Code first, abstract later.
- **Metrics instrumentation**: Pre-tool/post-tool hooks capture data. Analysis of that data is a separate feature.

---

## Next step

`/sensei:plan` — decompose Wave 1 into implementable features with acceptance criteria and test scenarios.
