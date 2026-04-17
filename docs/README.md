---
name: Sensei Documentation
description: Root index for all sensei documentation — product vision, ideas, analysis, blueprints, plans, and legacy docs
date: 2026-04-17
---

# Sensei Documentation

## What is sensei?

Sensei is a **developer intelligence platform** that makes AI-assisted development more efficient, more consistent, and more measurable. It sits between the developer, the AI coding assistant (Claude Code, Cursor, etc.), and the codebase — providing the context, patterns, and guardrails that prevent rework.

## Key problems we're solving

| # | Problem | Impact | How sensei addresses it |
|---|---------|--------|------------------------|
| 1 | **The AI starts cold every session** — decisions, patterns, and constraints evaporate between conversations | Rework. The AI rebuilds understanding from scratch, makes the same mistakes, ignores established patterns. | Phase documents survive between sessions. Guardrails persist. Session state tracks where you left off. The AI reads, it doesn't need to remember. |
| 2 | **No phase discipline** — ideation, design, and implementation blur together | Premature coding. The AI writes code before the design is clear, leading to throwaway work. Auto-mode amplifies this. | Workflow commands set intent. `/sensei:brainstorm` for exploration, `/sensei:build` for implementation. Soft gates nudge when details are insufficient. |
| 3 | **The AI reinvents structure** — ignores existing patterns, creates duplicates, doesn't follow conventions | Inconsistent codebase. Adapter pattern exists but the AI writes a standalone parser. Task worker pattern exists but the AI writes a monolith. | Pattern detection in the indexer. `get_patterns()` during the locate step. Pattern enforcement during review. Guardrails capture "always use X pattern for Y." |
| 4 | **No testability guidance** — AI writes monolithic functions, TDD is stated but not enforced | Untestable code. Tests written after implementation validate what was built, not what should be built. | Decomposition step before coding. Tests presented to user for approval before implementation. Testability scoring from graph. |
| 5 | **Context decays in long conversations** — guardrails, tool awareness, and task focus disappear after compaction | The AI forgets constraints. Uses grep instead of MCP. Ignores guardrails it was given 50 turns ago. | PreCompact hook auto-reloads guardrails. Refocus commands re-anchor manually. State file persists across compaction. |
| 6 | **No way to measure quality** — FTR, turn count, rework rate are invisible | No feedback loop. Can't tell if development quality is improving or which patterns cause rework. | Event capture (hooks + commands). Metrics computed by daemon. Desktop dashboard shows trends. Coaching from analysis. |
| 7 | **Code intelligence is shallow** — indexer knows symbol names but not function shape, patterns, or relationships | AI can't navigate the codebase intelligently. Can't detect "this should be an adapter" or "this function has too many dependencies." | Rich graph nodes (params, returns, implements, docstring). Pattern detection. Testability scoring. Traceability matrix. |
| 8 | **Documentation drifts from code** — design docs go stale, the AI trusts outdated information | Decisions based on wrong information. Requirements change but docs don't follow. | Doc nodes with frontmatter. TRACES_TO edges. Drift detection. Freshness scoring. |

## User journeys

### Journey 1: Starting a new project

```
1. Developer installs sensei, runs `senseid start`
2. Opens project in Claude Code — session-start hook fires, injects context
3. `/sensei:brainstorm` — "I want to build a task scheduler"
   → AI asks clarifying questions, produces docs/ideas/task-scheduler.md
4. `/sensei:analyze` — AI scans codebase, finds related patterns, produces analysis
5. `/sensei:blueprint` — AI designs the architecture, presents options
6. `/sensei:plan` — AI decomposes into GitHub issues with acceptance criteria
7. `/sensei:build` on first issue — locate step → decompose → tests → implement
8. Desktop shows: phase timeline, quality metrics, pattern coverage
```

### Journey 2: Daily development workflow

```
1. Start session — `/sensei:session` loads last checkpoint, open decisions
2. `/sensei:status` — shows current phase, active issue, guardrails loaded
3. Pick next issue from backlog (or `/sensei:build` picks highest priority)
4. AI runs locate step — search(), get_patterns(), get_callers()
5. AI proposes decomposition — pure functions + orchestrator
6. AI writes tests, presents for approval — user confirms
7. AI implements — runs tests — shows results
8. `/sensei:review` auto-triggers — checks patterns, duplicates, quality
9. `/sensei:commit` — zero-errors check, closes issue with reference
10. Move to next issue
```

### Journey 3: Mid-conversation course correction

```
1. AI is implementing a feature, going off track
2. User notices: types `/sensei:refocus`
   → AI re-reads state, plan, current task, guardrails
   → "I was working on issue #42. I had drifted into refactoring unrelated code.
      Returning to: implement SqlAdapter following the language-adapter pattern."
3. Or: context compacts automatically
   → PreCompact hook fires, saves guardrails + state
   → AI retains: active task, key constraints, tool preferences
```

