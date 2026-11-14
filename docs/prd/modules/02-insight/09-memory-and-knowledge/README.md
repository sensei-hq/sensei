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

> **Status (2026-11-03).** Session-start memories are delivered by
> **`get_layered_context`** (`activity.memory_loads.source` defaults to it). The only
> LLM-facing feedback tool is **`record_outcome`** (one `memory_outcome` per memory id:
> `applied | consulted | violated | ignored`); there is **no `get_memories`,
> `report_memory_use` or `report_learning` MCP tool** (gap `G-MEM-FEEDBACK`). The human
> feedback routes are HTTP `…/reinforce` / `…/challenge` / `…/archive`. The memory API lives
> in `api/handlers/knowledge.rs` (not `memories.rs`). The stored model is `status` / `type` /
> `category` / `scope`+`scope_filter` (not `state` / `scope_level` / `scope_*` columns).
> The D56 promotion path (user → repository → workspace → dōjō → global shared) is
> **designed, not built** in the memory record: `memory_scope` has no `user`, `repository`
> or `dojo` step (gap `G-MEM-SCOPE`).

## In scope / out of scope

- In: the memory record and its step on the promotion path; formation (auto-cluster
  from correction signatures, a repeated blocking-target miss (D66), explicit capture);
  the **promotion path** (D56: user → repository → workspace → dōjō → global shared,
  never widen without a signal); the three user verbs (D65: **Share →**, **Reinforce**,
  **Retire**); the lifecycle state machine (proposed → reinforced → battle-tested ·
  challenged · archived); retrieval (`get_layered_context`) + feedback (`record_outcome`,
  HTTP `…/reinforce` / `…/challenge` / `…/archive`); session-start context assembly;
  continuity memories; consolidation (MOE-assisted merge); conflict resolution;
  preference-as-memory and skill-as-memory (no separate tables for these).
- Out: the correction-signal detection that seeds candidates (M7 analyzer); pattern
  machinery (M10); rule promotion from battle-tested memories (M13); the dōjō side of
  Share — review, k ≥ 3 publish to global shared, downstream pull (T1–T6); the verdict
  that drives Reinforce / Retire (M19 Impact); blocking-target evaluation itself (T3);
  user-side learnings (deferred, called out below).

## Surfaces

- **MCP** — `get_layered_context` (session-start loads — logged to `activity.memory_loads`,
  whose `source` defaults to `get_layered_context`), `context_pack`; writes via
  `save_memory` / `propose_memory` / `promote_memory`; feedback via `record_outcome`.
  **No** `get_memories` / `report_memory_use` / `report_learning` tool exists (see Status).
- **API** — `crates/senseid/src/api/handlers/knowledge.rs`;
  `GET /api/workspaces/{id}/memories`, `GET /api/knowledge/memories/{id}` plus
  `/promote`, `/generalise`, `/archive`, `/reinforce`, `/challenge`, `/dismiss`, `/merge`;
  workspace → dōjō share batches at `/api/workspaces/{id}/memory-batches` and
  `/api/share-review/*` (`workspace_detail.rs`, `share_review.rs`).
- **Daemon** — `tasks/handlers/consolidate.rs` + `derive_signals` (formation);
  strength recompute on the analyzer tick.
- **App** — anatomy / curation surface (Memories), Insights Now (proposed +
  challenged), Today adopted lane, workspace memories + ready-to-share.

### The user's verbs (D65)

A memory card offers at most three verbs. They replace the competing set the walkthrough
flagged as "Action overload" (promote · skill · reinforce · transfer).

- **Share →** moves a memory one step outward on the D56 path: to repository, to
  workspace, to dōjō (and from a dōjō to global shared, which T1 gates at k ≥ 3). It is the
  only outward verb. Today it is served by two mechanisms under other names: the
  `promote_memory` MCP tool / `POST …/promote` route (code identifiers; they copy the memory
  as `proposed` onto a governance namespace) and the workspace → dōjō share batch
  (`memory_share_batches`, `…/generalise`). The app calls only the second one sharing
  ("Ready to share" on the workspace Memories page, "Share review" in the rail), and only
  for the workspace → dōjō step; Share → as the one verb for every step is **not built**.
- **Reinforce** and **Retire** follow Impact's verdict (M19): a positive verdict leads to
  Reinforce, a negative one to Retire. The verdict already writes the outcome (built:
  `reinforce_source_memory_for_rec` / `challenge_source_memory_for_rec`). In the app,
  `Reinforce` is a button on the Memories screen's active list (`/learnings`,
  `ActiveList.svelte`); Retire is the `…/archive` route (`archive` in `memoryState`), not
  yet labelled Retire.
