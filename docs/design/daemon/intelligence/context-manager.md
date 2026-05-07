---
id: context-manager
type: design
status: partial
implements:
  - feature: context
    items: [slice-loading, token-budget, checkpoints, recommend-next]
---

> **PARTIALLY IMPLEMENTED.** `load_context()` and `context_pack()` exist in `packages/server/src/mcp-server.ts` and work. The budget-aware context graph and `recommend_next()` prescription described here are not implemented — context loading is simpler: it builds slices from the symbol map and chunk DB on each call without a persistent graph structure. The token budget targets and slice definitions below remain accurate for the current implementation.

# Context Manager

## Overview

The Context Manager exposes targeted slices of the indexed repo — orientation, module exports, patterns — each sized to fit within a defined token budget so agents never load more than the task requires. It tracks named checkpoints as task-boundary markers, and `recommend_next()` maps a task description to the minimal context prescription (which scope, which resolution level) before any loading happens.

---

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| token-efficiency | Orientation slice must be under 250 tokens; module slice under 150 tokens for a typical module |
| reliability | load_context() must return a valid response for any indexed path, even if empty |
| usability | recommend_next() prescription must be actionable in one sentence — no ambiguous output |
| scalability | get_context_summary() must enumerate all available scopes in under 1s for a 500-file repo |

---

## Slice Definitions

| Scope | Content | Source | Token target |
|-------|---------|--------|-------------|
| `"orientation"` | project name, description, stack, entry points | `sensei.repos` query | < 250 |
| `"src/<module>"` | L0 signatures for all exports in that module | `sensei.symbols` table query | < 150 |
| `"patterns"` | `.sensei/patterns.md` full content | on-disk file | varies |
| `"shortcuts"` | `.sensei/shortcuts.md` full content | on-disk file | varies |

Resolution levels used:
- L0 — signatures only (function names, types, no body)
- L1 — signatures + docstrings
- L2 — signatures + logic flow notation
- L3 — full source

---

## Checkpoint Storage

Checkpoints are stored in `sensei.events` (event_type: 'checkpoint'). In-session checkpoints are also kept in memory as an ordered list for fast access:

```typescript
interface Checkpoint {
  name: string;         // "auth-task-done" or generated timestamp
  timestamp: string;    // ISO 8601
  note?: string;        // optional annotation
}
```

---

## Algorithm / Flow

`recommend_next(task: string)`:

```
Step 1: Classify task type from description:
  → contains "list", "find", "what are" → discovery → L0
  → contains "explain", "understand", "how does" → understanding → L1/L2
  → contains "fix", "edit", "change", "implement" → edit → L3 for target, L0 for adjacent
  → unclear / broad → orientation → get_llmspec()
Step 2: Identify target scope from task (module path, symbol name)
Step 3: Return prescription as structured object
```

---

## API / Tool Contracts

```typescript
// MCP tools
load_context(scope: string, level?: 'L0' | 'L1' | 'L2' | 'L3'): ContextSlice
// scope: "orientation" | "patterns" | "shortcuts" | "src/<module>"
// Returns: content string + token estimate
// Default level: L0 for module scopes, N/A for named scopes

get_context_summary(): ScopeSummary[]
// Returns list of { scope, description, estimatedTokens } for all available scopes

checkpoint(name?: string): CheckpointResult
// Saves task boundary marker; name defaults to ISO timestamp
// Returns: { name, timestamp }

recommend_next(task: string): ContextPrescription
// Returns: { taskType, prescription: [{ tool, scope, level }], rationale }
```

---

## Error Handling

```
Unknown scope (load_context):     "No indexed content found for scope '<scope>'. Run reindex_repo() first."
Empty module (load_context):      Return empty content + 0 token estimate, no error
Unrecognized task (recommend_next): Default to orientation prescription, include rationale: "Task unclear — defaulting to orientation"
```

---

## Testing Strategy

```
Unit: src/context/context-manager.spec.ts
  - load_context("orientation") returns under 250 tokens
  - load_context("src/nonexistent") returns empty, no error
  - recommend_next("list exports") → L0 discovery prescription
  - recommend_next("fix bug") → L3 edit prescription
  - checkpoint() generates timestamp name when name omitted

E2E: e2e/context.e2e.ts
  - full index → load_context("orientation") → verify token budget
  - recommend_next → load prescribed scope → verify content matches prescription
```

---

## Open Questions

| Question | Status |
|----------|--------|
| Should checkpoints optionally persist to disk for long-running sessions? | Resolved — checkpoints persist to `sensei.events` (event_type: 'checkpoint') |

---

> This is a **design doc** — how it works, not what it does.
> User-facing needs belong in `docs/features/`.
> Status lives in `docs/traceability.yaml` — do not add a status table here.
