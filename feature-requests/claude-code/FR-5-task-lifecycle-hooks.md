---
target: anthropics/claude-code
status: draft
relates-to: docs/ideas/24a-observatory-data-audit.md
---

# FR-5: Task-level lifecycle hooks (PreTask / PostTask)

## Summary

Current hooks operate at session level (SessionStart) or tool level (PreToolUse/PostToolUse). There's nothing in between — no signal when a logical unit of work starts or ends within a session.

A single session often contains multiple tasks: "fix the bug", "add tests", "refactor the helper". Plugins need to know when each task begins and ends to compute per-task metrics (turns, tokens, cost, outcome).

## Current hook granularity

```
Session level:    SessionStart ──────────────────────── (no SessionEnd)
                       │
Task level:        (nothing)                            ← THE GAP
                       │
Turn level:        UserPromptSubmit ─── UserPromptSubmit ─── ...
                       │                      │
Tool level:     Pre/PostToolUse         Pre/PostToolUse
```

## What's needed

Two new hook events:

### TaskStart
Fires when Claude begins working on a distinct task (user gives a new instruction, switches topic, or explicitly starts a new task).

```json
{
  "event": "TaskStart",
  "task_id": "...",
  "session_id": "...",
  "prompt_preview": "Fix the null pointer in parser.rs",
  "timestamp": "2026-04-19T10:30:00Z"
}
```

### TaskEnd
Fires when a task reaches a natural conclusion (Claude reports completion, user moves to next task, or session ends).

```json
{
  "event": "TaskEnd",
  "task_id": "...",
  "session_id": "...",
  "outcome": "completed",
  "turn_count": 8,
  "tool_calls": 5,
  "tokens_in": 24000,
  "tokens_out": 8000,
  "duration_seconds": 180,
  "timestamp": "2026-04-19T10:33:00Z"
}
```

## Why this matters

**Per-task cost attribution:** "Fixing the bug cost $0.35 and took 8 turns. Adding tests cost $0.12 and took 3 turns."

**Quality tracking:** FTR is meaningless at session level if a session has 5 tasks. It needs to be per-task: "3 of 5 tasks completed first-try."

**Identifying expensive patterns:** "Refactoring tasks average 15 turns. Bug fixes average 6. Where's the waste in refactoring?"

**Action recipes:** "Tasks that start with an Analyst pass complete 22% faster. Tasks that skip it have 3x more corrections."

## What defines a "task"?

This is the hard part. Possible heuristics:
- User explicitly labels it: "Now fix the login bug" → new task
- Significant topic shift detected in user prompt
- User invokes a workflow command (/build, /review, /plan) → new task boundary
- Plugin signals it via a hook response (e.g., sensei's session-start hook declares a task)

A pragmatic first step: let plugins declare task boundaries via a hook response field, and fire TaskEnd when the next TaskStart occurs or the session ends.

## Alternatives

- **Just use SessionEnd:** Too coarse — a session with 5 tasks gets one aggregate metric
- **Infer from events:** Fragile — reconstructing task boundaries from turn timestamps is guesswork
- **Let plugins define it:** This is what we're proposing, but with ACP-level support for the lifecycle signals
