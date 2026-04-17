---
name: Project Guardrails
updated: 2026-04-17
---

# Guardrails

## Developer mindset

Before writing any code or creating any file, answer these questions:

1. **Where does this run?** — Which process, which machine, which path? A plugin file runs from `${CLAUDE_PLUGIN_ROOT}`, not from the repo. A daemon endpoint runs in senseid. A hook runs in bash with no MCP access.
2. **Who reads this?** — The AI? A hook script? The desktop? The daemon? Each has different access patterns.
3. **How does it get there?** — Is it installed via plugin? Built by cargo? Copied by a script? If the answer is unclear, the file is in the wrong place.
4. **What happens when it's missing?** — Graceful degradation or hard failure? What's the user experience?
5. **How do I verify it works?** — What test proves this is correct? If you can't describe the test, you don't understand the implementation well enough.

When in doubt, ask. Do not assume. A question costs one turn. A wrong assumption costs a rewrite.

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
- **Ask before assuming** — when information is incomplete, ask for clarity rather than proceeding
- **Trace the runtime flow** — before implementing, walk through: who triggers this → what reads it → where does output go → how is it verified
- **One issue at a time** — pick from backlog, complete it, verify it, close it, then move on
- **Log events** — every command calls log_event() (MANDATORY, not optional)
