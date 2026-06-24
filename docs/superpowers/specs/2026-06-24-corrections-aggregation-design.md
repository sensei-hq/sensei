---
title: Corrections aggregation — design
description: Aggregate recurring developer corrections into project-tagged clusters with a canonical statement, suggestion, and linked memory — backing the Observatory's Corrections view.
type: spec
status: design
created: 2026-06-24
references:
  - docs/analysis/2026-06-24-mockup-vs-daemon-data-gap.md
  - docs/mockups/Sensei/lib/learnings-data.js
  - crates/senseid/src/tasks/handlers/analyze.rs
  - crates/senseid/src/tasks/handlers/prompt_classify.rs
  - crates/senseid/src/pattern_effectiveness.rs
  - crates/senseid/src/tasks/analyzer_scheduler.rs
---

# Corrections aggregation — design

Step 5 of the standalone-completion build order (the sibling, pattern-effectiveness,
already shipped). Implements the **Corrections view** gap from
`docs/analysis/2026-06-24-mockup-vs-daemon-data-gap.md` (P2):

> UI wants per-correction-text aggregation `{text, count, lastSeen, projects[], suggestion, memoryId?}`;
> L1 has folder-level correction-prone patterns with prompt snippets. → aggregate by
> recurring correction text + attach suggestion/memory link.

## Goal

Cluster recurring corrective developer prompts — **globally across all projects** —
into a small set of canonical corrections, each carrying a clean statement, a
recurrence count, recency, the projects it appeared in, an LLM-written suggestion,
and a link to a related learned memory. Expose them via a global and a per-project
API for the Observatory.

## Scope (locked during brainstorming)

- **Full UI shape**, including the two "smart" fields: LLM-generated `suggestion`
  and similarity/LLM-matched `memoryId`.
- **Global clustering, project-tagged**: a correction recurring in two repos
  collapses into one entry whose `projects[]` lists both; `["all"]` semantics fall
  out naturally from a cluster spanning many projects.
- **Approach 1** (embedding-cluster → per-cluster LLM), which *subsumes* a lexical
  fallback when no embedding model is present.

### Out of scope (explicit follow-ups)
- Wiring `recommendations.based_on.corrections` to these (now-stable) correction
  ids — that is L2-generator work.
- Any dismiss / lifecycle status on corrections (the mockup shows none; corrections
  are fully re-derived each run).
- UI wiring; the API-wide snake/camel contract-hygiene sweep (its own later step).

## Why these decisions

