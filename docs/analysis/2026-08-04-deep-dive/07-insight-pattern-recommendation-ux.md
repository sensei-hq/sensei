## Insights, Patterns & Recommendations: Generated in Bulk, Unreadable and Unactionable

_sensei mines 943 patterns, 1,478 recommendations, and 273 open drift items — but the surfaces render machine names with no descriptions, no dedup, no priority, and no action path. The insight engine writes; nothing graduates._

**User's observation.** From `Observations.md`: on the Patterns page — _"Can't make out what the pattern is and what to do with it. Should be user friendly."_ On Traceability — _"Traceability repeats the same observation multiple times. Can't see detail and can't take any action."_ On Libraries — _"Libraries shows version conflict. Would be nice if we can generate a prompt as handoff action, or send to intake page from here … (version conflict is barely visible)."_ Three screens, one complaint: the insight is present in the data but arrives as an opaque, repeated, dead-end row.

**What the data shows.**

- **Patterns carry zero human-readable content.** All 943 rows have `description IS NULL`, `family IS NULL`, `severity IS NULL`, `enforcement IS NULL`, and `example IS NULL`. The `'No description captured'` string the user sees is an **app-side fallback**, not stored data. This is the literal root of "can't make out what the pattern is" — there is nothing to render.

  ```sql
  SELECT count(*) total,
         count(*) FILTER (WHERE family IS NULL)      family_null,
         count(*) FILTER (WHERE description IS NULL)  desc_null,
         count(*) FILTER (WHERE severity IS NULL)     sev_null,
         count(*) FILTER (WHERE enforcement IS NULL)  enf_null
  FROM inference.detected_patterns;
  -- 943 | 943 | 943 | 943 | 943
  ```

- **Every pattern is stuck at `lifecycle = 'suggested'`.** None promoted, none rejected, none enforced. The promotion path exists — 16 `promote_pattern` recommendations were generated — but 0 were acted on, so no pattern ever graduates to a convention.

  ```sql
  SELECT lifecycle, count(*) FROM inference.detected_patterns GROUP BY 1;
  -- suggested | 943
  ```

- **The "pattern" name space is 98% file-path rework labels, not patterns.** 922/943 names are `rework:<abs-path>`; only 21 are conceptual (`rule-candidates` ×11, `correction-prone` ×10). The rework rows have `instance_count = 1.0` on average — **no rollup**; each churned file is its own row.

  | name prefix        | count | avg confidence | avg instance_count |
  |--------------------|-------|----------------|--------------------|
  | `rework:<file>`    | 922   | 0.774          | 1.0                |
  | `rule-candidates`  | 11    | (null)         | 3.2                |
  | `correction-prone` | 10    | (null)         | 3.2                |

  ```sql
  SELECT split_part(name,':',1) prefix, count(*) FROM inference.detected_patterns GROUP BY 1 ORDER BY 2 DESC;
  ```

- **The instances jsonb is rich — the renderer just ignores it.** A rework pattern already stores `{"file": …, "total_edits": 9, "max_session_edits": 8}`; `correction-prone` stores the offending prompts + sessions. All 943 rows have populated `instances`, yet 0 have `evidence`. A one-line description ("`hooks.spec.svelte.ts` re-edited 9× in a session, 8 in one sitting") is fully derivable from data the row already holds.

- **Recommendations are a write-only pile: 1 of 1,478 acted on (0.07%).** 1,477 `pending`, 1 `accepted`; verdict identical (1,477 `pending`, 1 `positive`). Only 19 are `focal`.

  | status   | count | verdict  | count |
  |----------|-------|----------|-------|
  | pending  | 1477  | pending  | 1477  |
  | accepted | 1     | positive | 1     |

  ```sql
  SELECT status, count(*) FROM inference.recommendations GROUP BY 1;
  SELECT count(acted_at) acted, count(*) FILTER (WHERE focal) focal FROM inference.recommendations;  -- 1 | 19
  ```