- **"Skill" is a memory kind, not a verb.** `memory_type` today is `decision | pattern |
  convention | preference | continuity | question` — **no `skill` kind** (not built). The
  separate `write_skill` upgrade kind (`upgrades/buckets.ts`) is a recommendation type, not
  a memory action.

## Data invariants

- `sensei.memories` — one row per memory: **`status`** (`memory_status`:
  `proposed | active | reinforced | challenged | battle_tested | archived | rejected`);
  **`type`** (`memory_type`: `decision | pattern | convention | preference | continuity |
  question`); **`category`** (`memory_category`: `correctness | convention | pattern |
  preference`); scoped by **`scope`** (`memory_scope`: `global | workspace | stack |
  task_type | module`) + **`scope_filter`** (stack / task / module qualifier) and
  `namespace_id`; provenance via `session_id` + `source_id`;
  `reinforced_count` / `violated_count`. There is **no** `state`, `scope_level`,
  `scope_project_id`/`scope_user_id`/`scope_org_id`, `references` jsonb, `signature` or
  `merged_into` column.
- **Promotion path (D56).** Knowledge moves outward: **user → repository → workspace →
  dōjō → global shared**. This runs opposite to the governance ladder (M13, D57). Each step
  outward is a **promotion, never a precedence**: a memory shared to the dōjō does not
  override a repository's own memory or rule; governance precedence stays the M13 rule.
  Leaving a dōjō for global shared needs **k ≥ 3 distinct contributors** (D26, T1; built
  on the dōjō side as `K_ANONYMITY_MIN_CONTRIBUTORS = 3` in
  `dojo/src/lib/server/artifacts-data.ts`).
- **Schema gap (not built).** `memory_scope` today is `global | workspace | stack |
  task_type | module` (`database/ddl/enum/sensei/memory_scope.ddl`), default `workspace`.
  It needs `user`, `repository` and `dojo` steps (gap `G-MEM-SCOPE`; the learnings lane
  owns the schema item). Until then the outward step is carried by `namespace_id` alone:
  `promote_memory` copies `scope` unchanged and swaps the namespace, and a `gov_scope` of
  `user` or `general` resolves to `namespace_id NULL`, so a user-step memory cannot be told
  apart from an unscoped one. Dedup/conflict resolution is by `content`/namespace — there
  is no `signature` column.
- **Formation.** Auto-cluster: `count ≥ CORRECTION_MIN` (2 — `analyze.rs`:
  `const CORRECTION_MIN: usize = 2`) across ≥ 2 distinct sessions → `proposed` memory.
  Explicit capture writes `status = proposed` and follows the same lifecycle.
  Formation always writes at the **narrowest step the signal justifies** — widening is a
  human decision (Share →).
- **Missed-target formation (D66, not built).** Two misses of a **blocking** target (T3)
  on the same cause propose a memory at the **user** step. Once that memory helps (a
  positive verdict, M19), it is offered for Share → up the D56 path. Nothing in
  `crates/senseid/src` links a target miss to a memory today; it needs the `user` step
  above and T3's target evaluation.
- **Never widen without a signal.** Share → recomputes strength at the wider step;
  battle-tested in one repository is not battle-tested for the workspace until reinforced
  in its other repositories. Narrowing is allowed and drops wider-step reinforcement.
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

| Element | Meaning | Worked example | Decision it drives / next step |
|---|---|---|---|
| `get_layered_context` result | what the assistant sees at session start | 6 battle_tested + 2 proposed, scope-ordered | none for the user — it is the assistant's input; candidate to cut as a shown element |
| Adopted lane | `status = battle_tested` or fresh promotion | "adopted: run pre-commit before PR" | Share → to the next step on the D56 path |
| Proposed memory | `status = proposed` | Insights Now ("review?") | accept it at its step, or dismiss it |
| Proposed from a missed target (D66, not built) | two blocking-target misses on one cause | "lint target missed twice: unused imports" | accept at the user step; Share → once it helps |
| Challenged memory | violation observed, or a negative Impact verdict | "keep or retire" card | Reinforce or Retire, per Impact's verdict (M19) |
| Ready to share | generalised memory, staged for a share batch | "3 memories ready to share" | Share → to dōjō (propose the batch) |
| Skipped-repeatedly (not built) | ≥5 consecutive skips in relevant sessions; needs `report_memory_use` | Insight: "this memory keeps getting skipped — retire?" | Retire |
| Memory-health | `activity.memory_loads` volume + skip rollups | `GET /api/workspaces/{id}/memories` (no dedicated health route) | none — candidate to cut |

## Done gate

