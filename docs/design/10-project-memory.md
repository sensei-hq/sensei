---
id: project-memory
type: design
implements:
  - feature: workflow
    items: [session-resume, decision-capture, pattern-capture, session-checkpoint, open-items]
  - feature: context
    items: [checkpoint-restore]
---

# Project Memory

## Overview

Project memory is the cross-session knowledge layer. It survives between sessions, stays bounded in size, and is managed entirely by MCP tools — agents never read or write memory files directly. The core principle: **distil at session end, load compressed at session start**. Context budget stays flat regardless of project age.

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| reliability | No decision or pattern must be silently lost during checkpoint |
| token-efficiency | `get_session_context()` must stay under 400 tokens regardless of history length |
| maintainability | Checkpoint format must be human-readable YAML, not binary |

---

## Storage Layout

```
<repo-root>/
  .index/
    checkpoints/
      memory.yaml         ← distilled project knowledge (decisions, context)
      patterns.yaml       ← proven patterns (name + convention, no examples)
      open-items.yaml     ← unresolved questions + next steps
      sessions/
        2026-03-06.yaml   ← archived session snapshot (not loaded on resume)
        2026-03-05.yaml
        ...
```

---

## File Schemas

### `memory.yaml`

```yaml
version: 1
updated: "2026-03-06"
decisions:
  - id: "repo-pattern"
    text: "Use repository pattern for all DB access"
    date: "2026-03-01"
  - id: "zod-validation"
    text: "All external inputs validated with Zod at boundary"
    date: "2026-03-03"
context:
  project: "strategos"
  stack: [typescript, bun, postgres]
  phase: "Phase 2 — entity cleanup"
```

### `patterns.yaml`

```yaml
version: 1
patterns:
  - name: "data-attribute DOM"
    convention: "data-{component} on root, data-{component}-{part} on children"
    uses: 4
    added: "2026-02-15"
  - name: "navigator-wrapper-proxy"
    convention: "ProxyItem (read) → Wrapper (state) → Navigator (DOM events)"
    uses: 7
    added: "2026-01-20"
```

### `open-items.yaml`

```yaml
version: 1
items:
  - id: "lock-strategy"
    question: "Should we use optimistic locking or row versioning?"
    added: "2026-03-06"
    status: open
  - id: "cache-ttl"
    question: "What TTL for the MCP response cache?"
    added: "2026-03-05"
    status: open
```

### `sessions/<date>.yaml` (archived)

```yaml
date: "2026-03-06"
summary: "Added POST /users endpoint. Tests pass. Next: validation middleware."
decisions_added: ["zod-validation"]
patterns_added: []
items_closed: []
items_opened: ["lock-strategy"]
```

---

## Distillation Protocol

Distillation is **LLM-driven** — the agent summarises what happened, then calls `checkpoint(summary)`. The MCP tool handles the storage mechanics.

```
Agent (session end):
  1. Summarise in 1-3 sentences: what was done, decisions made, what's next
  2. Call checkpoint(summary)

MCP checkpoint() tool:
  1. Parse summary for: decisions, patterns, open items, next steps
  2. Merge decisions → memory.yaml (dedup by semantic similarity of text)
  3. Merge patterns → patterns.yaml
  4. Update open-items.yaml (close resolved, add new)
  5. Write session archive → sessions/<date>.yaml
  6. Return: "Checkpointed. Resume with get_session_context()."
```

**Why LLM distillation over tool-based:** The LLM understands *what matters*. "After 3 hours of discussion we settled on repository pattern" compresses to one decision line. A dumb file appender can't do this — it accumulates noise.

**Cost:** ~200 tokens at session end.
**Saving:** ~1,200 tokens per future session start. Break-even after one session.

---

## Session Resume Protocol

```
Agent (session start):
  1. Call get_session_context()

MCP get_session_context() tool:
  1. Load memory.yaml (compressed decisions + context)
  2. Load open-items.yaml (unresolved questions + next steps)
  3. Load active plan incomplete steps only (from .index/checkpoints/active-plan.yaml)
  4. Format as structured summary
  5. Return ~300 tokens regardless of project age
```

