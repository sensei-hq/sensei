---
name: compressing-content
description: Use when working with code in LLM context and needing to reduce token usage, choose representation levels, or compress code for agent consumption without losing reasoning ability.
---

# Content Compression

## Overview

Code has four resolution levels. Serve the minimal level that lets the LLM complete its task. Docstrings and doc-comments waste tokens — LLMs infer meaning from signatures.

## Resolution Levels

| Level | Format | ~Tokens | Use when |
|---|---|---|---|
| L0 — Signature | `processOrder(orderId: string): Promise<Order>` | 10 | Discovery, listing available functions |
| L1 — IO Pattern | `order = processOrder(orderId)` + input/output types | 30 | Understanding what a function does |
| L2 — Logic Flow | Bullet steps or pseudocode | 80 | Understanding how it works |
| L3 — Full Source | Actual code | 200–2000 | Editing, debugging, modifying |

## Task → Level Mapping

```
"list available functions in auth module"   → L0
"what does login() return?"                 → L1
"trace the auth flow"                       → L2
"fix a bug in validateToken()"              → L3
```

## Compression Rules

**Always strip:**
- Docstrings and doc-comments (repeat what signatures already say)
- Import statements at L0/L1/L2
- Type boilerplate (generics, decorators) at L0/L1
- Dead code, commented-out blocks

**L2 logic flow notation:**
- `if/else` → indented bullets: `if valid → proceed`, `else → throw error`
- Loops → `for each item → transform and collect`
- Pipelines → `raw → parse() → validate() → transform() → output`
- Async → `await fetch → check status → parse body`

**L1 IO pattern notation:**
```
result = functionName(input)
// input: { id: string, options?: Config }
// result: Promise<{ data: T, error?: string }>
```

**State machine shorthand (any level):**
```
idle → loading → success
              ↘ error → retry → loading
```

## Common Mistakes

| Mistake | Fix |
|---|---|
| Loading L3 to understand what a function does | Use L1 instead — 98% token reduction |
| Keeping docstrings at L0/L1/L2 | Strip them — they duplicate signature info |
| Loading full file to find one function | Use `get_file_context(path, "L0")` to list, then `L3` for the specific function |
| Summarising code yourself | Call `get_file_context(path, level)` MCP tool — consistent, cached, zero tokens spent |
