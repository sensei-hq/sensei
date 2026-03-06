---
name: project-workflow
description: Use when starting work on any project to establish the session protocol, capture decisions and patterns, and maintain knowledge across sessions without token bloat.
---

# Project Workflow

## Overview

Process-centric protocol for working on a project across sessions. Knowledge persists via MCP tools — agents never read or write memory files directly. Brainstorming, planning, and TDD are handled by superpowers plugins; this skill covers what survives between sessions.

## Session Start

```
1. call: get_session_context()    ← memory + open items + active plan (~300 tokens)
2. review open items              ← anything needing resolution before starting?
3. call: recommend_next(task)     ← get context prescription for first task
```

## During Work

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

## Session End

```
call: checkpoint(
  summary: "1-3 sentences: what was done, key decisions, next steps",
  decisions: ["any decisions not yet captured via add_decision"],
  patterns: ["any patterns not yet captured via add_pattern"]
)
```

The MCP tool handles archiving. Next session resumes with `get_session_context()`.

## Quality Gate (before closing a task)

- [ ] All tests pass
- [ ] Lint: 0 errors
- [ ] Decisions captured via `add_decision()`
- [ ] Open questions queued via `ask_question()`

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

## Crash Recovery

If a session ends without a checkpoint (crash, timeout, interrupt):

```
1. call: get_session_context()    ← loads last archived checkpoint
2. review what was in progress
3. call: ask_question("Verify: was <last action> completed?")
4. resume from last known good state
```

The checkpoint archive in `.index/checkpoints/sessions/` preserves the last known state.