- **94% of recommendations have no executable handoff.** Only 87/1,478 (5.9%) carry a `prompt`; `impact` is set on 3; and `action_detail`, `evidence`, `default_acp` are populated on **0** rows. So even where the user wants to "generate a prompt as handoff action," 94% of rows have no prompt and no structured payload to hand off.

  ```sql
  SELECT count(*) FILTER (WHERE prompt IS NOT NULL) has_prompt,          -- 87
         count(*) FILTER (WHERE impact IS NOT NULL) has_impact,          -- 3
         count(*) FILTER (WHERE action_detail::text NOT IN ('{}','null')) has_detail, -- 0
         count(*) FILTER (WHERE default_acp IS NOT NULL) has_acp         -- 0
  FROM inference.recommendations;
  ```

- **Recommendations are not measurably moving FTR — the loop is empty.** `baseline_ftr` is populated on **0** rows; `current_ftr` on 1; `measured_at` on 1. The one acted recommendation ("Architectural stability review needed for 'rokkit' mono-repo setup", `enrich_memory`, acted 2026-07-15, measured 2026-07-31) shows `current_ftr = 1.000` but a **NULL baseline** — so even the single success cannot compute a delta.

  ```sql
  SELECT count(baseline_ftr) base, count(current_ftr) cur, count(measured_at) meas
  FROM inference.recommendations;  -- 0 | 1 | 1
  ```

- **The same signal is double-surfaced across two screens.** 502 files appear as BOTH a `rework:<file>` **pattern** and a `High rework:<file>` **recommendation** — the churn hotspot is counted once on Patterns and again on Recommendations. All 566 `audit_stale` recs share one templated `why`: _"Re-edited heavily within a single session — a churn hotspot worth auditing."_

  ```sql
  WITH pat AS (SELECT replace(name,'rework: ','') f FROM inference.detected_patterns WHERE name LIKE 'rework:%'),
       rec AS (SELECT replace(title,'High rework: ','') f FROM inference.recommendations WHERE title LIKE 'High rework:%')
  SELECT count(*) FROM (SELECT f FROM pat INTERSECT SELECT f FROM rec) x;  -- 502
  ```

- **Drift repeats the same observation 64% of the time.** 273 `broken` drift items but only 98 distinct `detail` strings — 175 are exact repeats. `expected_signature` and `actual_signature` are 100% NULL, so the only content is the `detail` line, and the same doc-symbol drift fans out across every doc node that mentions it.

  | detail                                              | occurrences |
  |-----------------------------------------------------|-------------|
  | ``Mentions `InferenceAdapter` which is not in the code.`` | 35 |
  | ``Mentions `ProviderError` which is not in the code.``    | 17 |
  | ``Mentions `onMount` which is not in the code.``          | 17 |
  | ``Mentions `access_groups` which is not in the code.``    | 11 |
  | ``Mentions `router_keys` which is not in the code.``      | 10 |

  ```sql
  SELECT count(*) broken, count(distinct detail) distinct_detail
  FROM inference.drift_items WHERE status='broken';  -- 273 | 98
  ```

- **113 libraries are pinned at conflicting versions — with no handoff affordance in the schema.** `typescript` alone spans 13 versions across 21 folders; `@rokkit/core` mixes `1.0.0-next.145`, `1.3.6`, `workspace:*`, and `workspace:latest`. Neither `referenced_libraries` nor `libraries` has any resolution/handoff/action column — the conflict is computed at read time and dead-ends. The 806 `library_update` recommendations that could act on it have a `why` but **no `impact`, no `prompt`, no `action_detail`**.

  | library      | eco | distinct versions | folders |
  |--------------|-----|-------------------|---------|
  | typescript   | npm | 13                | 21      |
  | eslint       | npm | 11                | 14      |
  | svelte       | npm | 10                | 16      |
  | vitest       | npm | 9                 | 13      |
  | @rokkit/core | npm | 6                 | 12      |

  ```sql
  SELECT count(*) FROM (
    SELECT library_id FROM sensei.referenced_libraries
    WHERE version_used IS NOT NULL
    GROUP BY library_id HAVING count(distinct version_used) > 1) t;  -- 113
  ```

