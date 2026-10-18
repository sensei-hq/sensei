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
human — it's the LLM starting the next session and calling `get_layered_context`
(which assembles the scoped memories; `activity.memory_loads.source` defaults to
`get_layered_context`).
The human's surfaces (anatomy view, adoption lane, triage) are windows into that
loop. The user feels this module as *the assistant stops repeating yesterday's
mistakes*.

> **Status honesty (2026-10-14).** Session-start memories are delivered today by
> **`get_layered_context`** (`activity.memory_loads.source` defaults to it) — there is
> **no `get_memories`, `report_memory_use` or `report_learning` MCP tool**; feedback is
> the HTTP `…/reinforce` / `…/challenge` routes. The memory API lives in
> `api/handlers/knowledge.rs` (not `memories.rs`). The stored model is `status` / `type` /
> `category` / `scope`+`scope_filter` (not `state` / `scope_level` / `scope_*` columns),
> and the `project > user > org` ladder is **designed, not built** (`memory_scope` has no
> user/org rung). See gaps `G-MEM-SCOPE`, `G-MEM-FEEDBACK`.

## In scope / out of scope

- In: the memory record + scope contract (project > user > org, tighter wins);
  formation (auto-cluster from correction signatures + explicit promotion); the
  promotion ladder (project → user → org → collective, skip-levels allowed, never
  widen without a signal); the lifecycle state machine (proposed → reinforced →
  battle-tested · challenged · archived); retrieval (`get_layered_context`) + feedback
  (HTTP `…/reinforce` / `…/challenge`, `record_outcome`); session-start context assembly;
  continuity memories; consolidation (MOE-assisted merge); conflict resolution;
  preference-as-memory (no separate preferences table for these).
- Out: the correction-signal detection that seeds candidates (M7 analyzer); pattern
  machinery (M10); rule promotion from battle-tested memories (M13); Dōjō
  upstream/downstream memory movement (T1–T4, via governance); user-side learnings
  (deferred, called out below).

## Surfaces

- **MCP** — `get_layered_context` (session-start loads — logged to `activity.memory_loads`,
  whose `source` defaults to `get_layered_context`), `context_pack`; writes via
  `save_memory` / `propose_memory` / `promote_memory`. **No** `get_memories` /
  `report_memory_use` / `report_learning` tool exists (see Status honesty).
- **API** — `crates/senseid/src/api/handlers/knowledge.rs`;
  `GET /api/projects/{id}/memories`, `GET /api/knowledge/memories/{id}` plus
  `/promote`, `/generalise`, `/archive`, `/reinforce`, `/challenge`, `/dismiss`, `/merge`.
- **Daemon** — `tasks/handlers/consolidate.rs` + `derive_signals` (formation);
  strength recompute on the analyzer tick.
- **App** — anatomy / curation surface (Memories), Insights Now (proposed +
  challenged), Today adopted lane, project memories + ready-to-share.

## Data invariants

- `sensei.memories` — one row per memory: **`status`** (`memory_status`:
  `proposed | active | reinforced | challenged | battle_tested | archived | rejected`);
  **`type`** (`memory_type`: `decision | pattern | convention | preference | continuity |
  question`); **`category`** (`memory_category`: `correctness | convention | pattern |
  preference`); scoped by **`scope`** (`memory_scope`: `global | project | stack |
  task_type | module`) + **`scope_filter`** (stack / task / module qualifier) and
  `namespace_id`; provenance via `session_id` + `source_id`;
  `reinforced_count` / `violated_count`. There is **no** `state`, `scope_level`,
  `scope_project_id`/`scope_user_id`/`scope_org_id`, `references` jsonb, `signature` or
  `merged_into` column.
- **Scope contract.** `memory_scope` is `global | project | stack | task_type | module`
  — there is **no user or org rung in the enum today** (`project_id NULL` = global). The
  `project > user > org` ladder is **designed, not built** (gap `G-MEM-SCOPE`);
  dedup/conflict resolution is by `content`/namespace — there is no `signature` column.
- **Formation.** Auto-cluster: `count ≥ CORRECTION_MIN` (2 — `analyze.rs`:
  `const CORRECTION_MIN: usize = 2`) across ≥ 2 distinct sessions → `proposed` memory.
  Explicit promotion writes `status = proposed` and follows the same lifecycle.
  Formation always writes at the **narrowest scope the signal justifies** — widening is a
  human decision.
