---
name: Marketplace Commands & Hooks
description: What needs to be built, revised, and retired in the marketplace plugin
date: 2026-04-17
type: design
traces:
  - ideas/01-workflow-system.md
  - ideas/02-commands.md
  - ideas/18-testability-tdd.md
  - analysis/01-skill-command-mapping.md
  - blueprints/01-workflow-engine.md
---

# Marketplace Commands & Hooks

## Overview

The marketplace plugin is the AI-facing layer — markdown commands that instruct the AI, hooks that capture events and maintain context, and skills that auto-trigger behaviors. This doc specifies what to build, revise, and retire.

---

## Commands to create (13 new)

### Phase commands (7)

| Command | Key behavior | MCP tools called | Traces to |
|---------|-------------|-----------------|-----------|
| `brainstorm.md` | Open creative conversation. Routes content to appropriate docs/ folder based on depth. | `update_phase`, `log_event`, `match_pattern` | ideas/01 (D11) |
| `idea.md` | Structured brainstorm. Ask clarifying questions. Output to docs/ideas/. No code. | `update_phase`, `log_event` | ideas/01 |
| `analyze.md` | Read idea doc + scan codebase. Produce feasibility with 2-3 options. Subsumes product/feature. | `update_phase`, `log_event`, `search`, `get_patterns`, `get_project_summary` | ideas/01, analysis/01 |
| `blueprint.md` | Architecture from chosen approach. Components, interfaces, data flow. No code. | `update_phase`, `log_event` | ideas/01 |
| `experiment.md` | Create branch. Build throwaway code. Produce findings doc. | `update_phase`, `log_event` | ideas/01 |
| `plan.md` | Decompose blueprint into features. Create GitHub issues. | `update_phase`, `log_event`, `gh issue create` | ideas/01 (D16, D17) |
| `build.md` | TDD cycle with locate step, decomposition, test approval, pattern enforcement. | `update_phase`, `log_event`, `search`, `get_callers`, `get_patterns`, `match_pattern` | ideas/01, ideas/18 |

### Cross-cutting commands (2)

| Command | Key behavior | MCP tools called | Traces to |
|---------|-------------|-----------------|-----------|
| `review.md` | Pattern conformance, duplicates, quality checks. Auto-triggers after build features. | `log_event`, `get_patterns`, `get_pattern_for`, `get_duplicates` | ideas/01, ideas/15 |
| `validate.md` | E2E tests, integration check, doc drift detection, acceptance criteria verification. | `log_event`, `get_workflow_state` | ideas/01, ideas/13 |

### Refocus commands (4)

| Command | Key behavior | MCP tools called | Traces to |
|---------|-------------|-----------------|-----------|
| `rules.md` | Re-read .sensei/rules.md, output compact summary. | `get_workflow_state` | ideas/01 (D9) |
| `patterns.md` | Re-read PATTERNS.md + detected patterns. Show catalog. | `get_patterns`, `get_project_conventions` | ideas/15, ideas/17 |
| `refocus.md` | Re-read state, plan, current task. Flush tangential context. | `get_workflow_state` | ideas/01 |
| `tools.md` | Re-read available MCP tools and preference hierarchy. | `get_workflow_state` | ideas/01 |

### Additional utility (1)

| Command | Key behavior | Traces to |
|---------|-------------|-----------|
| `status.md` | Read state.yaml + project structure, display full orientation. | blueprints/01 (section 8) |

---

## Hooks to create/revise (3)

| Hook | Event | Action | Traces to |
|------|-------|--------|-----------|
| `pre-compact` (NEW) | PreCompact | Read guardrails + state.yaml, inject compact reminder into context | blueprints/01 (section 3) |
| `user-prompt` (NEW) | UserPromptSubmit | Classify prompt (correction/continuation/clarification/new), log turn event, detect revision_requested | blueprints/01 (FTR), ideas/07 |
| `session-start` (REVISE) | SessionStart | Add guardrails loading, command awareness, tool preference reminder | blueprints/01 (section 3) |

Wire existing pre-tool and post-tool hooks into hooks.json (currently defined but not registered).

---

## Skills to retire (11)

After replacement commands are tested and working.

| Skill | Absorbed into | Traces to |
|-------|---------------|-----------|
| `working-smarter` | `build.md` | analysis/01 |
| `zero-errors-policy` | `build.md`, `commit.md` | analysis/01 |
| `decomposing-broad-tasks` | `plan.md` | analysis/01 |
| `design` | `blueprint.md` | analysis/01 |
| `session-management` | `session.md` + phase commands | analysis/01 |
| `context-efficiency` | phase commands (auto-call recommend_next) | analysis/01 |
| `pattern-based-development` | `patterns.md` + `build.md` locate step | analysis/01 |
| `identifying-patterns` | `pattern-extract` command | analysis/01 |
| `detecting-doc-drift` | `validate.md` | analysis/01 |
| `auditing-skill-descriptions` | retired (no replacement needed) | analysis/01 |
| `reformatting-docs` | retired (one-time cleanup) | analysis/01 |

---

## Testing commands and skills

### Using skill-creator eval framework

The `skill-creator` plugin provides a test-prompt → eval → iterate loop:

1. **Write test prompts**: scenarios that exercise the command
2. **Run claude-with-skill on each prompt**: produces outputs
3. **Grade outputs**: quantitative (did the right file get created? correct frontmatter?) + qualitative (user review)
4. **Iterate**: revise command markdown based on eval results

### Example test prompts for `/sensei:build`

```yaml
test_prompts:
  - id: build-with-pattern
    prompt: "/sensei:build — work on issue #1: add SQL adapter"
    setup:
      - create .sensei/rules.md with "use adapter pattern for parsers"
      - create PATTERNS.md with adapter pattern entry
      - create .sensei/state.yaml with active_plan
    assertions:
      - AI calls search() or get_patterns() before writing code
      - AI calls match_pattern() and finds adapter pattern
      - AI proposes decomposition before implementing
      - AI writes tests before implementation
      - AI presents tests for user approval
      - Created file follows adapter pattern (implements trait, registered)
      - state.yaml updated with active_issue
      - log_event called with type=locate

  - id: build-no-pattern
    prompt: "/sensei:build — work on issue #2: add config file validation"
    setup:
      - no matching pattern in PATTERNS.md
    assertions:
      - AI calls match_pattern(), gets no result
      - AI proceeds without pattern constraint
      - AI still decomposes and writes tests first
      - AI does NOT invent a non-existent pattern

  - id: build-tdd-approval
    prompt: "/sensei:build — implement the health check endpoint"
    assertions:
      - AI writes tests FIRST
      - AI PRESENTS tests to user before implementing
      - AI waits for approval (or proceeds with notice in auto-mode)
      - Tests are meaningful (not trivially passing)
```

### Example test prompts for `/sensei:refocus`

```yaml
test_prompts:
  - id: refocus-after-drift
    prompt: "/sensei:refocus"
    setup:
      - .sensei/state.yaml with active_phase=build, active_task="implement SqlAdapter"
      - conversation has drifted to discussing unrelated refactoring
    assertions:
      - AI reads state.yaml
      - AI outputs current phase, task, issue
      - AI acknowledges the drift
      - AI re-anchors to the active task
```