- **Consolidation gap: ~2,700 raw insight rows, 11 durable memories.** The system emits 943 patterns + 1,478 recommendations + 273 broken drift + 1 correction and distils them into 11 `sensei.memories` (all `type=convention`). The funnel from observation → durable, reusable knowledge is roughly 0.4%.

**Root cause / interpretation.**

The insight tables are being written as *event logs*, not as *rendered artifacts*. `inference.detected_patterns` was clearly designed for readable output — it has `description`, `example`, `family`, `severity`, `enforcement` columns — but the daemon's pattern-miner populates only the machine fields (`name`, `instances`, `confidence`, `is_anti_pattern`) and leaves every human-facing column NULL. The app then has nothing to show and falls back to `'No description captured'`. The fix is not a UI fix; the write path must synthesize a description at insertion time. The data to do so is already in `instances` (edit counts, files, offending prompts), so no new collection is required — only a formatting step the miner currently skips.

The repetition the user sees on Traceability and Drift is a **missing GROUP BY at the model layer**. Drift is stored per `(doc_node_id, symbol)`: when `InferenceAdapter` is renamed, every one of 35 docs that referenced it spawns an identical `broken` row. `rework:<file>` patterns and `High rework:<file>` recommendations are two projections of the same churn statistic, materialized independently into two tables (502 files carry both). Nothing rolls these up into "1 symbol drifted across 35 docs" or "these 12 files are your churn hotspots." The tables have no `rollup_of`/`superseded_by` edge and recommendations have no dedup key, so identical observations accumulate. Because `recommendations` also lacks a `created_at` column, the pile cannot even be aged — "unactioned-recommendation age," the natural triage sort, is not computable from the current schema.

The action dead-end is structural. The recommendation model *has* the fields for a handoff — `prompt`, `action_detail`, `default_acp` — but they are unpopulated on essentially every row, so the app can only render a title and a templated `why`. There is no button to press because there is no payload behind it. The FTR feedback loop that would justify a recommendation's existence (`baseline_ftr` → act → `measured_at` → `current_ftr`) is populated on 0/1/1 rows respectively; recommendations are generated but never causally tied to an outcome, so the engine has no signal about which of its 1,478 outputs were worth surfacing. This is why priority collapses to a coarse three-bucket `urgency` (medium 960 / low 373 / high 145) instead of a learned score.

Finally, the library version-conflict surface is a *derived view with no home in the write model*. The conflict is real and large (113 libraries), but there is no `sensei.library_conflicts` table, no resolution status, and no link from a conflict to the `library_update` recommendation that would fix it — so the user's exact request ("generate a prompt as handoff, or send to intake") has nowhere to attach. The affordance is missing because the schema has no object to hang it on.

**Recommendations.**

1. **(P0) Require a non-null `description` at pattern write-time; synthesize it from `instances`.** In the daemon's pattern-miner (`inference.detected_patterns` insert path), add a `NOT NULL` on `description` and a formatter: rework → _"`<file>` re-edited `total_edits`× in one session (peak `max_session_edits`) — churn hotspot"_; `correction-prone` → the top corrected prompt + count. Backfill the existing 943. **Effect:** the Patterns screen renders meaning instead of `'No description captured'`; directly closes "can't make out what the pattern is."

2. **(P0) Roll up drift by symbol before it reaches the UI.** Group `inference.drift_items` on `(folder_id, detail)` (or a derived `symbol`) into one parent with a `doc_node_id[]` list; render "`InferenceAdapter` missing — referenced by 35 docs" with an expandable list. Add a `superseded_by`/`rollup_of` self-FK so children collapse. **Effect:** 273 broken rows → ~98 distinct actionable items; kills the Traceability "repeats the same observation" complaint. Cut the same rollup for `rework` so 502 double-surfaced files appear once.

