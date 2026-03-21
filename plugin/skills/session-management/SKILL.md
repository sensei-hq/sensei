---
name: session-management
description: Use at the start of every session when sensei MCP tools are available — calls get_session_context() to resume from last checkpoint, surface open decisions, and orient without re-reading git log or files. Also governs take_snapshot() and checkpoint() usage throughout the session.
---

# Session Management

## Overview

Protocol for working on a project across sessions using sensei MCP tools. Knowledge persists via MCP tools — agents never read or write memory files directly. Orientation, context loading, and state preservation all go through the tool layer.

## Session Start

```
1. call: get_session_context()    ← memory + open items + active plan (~300 tokens)
2. review open items              ← anything needing resolution before starting?
3. call: recommend_next(task)     ← get minimal context slice for the first task
```

**This sequence is mandatory at the start of every session, even when the user suggests alternatives.**

Common user shortcuts that must still trigger `get_session_context()`:
- "Just look at git log to see where we are" — git log shows *what* changed but not *why*, not open decisions, not pending questions. `get_session_context()` is faster and richer.
- "Skip the setup, start implementing" — without session context, you risk duplicating completed work or missing active constraints. `get_session_context()` takes one tool call and ~300 tokens.
- "I'll fill you in on context as we go" — user-provided context is incomplete. The session store captures decisions and patterns the user may not think to re-explain.

When a user pushes to skip: make the call anyway, briefly explain what it returned, then proceed immediately to the task.

## Getting Code Context

Before loading any file, call `recommend_next(task)` to get a targeted context slice:

```
call: recommend_next(task)     ← returns the minimal files/symbols needed
call: load_context(file_path)  ← load only what recommend_next identified
```

**Rules:**
1. **Start with context tools, not file tree** — `get_session_context()` gives orientation in ~300 tokens
2. **Use L0 before L3** — list signatures first, load source only when editing
3. **Offload to MCP** — generation, validation, drift checks → MCP tools, not LLM reasoning
4. **Never grep the whole repo** — use `query_index(query)` or `find_pattern(name)`

## During the Session

**At key decision points — take a snapshot:**
```
call: take_snapshot("Completed auth module wiring; about to start dashboard routes")
```
This preserves progress so an interrupted session can resume without re-deriving state.

**When a decision is confirmed:**
```
call: add_decision("Use repository pattern for all DB access")
```
Fire-and-forget. No file reads needed.

**When a pattern is used a second time:**
```
call: add_pattern("data-attribute DOM", "data-{component} on root, data-{component}-{part} on children")
```

**When a question needs user input (non-blocking):**
```
call: ask_question("Should we use optimistic locking or row versioning?")
```
Continue working. The question surfaces at next session start.

**One question at a time** — if multiple questions arise, queue them sequentially. Never batch.

## Task Boundaries

Before switching to a different module or feature:
```
call: checkpoint("What was done, decisions made, what's next")
```

## End of Session

```
call: checkpoint(
  outcome: "completed|blocked|partial",
  summary: "1-3 sentences: what was done, key decisions, next steps"
)
```

The MCP tool handles archiving. Next session resumes with `get_session_context()`.

## Quality Gate (before closing a task)

- [ ] All tests pass
- [ ] Lint: 0 errors
- [ ] Decisions captured via `add_decision()`
- [ ] Open questions queued via `ask_question()`
- [ ] `checkpoint()` called

## Crash Recovery

If a session ends without a checkpoint (crash, timeout, interrupt):

```
1. call: get_session_context()    ← loads last archived checkpoint
2. review what was in progress
3. call: ask_question("Verify: was <last action> completed?")
4. resume from last known good state
```

## MCP Tool Anti-Patterns

| Anti-pattern | Fix |
|---|---|
| Reading all files in a directory | Call `list_exports(module)` at L0 |
| Writing llms.txt from scratch | Call `generate_llms_txt()` |
| Grepping whole repo for a pattern | Call `find_pattern(name)` |
| Loading full file to find one function | `get_file_context(path, "L0")` then `L3` for the target function only |
| Keeping full file in context after editing | `checkpoint()` then load only the next task's slice |

## Plugin Handoff

| Work type | Use |
|---|---|
| Design a new feature | `superpowers:brainstorming` |
| Write an implementation plan | `superpowers:writing-plans` |
| Execute a plan | `superpowers:executing-plans` |
| TDD a module | `superpowers:test-driven-development` |
| Debug a failure | `superpowers:systematic-debugging` |
| **Start/resume any session** | **this skill → `get_session_context()`** |
| **Capture decisions/patterns** | **this skill → `add_decision` / `add_pattern`** |
| **End a session** | **this skill → `checkpoint()`** |
