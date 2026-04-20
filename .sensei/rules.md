---
name: Project Rules — sensei-dev
updated: 2026-04-19
mindsets: .sensei/mindsets/
personas: .sensei/personas/
---

# Rules

## Mindsets and Personas (NON-NEGOTIABLE)

**Mindsets** (`.sensei/mindsets/`) define HOW to think. Apply the core three — Analyst → Developer → Acceptance Tester — in sequence on every task, no exceptions. If you find yourself writing code without an Analyst pass, stop. Apply specialist mindsets (UX, Security, Performance, DevOps) when the task touches their domain.

**Personas** (`.sensei/personas/`) define WHO we're building for. Each persona has questions, goals, and pain points. When designing or validating, put on each persona's hat and ask their questions. What's obvious to a developer may be opaque to an end-user. What's fine for a one-off script may be unacceptable for an API consumer.

Every mindset and persona file has a `## Questions` section. These questions are the core value — they catch the things you'd otherwise miss. Read them. Ask them. Answer them before moving on.

## Patterns

- **Adapter pattern** for all language parsers — implement the trait, register in mod.rs
- **Task worker pattern** — workers wrap testable pure functions, never put logic in the worker itself
- **Vertical feature slices** — features span all layers (daemon → MCP → command), implement innermost layer first, fully tested before building on top (D18)
- **Bottom-up implementation** — daemon first (with tests), then MCP (with tests), then hooks (with tests), then commands (with eval)

## Quality

- **TDD** — write tests first, present for approval, then implement
- **Zero errors** — `cargo test` must pass before and after every change
- **100% test coverage** on new code — no untested paths, no "I'll add tests later"
- **No half-baked implementations** — each layer feature-complete before building on top
- **No stubs** — if a dependency doesn't exist yet, build it first or don't start

## Architecture

- **AI calls MCP only** — never daemon HTTP endpoints directly
- **Hooks call daemon HTTP** — bash scripts, no MCP access, fire-and-forget
- **Desktop reads daemon HTTP** — never writes workflow state
- **Plugin files live in marketplace/** — not docs/, not project root
- **State.yaml is a cache** — daemon is source of truth, state.yaml synced for hook access
- **File paths are repo-relative** in the graph, never absolute

## Tools (MANDATORY — use MCP first, grep/glob as fallback only)

**Before ANY implementation task**, you MUST call:
1. `get_patterns()` — check existing patterns that apply to the change
2. `search()` — find related code through the graph, not grep

If MCP tools fail (server down), fall back to grep/glob and note the failure.

Available today:
- **`search(query)`** — symbol/function search via code graph (REPLACES grep for code nav)
- **`get_callers(name)`/`get_callees(name)`** — dependency tracing (REPLACES manual file reading)
- **`get_patterns(pattern)`** — find architectural patterns: adapter, worker, hook, route, component
- **`get_project_summary()`** — codebase stats, stack, structure
- **`get_communities()`** — architecture clusters
- **`get_lib_docs(name)`** — library docs before writing code that uses a library
- **`search_lib_docs(query)`** — search across all indexed library docs
- **`add_library(name)`** — index a library's docs when not yet available

Planned (not yet available):
- `match_pattern()` — structural pattern matching (#83)
- `log_event()` — workflow event logging (#80)
- `update_phase()` — workflow state management (#79)
- `get_workflow_state()` — orientation query (#79)

## Process

- **Design before code** — brainstorm/analyze/blueprint THEN build (D18)
- **Ask before assuming** — when information is incomplete, ask for clarity rather than proceeding. Keep questions small: 3-4 simple questions max, 1 complex question at a time. Make it a conversation, not a survey.
- **Trace the runtime flow** — before implementing, walk through: who triggers this → what reads it → where does output go → how is it verified
- **One issue at a time** — pick from backlog, complete it, verify it, close it, then move on
- **Log events** — when `log_event()` MCP tool is available, every command calls it (MANDATORY). Until implemented (#80), this rule is pending.