3. **(P0) Populate `prompt`/`action_detail` on every recommendation, or don't emit it.** Make the generator fill an executable payload (the `library_update` case is trivial: `bump <pkg> <from>→<to>`; `audit_stale` → "open `<file>`, review churn"). Enforce `prompt IS NOT NULL` for any `action_type` intended to be actionable. **Effect:** every recommendation becomes a one-click handoff; raises the 5.9%-with-prompt figure to ~100% and makes the Recommendations screen a work queue instead of a log.

4. **(P1) Add `sensei.library_conflicts` with a resolution status and a "generate handoff prompt / send to intake" action.** Materialize the 113-conflict view into a first-class table keyed by `library_id`, holding the version set + folder list + a `status` (open/resolving/resolved) and a link to the fixing `library_update` recommendation. Surface a "Send to intake" button on the Libraries screen that emits the prompt. **Effect:** delivers the user's exact ask and makes the "barely visible" conflict a triageable object.

5. **(P1) Add `created_at` to `inference.recommendations` and a learned `score`, then sort by it.** Without a creation timestamp, staleness/age triage is impossible; add it and backfill from the source reasoning trace. Replace the 3-bucket urgency sort with `score DESC` (the index `recommendations_score_idx` already exists — `score` is populated on 867 rows). **Effect:** the 1,477-deep pending queue becomes prioritizable; oldest-unactioned surfaces to the top.

6. **(P1) Close the FTR loop so recommendations prove their worth.** On accept, snapshot `baseline_ftr` from `activity.sessions`; on `measured_at`, recompute `current_ftr` and store the delta. Feed the delta back as the training signal for `score`. **Effect:** the engine learns which of its outputs actually moved FTR and can suppress the 806 low-value `library_update` bumps that no one acts on.

7. **(P2) Wire the promotion path so patterns graduate.** The 16 `promote_pattern` recommendations already exist but 0 were acted; add an inline "Promote to convention" affordance on the Patterns screen that flips `lifecycle` and writes a `sensei.memories` row. **Effect:** begins closing the 2,700→11 consolidation gap; patterns stop being a write-only log.

**Proposed metrics & instrumentation.**

| Metric | Definition / formula | Source (table.column) | Cadence | Current gap |
|--------|----------------------|-----------------------|---------|-------------|
| Insight action rate | `acted / total` recommendations | `inference.recommendations.acted_at` | daily | 1/1478 = 0.07% today |
| Description completeness | `patterns with non-null description / total` | `inference.detected_patterns.description` | project | 0/943 = 0% (all NULL) |
| Handoff readiness | `recs with non-null prompt / total` | `inference.recommendations.prompt` | daily | 87/1478 = 5.9% |
| Drift dedup ratio | `distinct detail / broken count` | `inference.drift_items.detail` (status=broken) | daily | 98/273 = 0.36 (64% redundant) |
| Cross-surface duplication | files present in both `rework:` patterns and `High rework:` recs | `detected_patterns.name` ∩ `recommendations.title` | weekly | 502 files double-surfaced |
| Pattern promotion rate | `patterns with lifecycle<>'suggested' / total` | `detected_patterns.lifecycle` | project | 0/943 = 0% |
| Version-conflict count | libraries with >1 distinct `version_used` | `referenced_libraries.version_used` | daily | 113; no resolution field |
| FTR movement per rec | `current_ftr − baseline_ftr` on acted recs | `recommendations.baseline_ftr/current_ftr` | per-rec on measure | baseline populated 0/1478 — uncomputable |
| Unactioned-rec age | `now() − created_at` for pending recs | `recommendations.created_at` | daily | **column does not exist** — must be added |
| Consolidation ratio | `durable memories / raw insight rows` | `sensei.memories` ÷ (patterns+recs+broken drift) | weekly | 11/2694 ≈ 0.4% |
