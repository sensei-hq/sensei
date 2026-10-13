# Memory & Knowledge · M9

**Plane:** insight · **Status:** [roadmap/02-insight.md](../../../../roadmap/02-insight.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)),
team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

Memory is how the pairing gets better over time. Every session produces evidence —
corrections, patterns, deference to one style over another. The analyzer clusters
that evidence into memory candidates; when a candidate accrues enough support across
sessions it becomes a **memory** an agent loads at the start of the next relevant
session. *Sensei watches, sensei forms an opinion, the agent adopts it, the outcome
either reinforces or challenges it.* The canonical consumer of a memory is not the
human — it's the LLM starting the next session and calling `get_memories(project=X)`.
The human's surfaces (anatomy view, adoption lane, triage) are windows into that
loop. The user feels this module as *the assistant stops repeating yesterday's
mistakes*.

## In scope / out of scope

- In: the memory record + scope contract (project > user > org, tighter wins);
  formation (auto-cluster from correction signatures + explicit promotion); the
  promotion ladder (project → user → org → collective, skip-levels allowed, never
  widen without a signal); the lifecycle state machine (proposed → reinforced →
  battle-tested · challenged · archived); retrieval (`get_memories`) + feedback
  (`report_memory_use`, `report_learning`); session-start context assembly;
  continuity memories; consolidation (MOE-assisted merge); conflict resolution;
  preference-as-memory (no separate preferences table for these).
- Out: the correction-signal detection that seeds candidates (M7 analyzer); pattern
  machinery (M10); rule promotion from battle-tested memories (M13); Dōjō
  upstream/downstream memory movement (T1–T4, via governance); user-side learnings
  (deferred, called out below).

## Surfaces

- **MCP** — `get_memories` (session-start loads, logged in `activity.memory_loads`),
  `report_memory_use` (self-report at session end), `report_learning` (proposes new
  memories), `promote_memory` (explicit promotion).
- **API** — `crates/senseid/src/api/handlers/memories.rs`; `/api/memories?project=…`,
  `GET /api/observatory/memory-health`.
- **Daemon** — `tasks/handlers/consolidate.rs` + `derive_signals` (formation);
  strength recompute on the analyzer tick.
- **App** — anatomy / curation surface (Memories), Insights Now (proposed +
  challenged), Today adopted lane, project memories + ready-to-share.

## Data invariants

- `sensei.memories` — one row per memory: `state` enum
  `proposed | reinforced | battle-tested | challenged | archived`; `category` enum
  `preference | convention | anti_pattern | pattern | correctness`; `scope_level`
  enum `global | user | org | project | module | stack`; scoped by `scope_project_id`
  / `scope_user_id` / `scope_org_id` / `scope_modules` / `scope_stack`;
  `references` jsonb of producing/reinforcing session ids; `violated`/`reinforced`
  counters; `signature` (deterministic hash — dedup + correction-to-memory linking).
- **Scope contract.** A memory matches a session when at least one of: same project,
  same user (cross-project, project_id NULL), or same org (overlay). On conflict by
  **signature**, the tighter scope wins (project > user > org) — resolved
  server-side, one canonical row per signature.
- **Formation.** Auto-cluster: `count ≥ CORRECTION_MIN` (3) across ≥ 2 distinct
  sessions → `proposed` memory. Explicit promotion writes `state = proposed` and
  follows the same lifecycle. Formation always writes at the **narrowest scope the
  signal justifies** — widening is a human decision.
- **Never widen without a signal.** Widening recomputes strength at the wider scope;
  battle-tested in one project is not battle-tested for a user until reinforced in
  their other projects. Narrowing is allowed and drops wider-scope reinforcement.
- **Lifecycle.** `reinforced ≥ MEM_REINFORCE_MIN` (3) in a rolling 30d window without
  violation advances state; a new violating correction → `challenged`; violations ≥
  `MEM_CHALLENGE_MAX` (3) → `archived` (unless the user reinforces first).
- **`strength = f(reinforced, violated, age)`** — recency-weighted; `skipped` reports
  never touch strength.
- **Retrieval.** Default `limit = 25`; ordered by `state`
  (battle-tested > reinforced > proposed) then strength desc; every call logged to
  `activity.memory_loads` (read-side telemetry that feeds reinforcement).
