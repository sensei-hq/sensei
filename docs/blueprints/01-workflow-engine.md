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

---

## Implementation Order

Based on analysis priority actions and dependency chain:

### Wave 1: Foundation (unblocks everything)

| # | Component | Type | Depends on |
|---|-----------|------|-----------|
| 1 | Phase doc templates (idea, analysis, blueprint, experiment, plan) | Templates | Nothing |
| 2 | Guardrails file template | Template | Nothing |
| 3 | `/sensei:guardrails` command | Command | Guardrails template |
| 4 | `/sensei:refocus` command | Command | Nothing |
| 5 | Pre-compact hook | Hook | Guardrails template |
| 6 | Wire pre-tool/post-tool hooks | Hook config | Nothing |
| 7 | Update session-start hook | Hook | Guardrails template |

### Wave 2: Phase commands (core workflow)

| # | Component | Type | Depends on |
|---|-----------|------|-----------|
| 8 | `/sensei:brainstorm` command | Command | Templates, config |
| 9 | `/sensei:idea` command | Command | Idea template |
| 10 | `/sensei:analyze` command | Command | Analysis template |
| 11 | `/sensei:blueprint` command | Command | Blueprint template |
| 12 | `/sensei:experiment` command | Command | Experiment template |
| 13 | `/sensei:plan` command | Command | Plan template |
| 14 | `/sensei:build` command | Command | Plan template, guardrails |
| 15 | `/sensei:validate` command | Command | Nothing |

### Wave 3: Cross-cutting and polish

| # | Component | Type | Depends on |
|---|-----------|------|-----------|
| 16 | `/sensei:review` command | Command | Nothing |
| 17 | `/sensei:tools` command | Command | Nothing |
| 18 | `/sensei:patterns` command | Command | PATTERNS.md |
| 19 | Update `/sensei:help` | Command | All commands exist |
| 20 | Update catalog.json | Config | All commands exist |
| 21 | Retire absorbed skills | Cleanup | Replacement commands tested |
| 22 | Archive `docs/superpowers/` | Cleanup | Nothing |

---

## What this blueprint does NOT cover

- **MCP tool contracts**: Stale references marked (D13). New commands will use current Rust API tools directly during build phase.
- **Dashboard/visualization**: Separate idea (10). Orthogonal to workflow engine.
- **Multi-coordinator**: Separate idea (12). Build for Claude Code first, abstract later.
- **Metrics instrumentation**: Pre-tool/post-tool hooks capture data. Analysis of that data is a separate feature.

---

## Next step

`/sensei:plan` — decompose Wave 1 into implementable features with acceptance criteria and test scenarios.
