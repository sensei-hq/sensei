---
name: context-manager
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
