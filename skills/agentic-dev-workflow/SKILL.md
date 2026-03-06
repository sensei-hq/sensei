---
name: agentic-dev-workflow
description: Use when starting an agentic developer session, beginning a new task in a codebase, or when an agent is spending too many tokens on orientation, broad searches, or loading full files unnecessarily.
---

# Agentic Dev Workflow

## Overview

Orient narrow, work targeted, offload deterministic tasks to MCP. Never load full files when a slice will do. The goal: minimum tokens, maximum accuracy.

## Session Protocol

```
Start session
  → get_llmspec()                    ~500 tokens, full orientation
  → recommend_next(task)             get minimal context slice for the task
  → load_context(scope)              load only what's needed
  → work on task
  → need specifics? → query_index / get_file_context(path, "L3")
  → task done → checkpoint()
  → recommend_next(next_task)
  → load new slice
```

## Rules

1. **Start with llmspec, not file tree** — `get_llmspec()` gives orientation in ~500 tokens
2. **Use L0 before L3** — list signatures first, load source only when editing
3. **Offload to MCP** — generation, validation, drift checks → MCP tools, not LLM reasoning
4. **Never grep the whole repo** — use `query_index(query)` or `find_pattern(name)`
5. **Checkpoint before switching** — `checkpoint()` preserves state, unloads dead context

## Task Entry Checklist

Before starting any task:
- [ ] Is `.llmspec.yaml` loaded? If not, call `get_llmspec()`
- [ ] Called `recommend_next(task)` to get the minimal context slice?
- [ ] Using content-compression levels correctly? (see `content-compression` skill)

## Offload to MCP (not LLM context)

| Task | MCP tool | Why |
|---|---|---|
| Generate llms.txt | `generate_llms_txt()` | Consistent output, zero context tokens |
| Check doc drift | `check_drift()` | Deterministic file comparison |
| List all exports | `list_exports(module)` | Cached index lookup |
| Find usage pattern | `find_pattern(name)` | Pre-indexed, targeted result |
| Orient new session | `get_llmspec()` | Structured briefing in one call |

## Anti-Patterns

| Anti-pattern | Fix |
|---|---|
| Reading all files in a directory | Call `list_exports(module)` at L0 |
| Writing llms.txt from scratch | Call `generate_llms_txt()` |
| Grepping whole repo for a pattern | Call `find_pattern(name)` |
| Loading full file to find one function | `get_file_context(path, "L0")` then `L3` for the target function only |
| Keeping full file in context after editing | `checkpoint()` then load only the next task's slice |