- **Never widen without a signal.** Widening recomputes strength at the wider scope;
  battle-tested in one project is not battle-tested for a user until reinforced in
  their other projects. Narrowing is allowed and drops wider-scope reinforcement.
- **Lifecycle.** `memory_status` advances under reinforcement and drops to `challenged` /
  `archived` under violations. The `MEM_REINFORCE_MIN` / `MEM_CHALLENGE_MAX` **named
  constants do not exist in code** — reinforcement amount is `REINFORCE_AMOUNT = 1.0`
  (`knowledge.rs`); treat the (3)/(3) figures as design intent, not code constants.
- **`strength = f(reinforced, violated, age)`** — recency-weighted; `skipped` reports
  never touch strength.
- **Retrieval.** Default `limit = 25`; ordered by `status`
  (`battle_tested > reinforced > active`) then strength desc; every call logged to
  `activity.memory_loads` (read-side telemetry that feeds reinforcement).
- **Self-report is untrusted evidence** — FTR movement + observed corrections outweigh
  it; "followed" alone never promotes to battle-tested.
- **Continuity memories** (`type: continuity`): auto-generated on session end,
  auto-decay after 7 days, never promote to rules, never widen scope.
- **Consolidation** merges clusters only after user approval; cross-scope merge
  requires widen/narrow first (ladder invariant). There is **no `merged_into` column**
  today — members archive and the representative absorbs strengths + reinforcement counts.

## Signals shown

| Element | Meaning | Worked example |
|---|---|---|
| `get_layered_context` result | what the assistant sees at session start | 6 battle_tested + 2 proposed, scope-ordered |
| Adopted lane | `status = battle_tested` or fresh promotion | "adopted: run pre-commit before PR" |
| Proposed memory | `status = proposed` | Insights Now ("review?") |
| Challenged memory | violation observed | "keep or retire" card |
| Skipped-repeatedly | ≥5 consecutive skips in relevant sessions | Insight: "this memory keeps getting skipped — retire?" |
| Memory-health | `activity.memory_loads` volume + skip rollups | `GET /api/projects/{id}/memories` (no dedicated health route) |

## Done gate

- `mcp_call get_layered_context` returns a scope-appropriate list for the current
  session's project, ordered by status then strength; every call lands in
  `activity.memory_loads` (`source = 'get_layered_context'`).
- A reinforcement / challenge write (`POST /api/knowledge/memories/{id}/reinforce`
  or `/challenge`, or `record_outcome`) updates the counters; `skipped` doesn't penalise.
- A memory past the reinforcement threshold without a challenge advances at the next
  analyzer tick; persistent violations archive unless the user reinforces first.
- Overlapping-scope memories never both fire — dedup holds.
- A project-scope memory for `other_project` never appears in a Dev session (scope
  confidentiality).
- `what`/`because` text is narration-cache-generated (`memory_what`/`memory_because`)
  with templated fallback — never "Memory 47".

```bash
curl -s "http://localhost:7744/api/projects/<project-id>/memories?limit=10" \
  | jq '.memories | map({status, scope, title})'
psql -A -t -c "select count(*) from activity.memory_loads
                where loaded_at > now() - interval '1h'" -d sensei
```

## Wrong gate

- **`get_layered_context` returns 0 memories for a project whose Anatomy view shows
  rows** — scope contract under-implemented (only `project_id` read).
- **Same memory returns twice** — dedup missing.
- **Status never advances from `proposed`** — reinforcement counter not wired.
- **Every memory has `strength = 0`** — strength recompute not scheduled.
- **`skipped` reports move strength down** — only violations penalise.
- **A memory widens scope without a fresh strength calc** — battle_tested claim
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
- **`G-MEM-SCOPE`** — `memory_scope` has no `user` / `org` rung; the cross-project and
  org-overlay ladder is designed, not built.
- **`G-MEM-FEEDBACK`** — the LLM-facing feedback tools (`report_memory_use`,
  `report_learning`) do not exist; feedback is HTTP-only today.

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
- Crate: `crates/senseid/src/tasks/handlers/consolidate.rs`,
  `crates/senseid/src/api/handlers/knowledge.rs`, `crates/mcp/src/lib.rs`
  (`get_layered_context`, `save_memory`, `propose_memory`, `promote_memory`).