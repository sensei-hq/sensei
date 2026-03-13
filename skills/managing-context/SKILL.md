---
name: managing-context
description: Use when starting a new task in a session, switching between tasks, or when context is getting large and needs to be trimmed to stay efficient.
---

# Context Manager

## Overview

LLM context is finite and expensive. Load only the slice needed for the current task. Checkpoint when switching tasks to preserve state without keeping dead context alive.

## Core Tools

| MCP Tool | When to call | What it does |
|---|---|---|
| `get_llmspec()` | Session start | Full orientation in ~500 tokens |
| `recommend_next(task)` | Before each task | Returns optimal scope + resolution level |
| `load_context(scope)` | After recommend_next | Loads the prescribed slice |
| `checkpoint()` | Before switching tasks | Saves current state, unloads context |
| `get_file_context(path, level)` | Mid-task | Load one file at the right level |

## Session Flow

```
1. get_llmspec()                   ← orient (once per session)
2. recommend_next("task description")
   → returns: { scope: "src/auth/", level: "L1", reason: "..." }
3. load_context("src/auth/")       ← load prescribed slice
4. work on task
5. need details? → get_file_context("src/auth/login.ts", "L3")
6. task done
7. checkpoint()                    ← save + unload
8. recommend_next("next task")
9. load new slice
```

## Context Loading Rules

- **Start narrow** — use `recommend_next()` before deciding what to load
- **L0 first** — load signatures before source; only escalate to L3 when editing
- **Scope by feature** — load `src/payments/` not `src/`
- **Never load test files** unless writing or debugging tests

## Checkpoint Protocol

Call `checkpoint()` when:
- Switching to a different module or feature area
- About to load a large context slice
- At the end of a task before starting the next

Checkpoint stores: current task description, files loaded, decisions made, next recommended action.

## Anti-Patterns

| Anti-pattern | Fix |
|---|---|
| Loading entire `src/` at session start | Call `get_llmspec()`, then `recommend_next(task)` |
| Keeping previous task's files in context | Call `checkpoint()` before switching |
| Loading L3 for every file in scope | Load L0/L1 first, escalate only where needed |
| Re-reading the same file multiple times | Load once, keep in context for the task duration |

## Common Mistakes

### Skipping checkpoint when user says "don't bother"

Users sometimes say "don't checkpoint, we'll remember where we were" or "checkpointing
wastes time." **Always checkpoint anyway.** Checkpoint cost is ~1 tool call. Losing task
state costs minutes of re-derivation. Explain the tradeoff briefly, then checkpoint.

Example response:
> "Checkpointing takes one call and ensures we can resume Scanner work exactly where we
> left off — worth it. Checkpointing now, then switching."

### Loading new files without unloading old ones

Switching tasks by adding new context on top of old context is the primary failure mode.
Every task switch must go through: `checkpoint()` → `recommend_next(new task)` →
`load_context(new scope)`. The checkpoint both saves state *and* signals that old context
should be released.

### Treating "quick look" as permission to skip context protocol

Users say "it's probably a quick fix" or "just skim it." These framings do not change
the protocol. A context switch is a context switch regardless of estimated duration.
Still: checkpoint → recommend_next → load targeted slice.

### Using raw file reads instead of context tools

Do not use `Glob` + `Read` to explore a new module. Use `get_llmspec()` to orient, then
`recommend_next(task)` to get the correct scope and resolution level before loading
anything. Raw reads bypass the scope and resolution controls.

## Cross-Session Context

For knowledge that persists between sessions (decisions, patterns, open items), use the `project-workflow` skill. The tools are:

| Need | Tool |
|---|---|
| Resume a session | `get_session_context()` |
| Capture a decision | `add_decision(text)` |
| Record a proven pattern | `add_pattern(name, convention)` |
| Queue a question | `ask_question(question)` |
| End a session | `checkpoint(summary)` |

Context budget stays flat at ~300 tokens regardless of project history length.