- **Self-report is untrusted evidence** — FTR movement + observed corrections outweigh
  it; "followed" alone never promotes to battle-tested.
- **Continuity memories** (`category: continuity`): auto-generated on session end,
  auto-decay after 7 days, never promote to rules, never widen scope.
- **Consolidation** merges clusters only after user approval; cross-scope merge
  requires widen/narrow first (ladder invariant). Members archive with
  `merged_into`; the representative absorbs strengths + reinforcement counts.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| `get_memories(scope)` result | what the assistant sees at session start | 6 battle-tested + 2 proposed, scope-ordered |
| Adopted lane | `state = battle-tested` or fresh promotion | "adopted: run pre-commit before PR" |
| Proposed memory | `state = proposed` | Insights Now ("review?") |
| Challenged memory | violation observed | "keep or retire" card |
| Skipped-repeatedly | ≥5 consecutive skips in relevant sessions | Insight: "this memory keeps getting skipped — retire?" |
| Memory-health | `activity.memory_loads` volume + skip rollups | `/api/observatory/memory-health` |

## Done gate

- `mcp_call get_memories` returns a scope-appropriate list for the current session's
  project + user + org, ordered by state then strength; every call lands in
  `activity.memory_loads`.
- A `report_memory_use` with `followed`/`violated` updates the counters; `skipped`
  doesn't penalise.
- A memory crossing `MEM_REINFORCE_MIN` without a challenge advances automatically at
  the next analyzer tick; violations crossing `MEM_CHALLENGE_MAX` archive unless user
  reinforces first.
- Same-signature, overlapping-scope memories never both fire — dedup holds.
- A project-scope memory for `other_project` never appears in a Dev session (scope
  confidentiality).
- `what`/`because` text is narration-cache-generated (`memory_what`/`memory_because`)
  with templated fallback — never "Memory 47".

```bash
curl -s "http://localhost:7744/api/memories?project=sensei&limit=10" \
  | jq '.memories | map({state, scope_level, what})'
psql -A -t -c "select count(*) from activity.memory_loads
                where loaded_at > now() - interval '1h'" -d sensei
```

## Wrong gate

- **`get_memories` returns 0 for a project whose Anatomy view shows rows** — scope
  contract under-implemented (only `scope_project_id` read).
- **Same memory returns twice** (project + user scope) — signature dedup missing.
- **State never advances from `proposed`** — reinforcement counter not wired.
- **Every memory has `strength = 0`** — strength recompute not scheduled.
- **`skipped` reports move strength down** — only violations penalise.
- **A memory widens scope without a fresh strength calc** — battle-tested claim
  silently inherited by a wider scope.
- **LLM self-reports ids that were never loaded for that session** — validate against
  `memory_loads` or drop.
- **Continuity memory promoted to a rule** — violates the resume-not-teach contract.

## Scenarios

- [`scenarios/memory.feature`](scenarios/memory.feature) — formation, scoping,
  lifecycle, retrieval/feedback, continuity, consolidation.

## Open gaps

- No M9 rows in [06-gaps.md](../../../../roadmap/06-gaps.md) today. Watch:
  post-compaction re-grounding (governance feature status 🟡) and user-side learnings
  (deferred by design — schema + narration kinds should stay shaped for it).

## Related

- [spec/pipeline/memory](../../../../spec/pipeline/memory.md) — the record, ladder,
  lifecycle, session-start context tree, consolidation (legacy depth, folds here).
- [spec/pipeline/narration-cache](../../../../spec/pipeline/narration-cache.md) — the
  `what`/`because` copy pipeline.
- [spec/pipeline/governance](../../../../spec/pipeline/governance.md) — memory → rule
  promotion path (the M13 fold is authoritative once written).
- [spec/screen/observatory-memories](../../../../spec/screen/observatory-memories.md) ·
  [features/03-observatory.md](../../../../features/03-observatory.md) — Memories
  section status.
- [prd/02-objectives.md](../../../02-objectives.md) · [prd/01-vision.md §The core loop](../../../01-vision.md#the-core-loop).
- Crate: `crates/senseid/src/tasks/handlers/consolidate.rs`, `crates/mcp/src/tools/get_memories.rs`.