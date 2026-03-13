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

## Storage

All project memory is stored in the `sensei.events` table in Supabase:

| `event_type` | `payload` fields | Purpose |
|---|---|---|
| `'decision'` | `{ text, id }` | Individual architectural decisions |
| `'pattern'` | `{ name, convention, uses, added }` | Proven coding patterns |
| `'open-item'` | `{ id, question, status, resolution? }` | Unresolved questions + next steps |
| `'checkpoint'` | `{ summary, decisions_added, patterns_added, items_closed, items_opened }` | Session-end distillation |
| `'session'` | `{ date, summary }` | Session archive (not loaded on resume) |

The `.sensei/patterns.md` file remains on disk as a human-readable view of current patterns, regenerated from `sensei.events` on each checkpoint. It can be manually edited — manual edits are preserved on re-generation.

---

## Distillation Protocol

Distillation is **LLM-driven** — the agent summarises what happened, then calls `checkpoint(summary)`. The MCP tool handles the storage mechanics.

```
Agent (session end):
  1. Summarise in 1-3 sentences: what was done, decisions made, what's next
  2. Call checkpoint(summary)

MCP checkpoint() tool:
  1. Parse summary for: decisions, patterns, open items, next steps
  2. Upsert decisions into `sensei.events` (dedup by semantic similarity of text)
  3. Upsert patterns into `sensei.events`; regenerate `.sensei/patterns.md`
  4. Update open-item events in Supabase (close resolved, add new)
  5. Insert session archive event into `sensei.events` (event_type: 'session')
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
  1. Query `sensei.events` for recent decisions and context
  2. Query `sensei.events` for open-item events with status: open
  3. Load active plan incomplete steps (stored in `sensei.events` with event_type: 'plan')
  4. Format as structured summary
  5. Return ~300 tokens regardless of project age
```

**Context budget stays flat:** Only recent decisions, open-item events, and the active plan are queried. Archived sessions are never loaded on resume — they exist for audit only.

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

// Storage: queries sensei.events for decisions, open-items, and plan events
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
// tool handles: upsert to sensei.events, archive session event, update open-item events
```

### `add_decision(text)`

```typescript
input: { text: string }
output: string  // "Decision recorded."

// Inserts decision event into sensei.events
// Deduplication: if semantically equivalent decision exists, updates timestamp only
// No read required by the agent — append-only
```

### `add_pattern(name, convention)`

```typescript
input: { name: string, convention: string }
output: string  // "Pattern recorded."

// Upserts pattern event into sensei.events
// Increments uses counter if pattern already exists
```

### `ask_question(question)`

```typescript
input: { question: string }
output: string  // "Question queued. ID: <id>"

// Inserts open-item event into sensei.events with status: open
// Non-blocking — agent continues without waiting for answer
```

### `get_open_items()`

```typescript
input: {}
output: string  // formatted list of open questions + next steps

// Queries sensei.events for open-item events with status: open
// Included automatically in get_session_context() — call directly for mid-session check
```

### `close_item(id)`

```typescript
input: { id: string, resolution?: string }
output: string  // "Item closed."

// Updates open-item event in sensei.events (sets status: resolved)
// Optional resolution text stored in the event payload
```

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
Session 10:  get_session_context() → 300 tokens  (sensei.events queried, deduped)
Session 100: get_session_context() → 300 tokens  (same — archives not loaded)
```

The budget is bounded because:
- Decisions in `sensei.events` are deduped (not an append log) — deduplication keeps results small
- Only the last 2 session summaries are in active memory; the rest are archived
- Open-item events shrink as items are closed