### Journey 4: Quality investigation (desktop)

```
1. Developer opens sensei desktop — sees quality dashboard
2. FTR has dropped from 80% to 60% this week
3. Drills into metrics: turn count is up, rework events concentrated in "adapters" module
4. Clicks into event log — sees 3 correction events: "wrong pattern", "should use adapter"
5. Sees recommendation: "Pattern adherence dropped for adapter module.
   Consider adding guardrail: 'all parsers must implement LanguageAdapter trait'"
6. Clicks "Add guardrail" — updates .sensei/guardrails.md
7. Next session: AI reads guardrail, follows adapter pattern, FTR recovers
```

### Journey 5: Pattern discovery and enforcement

```
1. Developer notices AI keeps creating similar-looking files differently
2. `/sensei:patterns` — shows all detected patterns + coverage
3. Sees: "4 adapters follow LanguageAdapter pattern, but 2 parsers don't conform"
4. `/sensei:pattern-extract` on best adapter implementation
   → AI extracts: interface, invariants, registration, file structure
   → Adds to PATTERNS.md
5. `/sensei:guardrails` — adds: "all new parsers must follow adapter pattern"
6. Next `/sensei:build`: AI automatically follows the pattern
7. `/sensei:review` flags any new code that violates it
```

### Journey 6: Exploring before committing (experiments)

```
1. Developer has a vague idea: "maybe we should use RxJS for real-time updates"
2. `/sensei:experiment` — AI creates branch, builds minimal prototype
3. Produces docs/experiments/rxjs-realtime.md — what worked, what didn't, recommendation
4. Developer reviews: "too complex for our needs, let's use simpler polling"
5. Experiment branch discarded, but findings doc is preserved
6. Next time someone suggests RxJS: the experiment doc shows why polling was chosen
```

### Journey 7: Guided prompts from analysis

```
1. Desktop analytics show: high rework rate in "api routes" module
2. System analyzes event data:
   - 5 issues in api routes this week, 3 had rework
   - Common correction: "missing validation middleware"
   - Pattern: api routes don't consistently use validation
3. System generates guided prompt:
   "Your api routes have a 40% rework rate, primarily from missing validation.
    Consider: `/sensei:pattern-extract` on a well-validated route to establish
    the validation pattern. Then add to guardrails."
4. Or, during a coding session: AI proactively says:
   "I notice this is an api route. Your routes sometimes miss validation —
    should I include validation middleware? (Based on 3 recent corrections)"
```

---

## Documentation structure

### Active (current)

| Folder | Phase | Purpose | Status |
|--------|-------|---------|--------|
| [ideas/](./ideas/) | 01 Ideate | Problem statements, concepts, early exploration | 18 files |
| [analysis/](./analysis/) | 02 Analyze | Feasibility, gap analysis, mapping against existing code | 2 files |
| [blueprints/](./blueprints/) | 03 Blueprint | Architecture, components, interfaces, data flow | 1 file |
| experiments/ | 04 Experiment | Findings from trying options (not yet created) | — |
| plans/ | 05 Plan | Task breakdowns with acceptance criteria (not yet created) | — |
| [templates/](./templates/) | — | Doc templates for each phase | 2 files (needs expansion) |

### Reference (still valid, needs refresh)

| Folder | Purpose | Status |
|--------|---------|--------|
| [features/](./features/) | User-facing feature specs with Gherkin scenarios | 12 files — good template, content needs updating |
| [design/](./design/) | Technical design docs — schemas, algorithms, APIs | 30+ files — many stale (Supabase refs), some still valid |
| [roadmap/](./roadmap/) | Architectural direction docs | 12 files — mix of current and stale |
| [gap-analysis.md](./gap-analysis.md) | Codebase health assessment (2026-04-10) | Valid — model for `/sensei:analyze` output |

### Archive (to be moved per D1)

| Folder | Purpose | Status |
|--------|---------|--------|
| [superpowers/](./superpowers/) | Plans and specs from prior plugin-building phase | 43 files — to archive to `_archive/superpowers/` |

---

## Core concepts

| # | Concept | Ideas | Description |
|---|---------|-------|-------------|
| 1 | Workflow | 01–06 | Phased development workflow — commands, configuration, quality gates |
| 2 | Logging & Qualitative Analysis | 07, 11 | Interaction tracking, session recovery, coaching feedback |
| 3 | Metrics & Quantitative Analysis | 07, 10 | FTR, turn count, rework rate, trend visualization |
| 4 | Assistive Tooling | 08, 09, 14, 15, 17, 18 | Code intelligence, library knowledge, patterns, testability |
| 5 | Knowledge Integrity | 13 | Doc traceability, drift detection, freshness |
| 6 | Platform & Adoption | 12, 16 | Multi-coordinator, multi-repo, workspaces |

## Decisions log

All design decisions: [ideas/05-decisions.md](./ideas/05-decisions.md) (D1–D17)
