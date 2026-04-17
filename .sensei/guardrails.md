---
name: Project Guardrails
updated: 2026-04-17
---

# Guardrails

## Analyst mindset

Before designing or building anything, understand the problem:

1. **What problem are we solving?** — State the problem in the user's words, not technical terms. If you can't explain it simply, you don't understand it yet.
2. **Who benefits and how?** — Which user persona? What changes for them? What's the before/after?
3. **What are the constraints?** — Budget, time, technical limitations, dependencies. What's off the table?
4. **What are the acceptance criteria?** — How does the user know this is done? Not "tests pass" — what does the user observe?
5. **What are the edge cases?** — What happens with empty input, missing data, concurrent access, first-time use, migration from prior state?
6. **What are we NOT building?** — Scope boundaries prevent creep. Explicitly state what's out of scope.

If requirements are unclear, surface the ambiguity. Do not fill gaps with assumptions — ask.

## Developer mindset

Before writing any code or creating any file, answer these questions:

1. **Where does this run?** — Which process, which machine, which path? A plugin file runs from `${CLAUDE_PLUGIN_ROOT}`, not from the repo. A daemon endpoint runs in senseid. A hook runs in bash with no MCP access.
2. **Who reads this?** — The AI? A hook script? The desktop? The daemon? Each has different access patterns.
3. **How does it get there?** — Is it installed via plugin? Built by cargo? Copied by a script? If the answer is unclear, the file is in the wrong place.
4. **What happens when it's missing?** — Graceful degradation or hard failure? What's the user experience?
5. **How do I verify it works?** — What test proves this is correct? If you can't describe the test, you don't understand the implementation well enough.

When in doubt, ask. Do not assume. A question costs one turn. A wrong assumption costs a rewrite.

## Business acceptance tester mindset

After implementation, verify from the user's perspective — not just "does the code work" but "does this deliver value":

1. **Walk the user journey** — Start from the trigger (user types a command, session starts, context compacts). Follow every step the user would experience. Does it flow naturally?
2. **Test the happy path end-to-end** — Not unit by unit. The full flow: input → processing → output → side effects. Does the user see the right result?
3. **Test the first-time experience** — No config exists, no state file, no guardrails, no prior sessions. What happens? Is it helpful or confusing?
4. **Test the failure path** — Daemon is down, MCP disconnected, file missing, permissions wrong. Does the user get a clear message or a silent failure?
5. **Test the correction path** — User says "that's wrong." Does the system learn? Is the correction captured? Will it be different next time?
6. **Verify against acceptance criteria** — Go back to the issue. Read each criterion. Is it met? Not "probably" — demonstrate it.
7. **Check for regressions** — Did this change break something that was working? Run the full suite, not just the new tests.

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
