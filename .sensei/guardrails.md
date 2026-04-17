---
name: Project Guardrails — sensei-dev
updated: 2026-04-17
mindsets: marketplace/templates/mindsets.md
---

# Guardrails

> Mindsets (analyst, developer, BAT) are in `marketplace/templates/mindsets.md` — loaded by session-start hook. This file contains project-specific rules.

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

## Tools

- **Prefer sensei MCP** over grep/sed for symbol lookup
- **Use get_callers()/get_callees()** for dependency analysis, not manual file reading
- **Use match_pattern()** before creating new structures — check if a pattern exists first
- **Use get_lib_docs()** before writing code that uses a library — don't guess from training data

## Process

- **Design before code** — brainstorm/analyze/blueprint THEN build (D18)
- **Ask before assuming** — when information is incomplete, ask for clarity rather than proceeding. Keep questions small: 3-4 simple questions max, 1 complex question at a time. Make it a conversation, not a survey.
- **Trace the runtime flow** — before implementing, walk through: who triggers this → what reads it → where does output go → how is it verified
- **One issue at a time** — pick from backlog, complete it, verify it, close it, then move on
- **Log events** — every command calls log_event() (MANDATORY, not optional)