**Context budget stays flat:** Only `memory.yaml`, `open-items.yaml`, and the active plan are loaded. Archived sessions are never loaded on resume — they exist for audit only.

---

## MCP Tools

### `get_session_context()`

```typescript
input: {}
output: string  // structured summary: memory + open items + active plan steps

// Format:
// ## Project Memory
// Phase: Phase 2 — entity cleanup
// Decisions: [list of decision texts]
//
// ## Open Items
// - Should we use optimistic locking or row versioning?
//
// ## Active Plan (incomplete steps)
// - [ ] Step 3: Add validation middleware
// - [ ] Step 4: Write e2e test

// Token budget: ~300 tokens
// Falls back to get_llmspec() if no checkpoints exist
```

### `checkpoint(summary)`

```typescript
input: { summary: string, decisions?: string[], patterns?: string[] }
output: string  // "Checkpointed. Resume with get_session_context()."

// summary: 1-3 sentence agent-provided distillation
// decisions: optional explicit list (agent can extract from conversation)
// patterns: optional explicit list
// tool handles: merge to memory.yaml, archive session, update open-items
```

### `add_decision(text)`

```typescript
input: { text: string }
output: string  // "Decision recorded."

// Appends to memory.yaml decisions[]
// Deduplication: if semantically equivalent decision exists, updates timestamp only
// No read required by the agent — append-only
```

### `add_pattern(name, convention)`

```typescript
input: { name: string, convention: string }
output: string  // "Pattern recorded."

// Appends to patterns.yaml
// Increments uses counter if pattern already exists
```

### `ask_question(question)`

```typescript
input: { question: string }
output: string  // "Question queued. ID: <id>"

// Adds to open-items.yaml with status: open
// Non-blocking — agent continues without waiting for answer
```

### `get_open_items()`

```typescript
input: {}
output: string  // formatted list of open questions + next steps

// Returns only status: open items
// Included automatically in get_session_context() — call directly for mid-session check
```

### `close_item(id)`

```typescript
input: { id: string, resolution?: string }
output: string  // "Item closed."

// Marks item as resolved in open-items.yaml
// Optional resolution text stored alongside the closed item
```

---

## Migration from `agents/` Folder

`sensei migrate` converts manual agent files to the MCP-managed structure:

```
agents/memory.md        → .index/checkpoints/memory.yaml  (LLM-distilled)
agents/design-patterns.md → .index/checkpoints/patterns.yaml
agents/journal.md       → last entry extracted as open item / next step
agents/                 → archived to agents/_archived/   (not deleted)
CLAUDE.md               → regenerated referencing sensei workflow
```

The migration uses LLM distillation — it reads each source file and produces a compressed YAML equivalent, not a raw copy. The `agents/_archived/` folder is kept until the developer manually verifies parity and removes it.

---

## Plugin Compatibility

Project memory complements existing superpowers plugins — it does not replace them:

| Concern | Handled by |
|---|---|
| Design before implementation | `superpowers:brainstorming` |
| Writing implementation plans | `superpowers:writing-plans` |
| TDD, debugging, code review | superpowers plugin suite |
| **Knowledge persistence across sessions** | **project-memory MCP tools** |
| **Session resume (~300 tokens)** | **`get_session_context()`** |
| **Decision + pattern capture** | **`add_decision()`, `add_pattern()`** |

The `project-workflow` skill is a thin protocol that calls these tools at the right moments — session start, mid-task decisions, session end.

---

## Context Budget Over Time

```
Session 1:   get_session_context() → 300 tokens  (fallback to llmspec)
Session 10:  get_session_context() → 300 tokens  (memory.yaml compressed)
Session 100: get_session_context() → 300 tokens  (same — archives not loaded)
```

The budget is bounded because:
- `memory.yaml` is a *distilled* file (not an append log) — deduplication keeps it small
- Only the last 2 session summaries are in active memory; the rest are archived
- `open-items.yaml` shrinks as items are closed
