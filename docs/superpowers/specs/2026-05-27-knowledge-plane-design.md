# Knowledge Plane (Phase 0) Design

**Date:** 2026-05-27
**Status:** Draft — awaiting user review
**Scope:** Build the AI-facing and human-facing interaction surface on top of sensei's existing memory schema. No new service. No remote sync. Phase 1+ (remote `sensei-knowledge` service, RBAC, governance) is named here but deliberately out of scope.

---

## Background

The Ethico AI-Native Engineering Operating Model spec (`/Users/Jerry/Work/Ethico/Ethico_AI_Operating_Model_Spec.md`) describes an org-wide layered knowledge plane: constitution → org → client → project → repo → local memories, with MCP tools, promotion ladders, RBAC, audit, and risk classes. Sensei already has most of the *data model* for this (multi-scope `memories`, evidence/examples/links, lifecycle status, an `insights` table designed for cross-machine sync) but lacks the *interaction surface*: write APIs, the assemble-context read path, the outcome feedback loop, and a UI for human review.

Phase 0 closes the interaction-surface gap inside the existing daemon. A future Phase 1 will extract a remote `sensei-knowledge` service for team mode; this design names that boundary so Phase 0 doesn't paint us into a corner.

## Vocabulary

We keep sensei's existing vocabulary; Ethico's terms are mapped to it.

| Ethico term | Sensei term | Notes |
|---|---|---|
| Learning / Rule | **Memory** | The `memories` table is the source of truth |
| Promotion (local→org) | (deferred) | Phase 1 — single-developer mode has no promotion ladder |
| `record_outcome` | **Memory outcome** | Per-memory event log (new `memory_outcomes` table — see Schema) |
| Constitution | (deferred) | Phase 1 — single-developer mode has no inviolable tier |
| Risk class | (deferred) | Phase 1 |
| `get_layered_context` | Same name | Returns blended `global + project + stack-matched` memories |
| `propose_promotion` | (deferred) | Phase 1 |

## Architecture

Everything lives in one new module inside the existing daemon: `crates/senseid/src/knowledge/`.

```
HTTP request                          MCP tool call
     │                                      │
     ▼                                      ▼
┌──────────────────────┐         ┌──────────────────────┐
│  /api/knowledge/*    │◀────────│  knowledge MCP tools │
│  handlers            │         │  (proxy to handlers) │
└─────────┬────────────┘         └──────────────────────┘
          │
          ▼
┌──────────────────────┐
│  knowledge module    │
│  - assemble_context  │   reads:  memories (filtered by scope)
│  - propose_memory    │           memory_evidence, memory_examples
│  - save_memory       │   writes: memories (status='proposed'/'active')
│  - accept_proposal   │           memory_outcomes
│  - reject_proposal   │           memory_evidence
│  - record_outcome    │
└─────────┬────────────┘
          │
          ▼
┌──────────────────────┐
│  Postgres (existing) │
│  - memories          │
│  - memory_outcomes   ◀── new (small table)
│  - memory_evidence   │
│  - memory_examples   │
│  - past_memories     │  (existing audit trigger keeps firing)
└──────────────────────┘
```

The Learnings page in the desktop app reads through the same `/api/knowledge/*` endpoints — no separate UI-only DB queries.

**Three properties Phase 0 must hold:**

1. **No new service.** Daemon-only. Phase 1 may extract; the design must not depend on extraction for Phase 0 to work.
2. **Additive schema.** Existing memory rows continue to function unchanged. Two enum values added; one small new table.
3. **Phase 1 sync hook preserved.** The `inference.insights` table is the future sync payload. Accept and outcome events are designed to make `insights` an append-only emit when Phase 1 lands.

## Schema delta

Three changes to `database/ddl`. Per project convention, DDL files are edited in place (full enum / table definition) and `dbd` handles applying the delta. No `ALTER` statements in DDL files.

### 1. `enum/sensei/memory_status.ddl` — add two values

```sql
create type memory_status
    as enum ('proposed', 'active', 'reinforced', 'challenged',
             'battle_tested', 'archived', 'rejected');
```

