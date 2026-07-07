# 覚 · Pipeline · Memory

**Owner files:**
- Formation: `crates/senseid/src/tasks/handlers/consolidate.rs` +
  `crates/senseid/src/tasks/handlers/analyze.rs::derive_signals` (correction clustering)
- Retrieval: `crates/senseid/src/api/handlers/memories.rs` +
  `crates/mcp/src/tools/get_memories.rs`
- Feedback: `crates/senseid/src/api/handlers/memories.rs::report_memory_use`

**Primary consumer:** the LLM agent, at session start (via MCP tool
`get_memories`). Human review is secondary — the surface for
inspecting, editing, and curating what the LLM sees.

## Purpose

Memory is **how the pairing gets better over time**. Every session
produces evidence — corrections, patterns, deference to one style
over another. The analyzer clusters that evidence into **memory
candidates**; when a candidate accrues enough support across
sessions, it becomes a **memory** an agent can load at the start of
the next relevant session.

The mental model: *sensei watches, sensei forms an opinion, the
agent adopts it, the outcome either reinforces or challenges it.*
Every screen the user sees is a window into this loop — Today's
adopted lane, Insights' triage, the Memories anatomy view — but the
canonical consumer of a memory is not the human. It's the LLM
starting the next session and calling `get_memories(project=X)`.

Kanji is 覚 — *to remember / awareness*.

## Data invariants

### The record

- `sensei.memories` — one row per memory:
  - `id` uuid
  - `what` text (statement — model-generated, cached in insight_copy)
  - `because` text (causal chain — model-generated)
  - `state` enum `proposed | reinforced | battle-tested | challenged | archived`
  - `strength` numeric 0..1
  - `category` enum `preference | convention | anti_pattern | pattern | correctness`
  - `scope_level` enum `global | user | org | project | module | stack`
  - `scope_project_id` uuid nullable
  - `scope_user_id` uuid nullable (usually the current Jerry, but Dōjō layers add org-mates)
  - `scope_org_id` uuid nullable
  - `scope_modules` text[] nullable
  - `scope_stack` text[] nullable
  - `references` jsonb — session ids that produced or reinforced this memory
  - `violated`, `reinforced` int counters
  - `created_at`, `last_seen_at`, `promoted_at` timestamptz
  - `signature` text — deterministic hash of the memory shape used
    for dedup and correction-to-memory linking

### The scope contract (retrieval)

Session scope at boot = `{ project_id, user_id, org_id? }`.

A memory matches when **at least one of**:

1. `scope_project_id == session.project_id` (this project)
2. `scope_user_id == session.user_id AND scope_project_id IS NULL`
   (this user, cross-project)
3. `scope_org_id == session.org_id AND scope_project_id IS NULL AND scope_user_id IS NULL`
   (org-wide overlay)

Priority on **conflict** (same signature, different scopes): the
tighter scope wins (`project > user > org`). The retrieval API
resolves collisions server-side — the assistant sees one canonical
row per signature.

### Formation

Memories come from **two paths**, both writing to the same table:

1. **Auto-cluster (primary).** `derive_signals` clusters corrections
   by signature. When count ≥ `CORRECTION_MIN` (currently 3, in
   `analyze.rs`) across ≥ 2 distinct sessions, `consolidate` inserts
   a `proposed` memory referencing the source corrections.
2. **Explicit promotion.** The user promotes a session turn (or a
   pattern) to a memory via the UI or the MCP `promote_memory` tool.
   `state = proposed` on insert; goes through the same lifecycle
   after.

### Scope widening — the human's job