- **Why a dedicated table, not `detected_patterns`?** A correction cluster spans
  **multiple** projects (`projects[]`), so a one-row-per-scope pattern can't hold
  it — this holds whether `detected_patterns` stays folder-scoped or moves to
  project scope (the latter is its own change, #82). There's also no natural home
  in `detected_patterns` for the canonical text / suggestion / memory link.
  `recommendations.based_on` already reserves `corrections:[ref]`, so corrections
  need a stable identity to be referenceable.
- **Why a dedicated global task, not inside `analyze_project`?** The analyzer
  scheduler is per-project; global clustering needs cross-project data, and running
  it inside every per-project pass would re-cluster N times per cycle. A single
  global task per tick is correct and cheaper.
- **Why derive-and-store, not on-read?** The full shape needs an embedding pass +
  per-cluster LLM calls — far too heavy to run on every API read. This matches the
  L2-generator architecture (write during analyzer passes; API reads cheaply).
- **Why embeddings + LLM with graceful fallback?** Mirrors the established
  recall→precision + degrade-gracefully idiom (`derive_signals` regex recall →
  `classify_batch` LLM precision; the analyzer never blocks on a model).

## Data model

New DDL `database/ddl/table/inference/corrections.ddl` (full DDL,
`create table if not exists` → picked up by `dbd apply`; no ALTER in the file).

| column | type | purpose |
|---|---|---|
| `id` | uuid pk default `gen_random_uuid()` | surrogate key; stable across runs via upsert-on-signature |
| `signature` | text **not null unique** | deterministic cluster identity; natural key for idempotent upsert |
| `text` | text not null | canonical correction statement (LLM); fallback = representative member snippet |
| `suggestion` | text (null) | LLM advisory; null when no chat model |
| `count` | integer not null default 0 | number of corrective prompts in the cluster |
| `project_ids` | uuid[] not null default `{}` | distinct projects the correction appeared in (queryable; names resolved by the API) |
| `last_seen` | timestamptz | max ts across member prompts |
| `memory_id` | uuid → `sensei.memories(id)` on delete set null | LLM-matched learned memory; null if none |
| `instances` | jsonb not null default `[]` | provenance `[{project_id, session_id, ts, prompt: snippet}]` (drill-down + recompute) |
| `detected_at` | timestamptz not null default `now()` | first derivation time |
| `modified_at` | timestamptz not null default `now()` | last upsert time |

Indexes: `unique(signature)`; GIN on `project_ids`. Standard column comments per
house DDL style.

## Idempotency (the load-bearing detail)

Clusters are recomputed globally each run, so the signature must stay stable as a
cluster *grows*:

1. **Deterministic clustering** — corrective prompts are processed in `ts` order;
   each is assigned to the **first** existing cluster within the cosine threshold,
   else it seeds a new cluster. Embeddings are deterministic for fixed text, so
   this is reproducible.
2. **Earliest-by-ts member is the cluster seed.** Past events never change, so the
   seed is stable as later paraphrases join the cluster.
3. `signature = hash(seed.session_id + ":" + normalize(seed.prompt))`, computed
   from the seed **before** any LLM call — identical whether or not models run.
4. Write path: `INSERT … ON CONFLICT(signature) DO UPDATE` (keeps `id` stable),
   then `delete_corrections_not_in(current_signatures)` removes corrections that no
   longer recur or that merged.

### Recurrence threshold

The view is "**recurring** corrections," so a cluster must reach a minimum size to
surface: `count >= CORRECTION_CLUSTER_MIN` (tunable constant, **default 2**),
mirroring the existing `CORRECTION_MIN` gate on folder-level correction-prone
patterns. Singletons (a one-off correction) are not written. The threshold is
applied after clustering, before upsert; clusters below it are excluded from
`current_signatures` so they are also pruned if they previously qualified and then
fell out (they can't fall, since corrections only accrue — but the delete pass keeps
the rule uniform).

Because `id` is stable, the L2 generator can later reference a correction in
`recommendations.based_on.corrections` (follow-up).

## Pipeline

New `TaskKind::AggregateCorrections`, a global task. The per-project analyzer
scheduler enqueues it **once per tick** when at least one project was due (i.e.
there was new session activity), so it re-clusters only when corrections may have
changed and runs once globally rather than per project.

```
AggregateCorrections handler (tasks/handlers/corrections.rs):
  pg.get_all_user_prompts()                          // global: project_id, project_name, session_id, ts, prompt
  → correction_signal (regex recall)        ─┐  reuse existing L1 classifiers
  → classify_batch (LLM precision, graceful) ─┘  → corrective items only
  → embed_batch("embed" chain) over corrective texts
  → corrections::cluster(items, embeddings, τ)       // pure, deterministic
  → per cluster: corrections_llm::summarize(reps, memory_titles)  // {text, suggestion, memory_id}
  → pg.upsert_correction(signature, text, suggestion, count, project_ids, last_seen, memory_id, instances)
  → pg.delete_corrections_not_in(current_signatures)
```

The corrective-prompt set is filtered in Rust (regex recall over all user prompts,
then the existing `classify_batch` precision pass) so only corrections are
embedded — keeping the embedding pass small. This reuses the exact classification
code path from `derive_signals`; no reclassification logic is duplicated.

## Modules (isolation + testability)

Mirrors `pattern_effectiveness.rs` (pure module + tests) and `prompt_classify.rs`
(pure build/parse + graceful async call).

1. **`crates/senseid/src/corrections.rs`** — *pure, no IO, fully unit-tested*:
   - `normalize(prompt) -> String` — lowercase, collapse whitespace, strip
     punctuation, cap length.
   - `cosine(a: &[f32], b: &[f32]) -> f32`.
   - `cluster(items, embeddings, threshold) -> Vec<Cluster>` — deterministic greedy
     clustering in ts order. `threshold` (cosine τ) is a tunable constant alongside
     `CORRECTION_CLUSTER_MIN`.
   - `signature(seed_session, seed_prompt) -> String`.
   - `Cluster { signature, seed_idx, member_idxs, count, last_seen, project_ids, representative_text }`.
     `representative_text` = `normalize`-bounded snippet of the **seed** member (the
     earliest-by-ts prompt) — deterministic and consistent with the signature seed;
     it is the `text` fallback when no chat model runs.
   - `lexical_cluster(items) -> Vec<Cluster>` — the no-embedding fallback (group by
     `normalize()`), same `Cluster` output shape.

2. **`crates/senseid/src/tasks/handlers/corrections_llm.rs`** — mirrors
   `prompt_classify.rs`:
   - *pure*: `build_prompt(reps, memory_titles)`,
     `parse_response(content, memory_ids) -> Option<ClusterSummary>` (tolerates
     fences/prose; `memory_id` accepted only when it is in the supplied shortlist).
   - *async, graceful*: `summarize_cluster(gateway, …) -> Option<ClusterSummary>` —
     returns `None` when no chat model is configured or the call/parse fails.
   - `ClusterSummary { text: String, suggestion: Option<String>, memory_id: Option<Uuid> }`.

3. **`crates/senseid/src/tasks/handlers/corrections.rs`** — the IO orchestrator
   `aggregate_corrections(ctx) -> Result<u32, String>`. A pure assembly helper maps
   `(clusters, summaries) -> Vec<CorrectionRow>` so the row-building logic is
   unit-testable without IO.

## pg_store additions

- `get_all_user_prompts() -> Vec<(Uuid /*project_id*/, String /*project_name*/, Uuid /*session_id*/, DateTime<Utc> /*ts*/, String /*prompt*/)>`
  — global `UserPromptSubmit` prompts with project + timestamp.
- `get_learned_memories_for_matching() -> Vec<(Uuid, String /*title*/)>` — shortlist
  fed to the LLM for `memory_id` matching.
- `upsert_correction(...)` — `ON CONFLICT(signature) DO UPDATE`.
- `delete_corrections_not_in(signatures: &[String]) -> u32`.
- `list_corrections() -> serde_json::Value` and
  `list_corrections_for_project(project_id) -> serde_json::Value` (read path; resolve
  `project_ids` → `[{id, name}]`).

## API

New handler module `crates/senseid/src/api/handlers/corrections.rs`, registered in
`api/routes.rs`:

- `GET /api/corrections` — global list, ordered `count DESC, last_seen DESC`.
- `GET /api/projects/{id}/corrections` — corrections touching a project
  (`WHERE $id = ANY(project_ids)`).

Response (camelCase, matching `learnings-data.js`):
```json
{ "corrections": [
  { "id": "uuid", "text": "Rewrite `let x = …` → `$state(…)`",
    "count": 6, "lastSeen": "2026-06-24T12:00:00Z",
    "projects": [{ "id": "uuid", "name": "koto" }],
    "memoryId": "uuid|null", "suggestion": "…|null" }
] }
```
`lastSeen` is an ISO timestamp; the UI humanizes it ("today"/"yesterday"). New
endpoints are born camelCase; the API-wide case sweep remains a separate later step.

## Graceful degradation (never blocks the analyzer)

- No embed model → skip embeddings, use `lexical_cluster()` (group by `normalize()`).
- No chat model → `text` = representative member snippet, `suggestion` / `memory_id`
  = null.
- A per-cluster LLM failure is logged (`tracing::warn!` — no silent errors) and that
  cluster degrades individually; the pass continues and returns the count written.

## Testing (TDD; zero-errors-policy at both checkpoints)

- **`corrections.rs`** (pure): `cosine`; `normalize` idempotence; deterministic
  clustering (same input → same clusters & signatures); paraphrases within τ merge
  while distinct corrections stay separate; **signature stability when a cluster
  gains a later member**; `last_seen` = max; `project_ids` dedup; `lexical_cluster`
  grouping.
- **`corrections_llm.rs`** (pure): `build_prompt` bounded; `parse_response`
  valid/malformed/fenced JSON → graceful `None`; `memory_id` accepted only when in
  the shortlist.
- **Assembly** (pure): `(clusters, summaries) -> rows` mapper.
- **Integration** (gated on DB availability, mirroring existing tests): idempotency —
  run `aggregate_corrections` twice on fixed data → identical rows, no duplicates;
  `delete_corrections_not_in` prunes stale signatures.

## Delivery

- DDL applied via `dbd` (`SENSEI_DDL_DIR` locally during iteration; released to the
  daemon on `make bump`).
- Work on `develop`; commit when green (zero-errors-policy run before and after).
- Verify live: trigger the task, confirm rows in `inference.corrections` and both
  endpoints return the expected shape; re-run to confirm idempotency.