- `proposed` — AI auto-captured; awaiting user review in the triage queue. Never returned by `assemble_context`.
- `rejected` — User declined the proposal. Retained for audit and to prevent re-proposing the same lesson. Never returned by `assemble_context`.

### 2. `table/sensei/memories.ddl` — two new columns

```sql
,  tags             text[]    not null default '{}'
,  triage_signal    text                              -- null for explicit /save
```

Plus a GIN index on `tags`:

```sql
create index if not exists memories_tags_idx on memories using gin (tags);
```

`triage_signal` records which capture heuristic fired (`revert`, `correction`, `actually`, `repeat_pattern`, `override`, …). Surfaced in the triage UI so the user knows why the AI proposed it.

### 3. `table/sensei/memory_outcomes.ddl` — new table

```sql
set search_path to sensei, extensions;

create table if not exists memory_outcomes (
  id            uuid            primary key default gen_random_uuid()
, memory_id     uuid            not null references sensei.memories(id) on delete cascade
, session_id   uuid             references activity.sessions(id) on delete set null
, outcome      memory_outcome   not null
, context      text
, recorded_at  timestamptz      not null default now()
);

create index if not exists memory_outcomes_memory_id_idx
    on memory_outcomes(memory_id, recorded_at desc);
```

With a new enum:

```sql
-- enum/sensei/memory_outcome.ddl
create type memory_outcome
    as enum ('applied', 'consulted', 'violated', 'ignored');
```

And a trigger (function in `ddl/function/sensei/`) that runs `AFTER INSERT` on `memory_outcomes`:

- `applied` → `memories.reinforced_count += 1`, `strength = least(strength + 0.5, 5.0)`, `last_relevant_at = now()`. If `strength >= 4.0` and `violated_count = 0`, set `status = 'battle_tested'`. If status was `challenged`, status stays `challenged` until three consecutive `applied` outcomes have been recorded since the most recent `violated` outcome — at which point it returns to `reinforced`. The trigger computes "consecutive applied since last violation" by counting `memory_outcomes` rows for this memory with `outcome='applied'` and `recorded_at > (last violated event's recorded_at)`.
- `consulted` → `last_relevant_at = now()`. No strength change.
- `violated` → `memories.violated_count += 1`, `strength = greatest(strength - 0.7, 0.0)`, `status = 'challenged'`. If `strength < 1.0`, `status = 'archived'`.
- `ignored` → no-op on memories; recorded only for analytics.

This keeps the rest of the system unchanged: `past_memories` continues to capture before/after rows for every memory update via its existing trigger.

### Why not reuse `recommendations`

The `inference.recommendations` table is for **system-level imperatives** (`create_persona`, `promote_pattern`, `revise_rule`, `fix_anti_pattern`) with before/after FTR measurement at acceptance time. Memory outcomes are a different shape entirely: high-frequency per-memory event logs with no FTR measurement and no user-accept lifecycle. Forcing memory events through `recommendations` would muddle both. Keeping the tables distinct lets each evolve independently.

## API surface

All routes are under `/api/knowledge/` on the existing senseid HTTP server. JSON request/response. Each route has a matching MCP tool that proxies to it.

### Write — proposals (AI auto-capture)

```
POST /api/knowledge/proposals
  body: {
    project_id:     uuid,
    scope:          "global" | "project" | "stack",
    scope_filter:   string?,        // required when scope = "stack"
    type:           memory_type,    // existing enum
    title:          string,
    content:        string,
    impact:         string?,
    tags:           string[],
    triage_signal:  string,         // which heuristic fired
    evidence:       [{ url?, session_id?, note? }],
  }
  → { id, status: "proposed" }

POST /api/knowledge/proposals/:id/accept
  body: { edits?: { title?, content?, impact?, tags?, scope?, scope_filter? } }
  → { id, status: "active" }

POST /api/knowledge/proposals/:id/reject
  body: { reason?: string }
  → { id, status: "rejected" }
```

### Write — explicit save (user via /save)

```
POST /api/knowledge/memories
  body: same as POST /api/knowledge/proposals minus `triage_signal`
  → { id, status: "active" }
```