Formation writes memories at the **narrowest scope the signal
justifies** — usually `project` (because the correction came from a
project's sessions). The human decides whether that scope is right.

Example: sensei's own sessions produce a memory "use dbd for
database schema migrations". Because it came from the sensei
project's sessions, the daemon writes it at `scope = project =
sensei`. But `dbd` is my (Jerry's) tool across projects — so I
widen the scope to `user` from the anatomy screen. Now it applies
to every session I open, in any project.

Another example: "always run pre-commit before opening a PR". That
came from one project too, but it's a universal habit — widen to
`org` (if I have an org) or leave at `user`.

Counter-example: "the auth module uses refresh tokens with
device-flow" is inherently project-specific. It stays at project
scope; widening it would be nonsense.

**The promotion ladder.** Memories can move UP:

    project ─→ user ─→ org ─→ collective (global)
                                   ↑
    project ────────────────────────┘ (skip levels is allowed)

- `project → user`: click "promote to user" on the anatomy screen.
  No governance step. It's the user's own scope.
- `user → org`: enters the org's Dōjō triage queue (see
  [[pipeline/governance]] and [[pipeline/dojo-lifecycle]]). A
  maintainer approves; on approval, the memory is published at org
  scope and every matching session downstream picks it up.
- `org → collective`: same triage step at the collective's level.
  Attribution rules apply — client-work memories are auto-
  dereferenced.

**Never widen without a signal.** Widening triggers a fresh
strength calc at the wider scope — the strength doesn't carry.
A memory battle-tested in one project isn't battle-tested for a
user until it's reinforced in that user's other projects.

**Narrowing is also allowed.** If a memory turns out to be
project-specific after being widened, the user can narrow it back.
Narrowing drops the wider-scope reinforcement history.

### Lifecycle (state machine)

    proposed ─────┬─→ reinforced ─────┬─→ battle-tested
                  │                     │
                  └──────challenged ←───┴──── (violation observed)
                                        │
                                        ├─→ reinforced (recovers)
                                        └─→ archived (retired)

- `proposed → reinforced` when `reinforced ≥ MEM_REINFORCE_MIN`
  (currently 3) in a rolling 30d window without a violation.
- `reinforced → battle-tested` when the same threshold is hit again
  after promotion without a violation.
- `{any} → challenged` when a new correction violating the memory's
  signature is captured.
- `challenged → reinforced` when a subsequent session confirms the
  memory again.
- `challenged → archived` when the user archives OR
  `MEM_CHALLENGE_MAX` (currently 3) violations accumulate.

`strength = f(reinforced_count, violated_count, age)` — recency-
weighted so old-but-stable memories decay slowly.

### Retrieval — the MCP contract

Tool: `get_memories`.

Input (all optional): `{ project_id?, user_id?, org_id?, category?, limit? }`.

Behaviour:

- Resolves scope defaults from the session context if omitted.
- Returns memories matching the scope contract above, ordered by
  `state` (`battle-tested > reinforced > proposed`), then
  `strength` desc.
- Default `limit = 25`. Higher requires an explicit `limit` param
  so cost stays predictable.
- Every call is **logged** in `activity.memory_loads` with
  `{session_id, loaded_ids, loaded_at}` — this is the read-side
  telemetry that feeds strength reinforcement.

### Feedback — the LLM self-reports

Tool: `report_memory_use`.

Called by the assistant at session end (or at any consolidation
point). Input:

    {
      "session_id": "…",
      "followed": [ { "id": "…", "note": "…" }, … ],
      "skipped":  [ { "id": "…", "reason": "…" }, … ],
      "violated": [ { "id": "…", "how": "…" }, … ]
    }

Writes into `activity.memory_use_reports`. The analyzer's next tick
applies:

- `followed` → `reinforced++`
- `violated` → `violated++` → maybe state transition to `challenged`
- `skipped` → does NOT touch strength (recorded for diagnostics
  only). If a memory is skipped in ≥ 5 consecutive relevant
  sessions, an Insight is generated ("this memory keeps getting
  skipped — retire?").

**Self-report is untrusted evidence.** It's a signal, not a source
of truth. Actual FTR movement + observed corrections outweigh
self-report on strength calc. This is why the "followed" tally
alone doesn't promote a memory to `battle-tested` — corrections
absence matters too.

## Signals produced

| Signal | Source | Consumer |
|---|---|---|
| Adopted memory row | insert to `sensei.memories` with `state=battle-tested` OR fresh promotion | Today "adopted lane" |
| Proposed memory row | insert with `state=proposed` | [[screen/observatory-insights]] Now column |
| Challenged memory | state transition | Insights Now column ("challenged — keep or retire") |
| Skipped-repeatedly | activity.memory_use_reports rollup | Insights Soon column recommendation ("retire?") |
| Memory match at session start | MCP `get_memories` call | Assistant context; logged in `activity.memory_loads` |

## Done gate

- On Jerry's live data, the MCP `get_memories` tool returns a
  scope-appropriate list for the current session's project + user +
  optional org, ordered by state then strength.
- Every `get_memories` call writes to `activity.memory_loads`.
- Every session with a `report_memory_use` call updates the
  `reinforced` / `violated` counters on the reported ids.
- A memory that reaches `reinforced ≥ MEM_REINFORCE_MIN` without a
  `challenged` transition in the window advances state
  automatically at the next analyzer tick.
- A memory whose `violated` crosses `MEM_CHALLENGE_MAX` advances
  to `archived` at the next analyzer tick unless the user
  reinforces first.
- The `what` and `because` text on every memory is populated via
  [[pipeline/insight-copy]] with `kind = memory_what` /
  `kind = memory_because` — templated fallbacks otherwise.
- Scope resolution is deterministic: the same session id resolves
  to the same memory set within a tick.
- Dedup: memories with the same `signature` in overlapping scopes
  don't both fire — the tighter scope wins.

Optional check:
```
# What memories would the assistant see for sensei right now?
curl -s "http://localhost:7744/api/memories?project=sensei&limit=10" \
  | jq '.memories | map({state, scope_level, what})'

# Did the last N `get_memories` calls actually log to memory_loads?
psql -A -t -c "select count(*) from activity.memory_loads
                where loaded_at > now() - interval '1h'" -d sensei

# Are any battle-tested memories getting skipped repeatedly?
curl -s http://localhost:7744/api/observatory/memory-health | jq '.skipped_repeatedly'
```

## Wrong gate

- **`get_memories` returns 0 for the sensei project despite the
  Anatomy view showing rows.** Scope match not implemented; the
  MCP tool is reading only `scope_project_id == this` and ignoring
  the user/org overlays.
- **Same memory returns twice** (once at project scope, once at
  user scope). Dedup by signature isn't applied.
- **Memory state never advances from `proposed`.** Reinforcement
  counter not being incremented from `report_memory_use` writes.
- **Every memory has `strength = 0`.** Strength recomputation not
  scheduled (see [[pipeline/analyzer]] tick).
- **`skipped` reports move `strength` down.** Skipping shouldn't
  penalise; only violations do.
- **A memory with `scope_project_id = other_project` shows up in
  Jerry's session.** Scope filter bug — a hard confidentiality
  regression if Dōjō is wired.
- **`what` reads "Memory 47".** Insight-copy fallback fired but
  no template was set for `kind = memory_what` — add one.
- **The LLM says "I followed memory X" but there's no matching
  `id` in memory_loads for that session.** Self-report accepts
  arbitrary ids — validate against the load log or drop.

## Related

- [[pipeline/analyzer]] — schedules consolidate + strength recompute
- [[pipeline/signals]] — the correction-signal that seeds candidates
- [[pipeline/insight-copy]] — `what` and `because` text
- [[pipeline/governance]] — Dōjō upstream/downstream for memories
- [[screen/observatory-memories]] — the human anatomy / curation surface
- [[screen/observatory-insights]] — proposed and challenged land here
- [[screen/observatory-today]] — the adopted lane consumer
- [[screen/project-memories]] — project-scoped view + ready-to-share