- `mcp_call get_layered_context` returns a scope-appropriate list for the current
  session's workspace, ordered by status then strength; every call lands in
  `activity.memory_loads` (`source = 'get_layered_context'`).
- A reinforcement / challenge write (`POST /api/knowledge/memories/{id}/reinforce`
  or `/challenge`, or `record_outcome`) updates the counters; `skipped` doesn't penalise.
- A memory past the reinforcement threshold without a challenge advances at the next
  analyzer tick; persistent violations archive unless the user reinforces first.
- Overlapping-scope memories never both fire — dedup holds.
- A workspace-scope memory for `other_project` never appears in a Dev session (scope
  confidentiality).
- A memory shared to the dōjō step never overrides a repository rule or memory at
  resolution time (D56: promotion, not precedence).
- `what`/`because` text is narration-cache-generated (`memory_what`/`memory_because`)
  with templated fallback — never "Memory 47".

```bash
curl -s "http://localhost:7744/api/workspaces/<workspace-id>/memories?limit=10" \
  | jq '.memories | map({status, scope, title})'
psql -A -t -c "select count(*) from activity.memory_loads
                where loaded_at > now() - interval '1h'" -d sensei
```

## Wrong gate

- **`get_layered_context` returns 0 memories for a workspace whose Anatomy view shows
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
- **A memory card offers several competing outward verbs** (promote · skill · transfer)
  instead of one Share → — the "Action overload" gap returns.
- **A shared memory wins over a narrower rule because it sits further out** — promotion
  treated as precedence (D56).
- **A memory leaves a dōjō for global shared with fewer than 3 contributors** (D26).

## Scenarios

- [`scenarios/memory.feature`](scenarios/memory.feature) — formation, scoping,
  lifecycle, retrieval/feedback, continuity, consolidation.

## Open gaps

- No M9 rows in [06-gaps.md](../../../../roadmap/06-gaps.md) today. Watch:
  post-compaction re-grounding (governance feature status 🟡) and user-side learnings
  (deferred by design — schema + narration kinds should stay shaped for it).
- **`G-MEM-SCOPE`** — `memory_scope` has no `user`, `repository` or `dojo` step; the D56
  promotion path is designed, not built in the memory record (outward steps ride on
  `namespace_id` today).
- **`G-MEM-FEEDBACK`** — designed, not built. Two LLM-facing tools
  ([spec/pipeline/memory](../../../../spec/pipeline/memory.md)):
  - `report_memory_use({session_id, followed[], skipped[], violated[]})`, called at
    session end: per loaded memory id, whether the assistant followed, skipped or violated
    it (with a note / reason / how). `followed` → reinforcement, `violated` → violation,
    `skipped` → recorded only (drives Skipped-repeatedly). Ids must have been loaded for
    that session (`activity.memory_loads`) or are dropped. Its planned store
    `activity.memory_use_reports` does not exist.
  - `report_learning({session, observation, evidence})`: the assistant proposes a *new*
    memory ("we corrected the auth handler shape three times"); confirmation writes it as
    `proposed` at the user step.
  `record_outcome` exists and covers per-id outcomes, but has no skipped report and no
  way to propose a new memory.
- **D66 missed-target formation** and **Share → as one verb** — not built (see above).

## Related

- [spec/pipeline/memory](../../../../spec/pipeline/memory.md) — the record, ladder,
  lifecycle, session-start context tree, consolidation (legacy depth, folds here).
- [spec/pipeline/narration-cache](../../../../spec/pipeline/narration-cache.md) — the
  `what`/`because` copy pipeline.
- [spec/pipeline/governance](../../../../spec/pipeline/governance.md) — memory → rule
  promotion path (the M13 fold is authoritative once written).
- Decisions D56 (promotion path), D65 (verbs), D66 (missed-target memories), D26 (k ≥ 3) —
  [plan/2026-11-03-parallel-lanes.md §0](../../../../plan/2026-11-03-parallel-lanes.md).
- [M19 Insights & Impact](../../03-delivery/19-insights-and-impact/README.md) — the
  verdict behind Reinforce / Retire.
- [spec/screen/observatory-memories](../../../../spec/screen/observatory-memories.md) ·
  [features/03-observatory.md](../../../../features/03-observatory.md) — Memories
  section status.
- [prd/02-objectives.md](../../../02-objectives.md) · [prd/01-vision.md §The core loop](../../../01-vision.md#the-core-loop).
- Crate: `crates/senseid/src/tasks/handlers/consolidate.rs`,
  `crates/senseid/src/api/handlers/knowledge.rs`, `crates/mcp/src/lib.rs`
  (`get_layered_context`, `save_memory`, `propose_memory`, `promote_memory`).