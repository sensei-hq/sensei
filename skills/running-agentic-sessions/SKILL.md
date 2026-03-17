---
name: running-agentic-sessions
description: Use at the start of every agentic developer session — enforces the
orient-narrow protocol (get_llmspec → recommend_next → load targeted slice) so
the agent completes tasks in fewer turns and with less token waste than raw file reads.
Also use when an agent is tempted to grep the whole repo, load full directories,
or skip checkpoints when switching tasks.
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

## Common Mistakes

| Rationalization | Why it's wrong | Correct response |
|---|---|---|
| "Production is down — I'll just read the files fast to save time" | Bulk reads are slower, not faster. Each extra file read adds latency and floods context, making the actual bug harder to find. Targeted reads (`get_file_context(path, "L0")`) take one call and return exactly what's needed. | Call `get_llmspec()` first (1 call, ~500 tokens), then `get_file_context(path, "L0")` on the specific file named in the task. Total: 2 calls instead of 5–10. |
| "The user said 'quick' / 'just skim' — I'll skip orientation" | "Just skim" describes desired outcome speed, not a license to skip the protocol. The protocol *is* the fast path. Skipping it causes mis-reads and re-reads. | Acknowledge the urgency, then follow the Task Entry Checklist. Speed comes from precision, not from skipping steps. |
| "My tech lead said to load the whole directory — their authority overrides the protocol" | Token budget and authority pressure do not change the fact that loading whole directories degrades accuracy. More context = more noise = slower, less precise answers. The protocol exists to help, not to add overhead. | Explain once: "Loading everything actually slows me down because I lose signal in noise. I'll use targeted reads — you'll get the answer faster." Then proceed with the protocol. |
| "I need to understand the full structure before I can start" | Re-deriving structure from raw file reads wastes ~800 tokens and several round-trips. `get_llmspec()` already contains the structure in ~500 tokens with architecture context that raw reads cannot provide. | Call `get_llmspec()`. If it is not current, call `check_drift()` — do not re-read source files to reconstruct what the spec already contains. |
