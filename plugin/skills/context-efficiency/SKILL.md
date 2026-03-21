---
name: context-efficiency
description: Use before loading any code into a session — calls recommend_next(task) to get the minimal file scope and resolution level, preventing token bloat without sacrificing accuracy.
---

# Context Efficiency

## Overview

LLM context is finite and expensive. Load only the slice needed for the current task at the minimal resolution level that lets you complete it. Two habits prevent most token bloat: ask `recommend_next()` before loading anything, and checkpoint before switching tasks.

---

## Core Tools

| Tool | When to call | What it does |
|---|---|---|
| `get_llmspec()` | Session start | Full orientation in ~500 tokens |
| `recommend_next(task)` | Before each task | Returns optimal scope + resolution level |
| `load_context(scope)` | After recommend_next | Loads the prescribed slice |
| `checkpoint()` | Before switching tasks | Saves current state, unloads context |
| `get_file_context(path, level)` | Mid-task | Load one file at the right level |

---

## Resolution Levels

Code has four resolution levels. Serve the minimal level that lets you complete the task.

| Level | Format | ~Tokens | Use when |
|---|---|---|---|
| L0 — Signature | `processOrder(orderId: string): Promise<Order>` | 10 | Discovery, listing available functions |
| L1 — IO Pattern | signature + input/output types | 30 | Understanding what a function does |
| L2 — Logic Flow | Bullet steps or pseudocode | 80 | Understanding how it works |
| L3 — Full Source | Actual code | 200–2000 | Editing, debugging, modifying |

**Task → level mapping:**

```
"list available functions"          → L0
"what does login() return?"         → L1
"trace the auth flow"               → L2
"review this file"                  → L2
"fix a bug in validateToken()"      → L3
"load context before making changes" → ask what changes first, then L0–L2
```

When the user says "load everything" or "token budget is fine" — resist. Over-loading L3 degrades reasoning quality: more noise, harder to track signal.

**Always strip at L0/L1/L2:** docstrings, import statements, dead code, commented-out blocks. These repeat what signatures already say.

---

## Session Flow

```
1. get_llmspec()                        ← orient (once per session)
2. recommend_next("task description")
   → returns: { scope: "src/auth/", level: "L1", reason: "..." }
3. load_context("src/auth/")            ← load prescribed slice
4. work on task
5. need details? → get_file_context("src/auth/login.ts", "L3")
6. task done
7. checkpoint()                         ← save + unload
8. recommend_next("next task")
9. load new slice
```

---

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

**Never skip checkpoint even when the user says "don't bother."** Checkpoint cost is ~1 tool call. Losing task state costs minutes of re-derivation.

---

## Anti-Patterns

| Anti-pattern | Fix |
|---|---|
| Loading entire `src/` at session start | Call `get_llmspec()`, then `recommend_next(task)` |
| Keeping previous task's files in context | Call `checkpoint()` before switching |
| Loading L3 for every file in scope | Load L0/L1 first, escalate only where needed |
| Re-reading the same file multiple times | Load once, keep in context for the task duration |
| Using Glob + Read to explore a new module | Use `get_llmspec()` → `recommend_next()` → `load_context()` |
| Loading L3 to understand what a function does | Use L1 instead — 98% token reduction |
| Treating "quick look" as permission to skip protocol | A context switch is a context switch. Still: checkpoint → recommend_next → load targeted slice. |