### Write — outcomes (AI reports per-session)

```
POST /api/knowledge/outcomes
  body: {
    outcomes: [{ memory_id, outcome, session_id?, context? }]
  }
  → {
      recorded: <count>,
      skipped:  [{ memory_id, reason }]   // empty array when all succeed
    }
```

Single endpoint, always batch (even for one outcome) — matches Ethico §5.3.8 and reduces chatter from AI sessions.

### Read — context assembly (the session-start call)

```
GET /api/knowledge/context?project_id=<uuid>&limit=<n>&tags=<csv>
  → {
      version:      string,         // hash of max(updated_at) per scope
      memories: [
        { id, scope, scope_filter, type, title, content, impact,
          strength, applied_count, violated_count, tags, updated_at }
      ],
      cache_until:  ISO8601         // 5 minutes from now
    }
```

Algorithm:

1. Resolve `project_id` → fetch `projects.detected_stack_ids` (existing column).
2. Three parallel queries against memories with `status IN ('active', 'reinforced', 'battle_tested', 'challenged')`:
   - `WHERE project_id = ?` (project scope; any value of `scope` column matches because the FK already pins it to this project)
   - `WHERE scope = 'stack' AND scope_filter = ANY(?)` (stack-matched)
   - `WHERE scope = 'global'` (global, project-id agnostic)
3. Merge with precedence: project > stack > global. Dedup by `id`.
4. Optional `tags` filter (`tags && ?` GIN op) applied post-merge.
5. Order by `strength DESC, last_relevant_at DESC NULLS LAST, modified_at DESC`.
6. Apply `limit` (default 200, cap 500).
7. `applied_count` and `violated_count` joined in from `memories.reinforced_count` / `violated_count` (no need to re-aggregate from `memory_outcomes` — the trigger keeps these counters live).

### Read — memory list (UI-facing)

```
GET /api/knowledge/memories?status=<status>&scope=<scope>&project_id=<uuid>&limit=<n>
  → { memories: [MemoryWithCounts] }
```

Same shape as context, but unfiltered by scope blending — used by the Learnings UI tabs. The existing `GET /api/projects/{id}/memories` endpoint stays for back-compat and is gradually deprecated by the UI switching to `/api/knowledge/memories`.

### Read — single memory detail

```
GET /api/knowledge/memories/:id
  → {
      memory:    MemoryWithCounts,
      evidence:  [{ url, note, recorded_at }],
      examples:  [{ node_id, is_good, is_bad, note }],
      outcomes:  [{ outcome, session_id, context, recorded_at }]   // last 20
    }
```

## MCP tool surface

Six new tools added to `crates/mcp/src/main.rs`. Each delegates to a daemon endpoint via the existing HTTP client. None of them block on long operations; all DB writes happen behind `spawn_blocking` in the daemon, following the gateway-router pattern.

| Tool | Endpoint | Caller |
|---|---|---|
| `propose_memory` | `POST /api/knowledge/proposals` | AI auto-capture |
| `save_memory` | `POST /api/knowledge/memories` | Explicit `/save` from human via assistant |
| `accept_proposal` | `POST /api/knowledge/proposals/:id/accept` | Either UI or `/accept` command |
| `reject_proposal` | `POST /api/knowledge/proposals/:id/reject` | Either UI or `/reject` command |
| `record_outcome` | `POST /api/knowledge/outcomes` | AI session — applied / consulted / violated / ignored |
| `get_layered_context` | `GET /api/knowledge/context` | AI session start, or any time refresh is needed |

Empty-string args are filtered before the request leaves the MCP tool (matches gateway tool conventions). Required-field validation lives in the daemon handler.

## Capture-trigger guidance

The daemon accepts any well-formed `propose_memory` call. The capture heuristics — when an AI assistant SHOULD call it — live in the sensei plugin's `CLAUDE.md` template and equivalent assistant instructions. Initial trigger list:

| Trigger | Description |
|---|---|
| `revert` | User reverted AI-suggested code within the same session |
| `correction` | User edited AI-suggested code and the edit was non-trivial (heuristic — git diff > 3 lines on the AI's output) |
| `actually` | User said "actually...", "no, we always...", "remember that...", "we never..." |
| `repeat_pattern` | The same kind of fix/edit happened in 2+ sessions in this repo |
| `override` | AI cited a memory and was overruled by the user — this fires `record_outcome(violated)` **and** `propose_memory` capturing the override |
| `test_failure` | Test failed on first run of generated code, and the user's fix is non-trivial |

These are guidance, not enforced. Server-side validation only checks that `triage_signal` is a non-empty string. The list will evolve as the marketplace plugin matures.

## Triage UI

Lives at `app/src/routes/(observatory)/learnings/`. Currently a stub rendering an empty `MemoryList` component — gets a full build-out. State managed by a singleton `memoryState.svelte.ts` following the wizard-state pattern.

### Layout

```
┌── /learnings ─────────────────────────────────────────────────┐
│  Tabs:  [Triage (3)]  [Active (47)]  [Archive]               │
│  Filters: scope ▾   tags ▾   search ▾                         │
│ ─────────────────────────────────────────────────────────────  │
│  ┌─ Triage list ──────────────┐   ┌─ Detail pane ──────────┐ │
│  │ ● scope=project signal=revert│   │ Title                  │ │
│  │   "wrap webhooks in idempotency │ │ Scope: project          │ │
│  │   keys at controller layer"   │ │ Tags: [security] [reliab] │ │
│  │   [Accept] [Edit] [Reject]   │   │ Signal: revert          │ │
│  │                              │   │ Evidence:               │ │
│  │ ● scope=stack/rust signal=… │   │  - PR #482 (link)       │ │
│  │   …                          │   │  - session 9af3…        │ │
│  │                              │   │ Outcomes panel (empty   │ │
│  │                              │   │  until accepted)        │ │
│  └──────────────────────────────┘   └──────────────────────────┘ │
└───────────────────────────────────────────────────────────────┘
```

### Tabs

| Tab | Status filter | Default landing |
|---|---|---|
| **Triage** | `proposed` | Yes if any proposals exist |
| **Active** | `active`, `reinforced`, `challenged`, `battle_tested` | Otherwise |
| **Archive** | `archived`, `rejected` | Never default |

### Per-row content (Triage tab)

- Scope chip (`global` / `project` / `stack:rust`)
- Title (one line, truncated)
- Triage-signal chip with icon (`revert` / `correction` / `actually` / …)
- Tag chips
- Three actions: **Accept**, **Edit & Accept**, **Reject**

### Per-row content (Active tab)

Same as Triage minus signal chip, plus an inline strength meter (0–5) and counts (applied N · violated M).

### Detail pane

Right pane, opens on row click. Shows full content, impact, evidence list with links, examples (linked via node_id where the indexer has nodes), outcomes panel (last 20 events with timestamps), and a strength-over-time sparkline (derived from `past_memories` audit history).

### Edit form (used by Edit & Accept and the inline edit action on Active)

Fields: scope (radio), scope_filter (visible only when scope=stack), type (select from `memory_type` enum), title, content, impact, tags (chip input).

## Outcome wiring

Memory outcomes drive the reinforcement loop. Wiring:

1. **AI session.** AI calls `get_layered_context` at session start → receives memory list.
2. **AI applies a memory.** When the AI generates code informed by a memory (e.g., cites it in a comment or makes a choice driven by it), it calls `record_outcome(memory_id, outcome='applied')` in a batch at end of turn.
3. **User overrules the AI.** If the user reverts/edits code that the AI applied a memory to, the assistant emits `record_outcome(memory_id, outcome='violated')` and also `propose_memory(...)` with `triage_signal='override'`.
4. **Trigger updates counters and strength** on insert into `memory_outcomes` (see Schema delta §3).
5. **UI surfaces.** Active tab shows live counters; detail pane shows event timeline.

## Phase 1 follow-ups (named, not built)

The design names these so accept events stay forward-compatible:

- **Remote `sensei-knowledge` service.** Daemon registers with it the same way it registers a gateway router (URL + key in Keychain, configured in the setup wizard). `insights` rows shipped as anonymized sync payloads. `accept_proposal` and successful `record_outcome` writes emit `insights` rows from day one of Phase 1 — Phase 0 leaves the emit point as a no-op function called from the same code path.
- **RBAC + governance roles + audit log** per Ethico §9.
- **`risk_class`** column on memories (`auto` / `review` / `approve`) for HITL gating.
- **Promotion ladder** (sensei-equivalent of `local→repo→project→org`) — requires a multi-machine concept that doesn't exist in Phase 0.
- **Constitution scope** as a non-overridable top tier above `global`.
- **Git mirror** of memories for external review.

## Error handling

| Error class | Daemon response | UI behavior |
|---|---|---|
| Invalid scope/scope_filter combo | 400 with field name | Show inline form error |
| Memory not found (accept/reject of unknown id) | 404 | Toast: "Proposal no longer exists" |
| Status transition forbidden (e.g. accept on already-active) | 409 with current status | Toast: "Already accepted at \<ts>" |
| Outcome on archived / rejected memory | Partial success: included in batch response under `skipped: [{memory_id, reason}]` (no top-level error) | Logged in daemon at INFO; AI batch caller may ignore or surface |
| Database unavailable | 503 | App-wide health banner (existing pattern) |

## Testing

Following sensei's TDD-first rule.

### Unit (Rust, `crates/senseid`)

- Knowledge module — assemble_context blending and precedence (test fixtures with all three scopes and overlapping ids)
- Trigger logic for memory_outcomes — each outcome value updates counters and status correctly; idempotency on duplicate inserts
- Edge cases: scope_filter empty for stack scope, tags filter with empty array, version hash stable across no-op reads

### Integration (Rust, `crates/senseid/tests/`)

- POST proposal → GET memories?status=proposed returns it
- Accept proposal → GET context returns it; GET memories?status=proposed excludes it
- Record_outcome (applied × 8) on a strength=1.0 memory transitions to `battle_tested`
- Record_outcome (violated) on `active` transitions to `challenged`; further violations decay strength below 1.0 → `archived`
- Tag filter on context returns subset

### Unit (TypeScript, `app/src/lib/`)

- `memoryState.svelte.ts` — hydration, tab counts, accept/reject mutations, optimistic UI
- `api.ts` — new methods (proposeMemory, acceptProposal, rejectProposal, recordOutcome, getLayeredContext, listMemories)

### E2E (Playwright, `app/e2e/tests/`)

- `learnings-triage.spec.ts` — Triage tab renders proposals, Accept moves row out, Reject moves to Archive
- `learnings-edit.spec.ts` — Edit & Accept saves changes
- `learnings-detail.spec.ts` — Detail pane shows evidence + outcomes
- `learnings-stack-filter.spec.ts` — Context call returns stack-matched memories for a Rust project but not for a Python one

## Open questions (resolved during writing)

- **Q: Reuse `recommendations` or new `memory_outcomes` table?**
  A: New table. `recommendations` is the wrong shape (FTR-measured imperatives, not per-event log).
- **Q: Triage signal column or in the proposal payload only?**
  A: Column. The signal is part of provenance and should survive accept/edit.
- **Q: Should `assemble_context` filter `rejected`?**
  A: Yes — always. Rejected memories are retained only for the Archive view.
- **Q: Where does the AI plugin's trigger-list live?**
  A: Bundled inside the sensei marketplace plugin (`marketplace/sensei-knowledge/CLAUDE.md`). Daemon doesn't enforce or care.

## Out of scope (Phase 0)

- Remote sync (`sensei-knowledge` service)
- RBAC, governance board, identity provider integration
- Promotion ladder across machines / users
- Risk-class HITL gating
- Constitution scope tier
- Git mirror of memories
- `pii_flag` / secret scanning on writes (will be added in Phase 1 alongside remote sync)
- Cache invalidation via pub/sub (Phase 0 uses simple in-process LRU; multi-replica coherence is Phase 1)

---

*End of Phase 0 specification. Phase 1 will be specced separately once Phase 0 is shipped and exercised.*
