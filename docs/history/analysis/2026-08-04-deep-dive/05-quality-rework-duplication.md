## Quality, Rework & Duplication — the un-scored objective

_Agents optimize for "task done," never "codebase healthier." sensei already records the wreckage — it just never scores it, enforces it, or feeds it back._

**User's observation.** From `2026-08-04-metrics.md`: _"Code quality and maintainability are generally non-goals for the agents, so there tends to be a lot of code duplication and redundancy… if user wants to design a big epic and then ask the model to run an unattended session it does not work well."_ And: _"Rework — opposite to FTR — covers repeated fixes on same feature/component."_ The proposed fix is a deterministic quality scan (qlty.sh) with project-level before/after snapshots and a new-vs-existing duplicate discrimination. The Observations doc adds the consumer-side symptom: the Patterns page _"can't make out what the pattern is / what to do."_

**What the data shows.**

- **Rework is the dominant "anti-pattern."** Of 943 `inference.detected_patterns`, 922 are literally named `rework: <path>` and 10 are `correction-prone`; only 11 are non-anti-pattern `rule-candidates`. There is **not one duplication-named pattern** — the whole anti-pattern corpus is a rework ledger keyed to file paths.

  ```sql
  SELECT CASE WHEN name LIKE 'rework:%' THEN 'rework: <path>' ELSE name END AS cls,
         count(*), sum(instance_count) FROM inference.detected_patterns GROUP BY cls ORDER BY 2 DESC;
  -- rework: <path> 922/922 | rule-candidates 11/19 | correction-prone 10/49
  ```

- **11,947 edits across 922 files; 11,025 of them (92.3%) are re-edits beyond the first write.** The detector fires at ≥5 edits/file (min observed = 5), averages 13 edits/file, and tops out at **301 edits on a single file** (`crates/senseid/src/db/pg_store.rs`).

  ```sql
  WITH rw AS (SELECT (jsonb_array_elements(instances)->>'total_edits')::int te
              FROM inference.detected_patterns WHERE name LIKE 'rework:%')
  SELECT count(*) files, sum(te) all_edits, sum(te)-count(*) beyond_first, max(te) worst FROM rw;
  -- 922 | 11947 | 11025 | 301
  ```

- **39.4% of reworked files (363/922) were churned across more than one session**, averaging 18.3 edits — i.e. the fix didn't stick, and a *later* session had to come back. This is the true "repeated fixes on same component" signal the user asked for. Instances carry `{file, total_edits, max_session_edits}`, so `total_edits > max_session_edits` cleanly isolates cross-session rework.

  ```sql
  WITH rw AS (SELECT (jsonb_array_elements(instances)->>'total_edits')::int te,
                     (jsonb_array_elements(instances)->>'max_session_edits')::int ms
              FROM inference.detected_patterns WHERE name LIKE 'rework:%')
  SELECT count(*) FILTER (WHERE te>ms) cross_session, count(*) all,
         round(100.0*count(*) FILTER (WHERE te>ms)/count(*),1) pct FROM rw;   -- 363 | 922 | 39.4
  ```

- **Rework is Pareto-tailed:** the worst 10% of files absorb **32.8% of all re-edits**. A quality budget that targets the top decile would recoup a third of the churn.

- **Rework density tracks FTR inversely across projects.** The project with the most rework files (`sensei`, 410) has the **lowest** FTR (43%) and the most corrections (20); `dbd` with 8× less rework (43 files) sits at **90% FTR**. Higher churn density ≈ lower first-time-right.

  | project | sessions | FTR % | corrections | rework files |
  |---|--:|--:|--:|--:|
  | dbd | 21 | 90 | 3 | 43 |
  | sensei | 16 | **43** | 20 | **410** |
  | torii | 12 | 73 | 5 | 136 |
  | gateway | 6 | 50 | 4 | 40 |
  | rokkit | 4 | 75 | 1 | 96 |
  | strategos | 2 | 50 | 2 | 122 |

  ```sql
  WITH rw AS (SELECT project_id, count(*) rf FROM inference.detected_patterns
              WHERE name LIKE 'rework:%' GROUP BY project_id),
       s AS (SELECT project_id, count(*) n,
                    round(100.0*count(*) FILTER (WHERE ftr)/nullif(count(*) FILTER (WHERE ftr IS NOT NULL),0),0) ftr,
                    sum(corrections) c FROM activity.sessions WHERE project_id IS NOT NULL GROUP BY project_id)
  SELECT p.name, s.n, s.ftr, s.c, coalesce(rw.rf,0) FROM s JOIN sensei.projects p ON p.id=s.project_id
  LEFT JOIN rw ON rw.project_id=s.project_id ORDER BY s.n DESC LIMIT 12;
  ```

  | file (top rework hotspots) | total edits | one-session max | other-session edits |
  |---|--:|--:|--:|
  | sensei/crates/senseid/src/db/pg_store.rs | 301 | 98 | 203 |
  | sensei/docs/llm-spec/park/_run-state.md | 210 | 197 | 13 |
  | sensei/crates/mcp/src/lib.rs | 90 | 22 | 68 |
  | sensei/crates/senseid/src/api/routes.rs | 80 | 30 | 50 |
  | strategos/gateway/crates/gateway/src/engine.rs | 70 | 60 | 10 |

- **Duplication is real and the daemon can already see it — but nothing consumes the signal.** The live `get_duplicates` MCP tool returns 50 embedding-near-duplicate pairs at ≥0.92 similarity in the `sensei` project alone (371 folders scanned): `as_db_str` is copied 6× inside one file (`crates/dojo-protocol/src/relay.rs`, similarity 1.0), and `design-canvas.jsx` exists as **9 separate file copies**.

  ```sql
  SELECT split_part(file_path,'/',array_length(string_to_array(file_path,'/'),1)) base,
         count(DISTINCT file_path) copies FROM sensei.nodes
  WHERE kind IN ('component','function') AND file_path LIKE '%.jsx'
  GROUP BY base HAVING count(DISTINCT file_path)>2 ORDER BY 2 DESC;
  -- design-canvas.jsx 9 | tweaks-panel.jsx 5 | App.jsx 3
  ```

- **Deterministic cross-file symbol collisions confirm it at scale:** 1,657 `function` names and 293 `hook` names recur across multiple files in the same project (methods = 10,777 groups, but that count is inflated by Rust trait impls like `fmt`/`new` and is noisy). Stripping framework conventions (`GET`/`POST`/`load`), the genuinely-duplicated helpers stand out: `validateRequestBody` (37 files), `makeReq` (31), `responseStub` (26), `fetcher` (19), `logEvent` (16) — classic copy-paste helpers that belong in a shared module.

  ```sql
  SELECT name, count(DISTINCT file_path) files FROM sensei.nodes WHERE kind='function'
  GROUP BY name HAVING count(DISTINCT file_path)>3 ORDER BY 2 DESC LIMIT 20;
  ```

- **There is zero deterministic quality signal stored anywhere.** A scan of every column in `sensei/activity/inference/history/gateway` for `complexity|coverage|lint|qualit|maintainab|duplicat|churn|cyclomatic|loc|sloc` returns **three false positives** (`local_path`) and nothing real. sensei has no complexity, no coverage, no lint-count, no quality score — so no quality *delta* can be computed today.

  ```sql
  SELECT table_schema,table_name,column_name FROM information_schema.columns
  WHERE table_schema IN ('sensei','activity','inference','history','gateway')
    AND lower(column_name) ~ 'complexity|coverage|lint|qualit|maintainab|duplicat|churn|cyclomatic|loc|sloc';
  -- only libraries.local_path, library_pages.local_path, project_libraries_resolved.local_path
  ```

- **Every pattern is a bare, un-enforceable label.** All 943 rows are `lifecycle='suggested'`, with `enforcement`, `severity`, `fix_pattern_id`, `example`, and `description` **NULL for 100% of them**. That is exactly why the Patterns page "can't make out what the pattern is / what to do" — the daemon literally stores no what-to-do.

  ```sql
  SELECT count(*) FILTER (WHERE lifecycle='suggested') suggested, count(enforcement) enf,
         count(fix_pattern_id) fix, count(example) ex, count(description) descr FROM inference.detected_patterns;
  -- 943 | 0 | 0 | 0 | 0
  ```

- **The insight loop is write-only. The action rate is 0.07%.** Rework *is* surfaced as recommendations — 566 of 1,478 are `audit_stale` titled _"High rework: <path> — Re-edited heavily within a single session, a churn hotspot"_ — but only **1 recommendation across the entire history has ever been acted on** (`status='accepted'`, `acted_at` non-null); 1,477 sit `pending`.

  ```sql
  SELECT status, count(*), count(acted_at) acted FROM inference.recommendations GROUP BY status;
  -- pending 1477 (0 acted) | accepted 1 (1 acted)
  ```

- **Nothing consolidates into durable memory.** 932 anti-patterns + 566 rework recommendations + 50 live duplicate pairs distil to **11 memories, all `type=convention`, none about rework or duplication**, with `violated_count=0` across the board (so violations are never counted against them either).

  ```sql
  SELECT type, count(*), sum(violated_count) FROM sensei.memories GROUP BY type;  -- convention 11 | 0
  ```

- **Users are the duplication detector of last resort.** 57 of 1,517 user transcript turns (3.8%) flag duplication/redundancy/reuse manually ("_the ask rokkit nav is **redundant**_", "_why are you **repeating** the same sentence?_"), and these show up verbatim inside the `correction-prone` pattern instances. 19/69 sessions (27.5%) ended `outcome='corrected'`, carrying 38 corrections; the correction turns triage as revert=18, correction=17, why=3.

  ```sql
  SELECT count(*) FILTER (WHERE lower(user_text) ~ 'duplicat|redundan|reuse|already exists|repeated|copy.paste') hits,
         count(*) total FROM activity.transcript_turns WHERE user_text IS NOT NULL;  -- 57 | 1517
  ```

**Root cause / interpretation.**

The agent's objective function ends at "the task compiles and the user stopped complaining." Nothing in the loop rewards *fewer future edits* or *less duplicated surface area*, so the model spends edits freely — `pg_store.rs` absorbed 301 of them, 203 in sessions *after* the one that first churned it. sensei observes this perfectly: the daemon's analyzer already computes per-file `total_edits`/`max_session_edits` and writes a `rework:` pattern plus an `audit_stale` recommendation. The failure is that this is a *passive ledger*. It is produced after the fact, keyed to a path, with no `description`, no `example`, no `fix_pattern_id`, and it is never injected back into the next session's context. The agent that will re-edit `pg_store.rs` for the 302nd time is never told it is standing on the single worst churn hotspot in the repo.

Duplication has the same shape but is worse, because sensei has a *deterministic and semantic* detector it doesn't use in the write path. `get_duplicates` (embedding cosine over `sensei.nodes.embedding`) and a trivial `GROUP BY name` over the code graph both find real copy-paste — 9 copies of `design-canvas.jsx`, `validateRequestBody` re-implemented in 37 files. But that detector only runs when a human invokes `/sensei:review` after the fact. At *write* time — when the agent is about to create a 38th `validateRequestBody` — the daemon knows the symbol already exists 37 times and says nothing. The user's precise ask ("a duplicated code created in the day needs to be compared against old code to identify if a duplicate was introduced or is existing") is answerable *today* from the graph: a new node whose name/embedding collides with an existing node in another folder is a *new* duplicate; a pre-existing collision is *existing* debt. sensei has both sides of that comparison and never runs it.

The consolidation gap explains why none of this improves over time. 943 patterns and 1,478 recommendations produce **1 action and 0 durable quality memories**. The recommendation surface is the app equivalent of a log file nobody tails. Because every pattern is `lifecycle='suggested'` with a NULL body, the Patterns screen can only render an opaque label — which is exactly the Observations complaint. There is no promotion path from "rework hotspot observed 6 times across sessions" → "enforced convention: extract shared helper before the 3rd copy," so the same mistake re-enters every unattended run. Quality is not merely un-scored; it is un-remembered.

Finally, the absence of any deterministic quality column means sensei cannot yet answer the user's headline question — *did this session make the code better or worse?* FTR and rework are behavioral proxies (did the human revert / re-edit), not code-health measurements. A qlty.sh (or `scc` + `rust-code-analysis` + `eslint --format json`) snapshot at session start and end is the missing anchor that turns "27.5% of sessions were corrected" into "this session raised duplication by 2 clones and cyclomatic complexity by 14."

**Recommendations.**

1. **(P0) Duplication guard in the write path — `dry-check` on PreToolUse(Write/Edit).** When an agent writes a new top-level symbol, have the MCP layer look it up against `sensei.nodes` (exact name match within project + `get_duplicates` embedding neighbor ≥0.92). If a match exists in another file, return a soft block: _"`validateRequestBody` already exists in 37 files — import it or justify a new one."_ Build: `crates/mcp` + a new daemon endpoint over `sensei.nodes`/`edges`; reuse the existing `get_duplicates` cosine query. Expected effect: converts the 50 known duplicate pairs and 1,657 recurring function names from after-the-fact findings into pre-write prevention, and directly implements the user's new-vs-existing discrimination (existing collision = warn; brand-new collision = block).

2. **(P0) Inject the rework hotspot list into session context.** At session start, `get_layered_context`/`context_pack` should include the top-N `rework:` files for the project ("these 5 files are churn hotspots — change them surgically, add a test before editing"). Build: extend the context-pack loader in `crates/senseid` to read `inference.detected_patterns WHERE name LIKE 'rework:%' ORDER BY total_edits DESC`. Expected effect: pre-loads the knowledge that today only exists as an unread `audit_stale` recommendation; targets the top decile that carries 32.8% of all re-edits.

3. **(P1) `quality-scan-and-resolve` skill wired after code-review.** Run a deterministic scanner (qlty.sh / `scc` for LOC+complexity, language linters for `--format json`) at session start and end, diff the two, and persist a `quality_delta`. This is the missing deterministic anchor. Build: new skill in `marketplace/`, results into a new `activity.quality_snapshots(session_id, project_id, taken_at, phase, sloc, complexity, dup_blocks, lint_count, coverage)` table. Expected effect: makes "did this session improve the code?" answerable per session/day/project; feeds the quality-delta metric below.

4. **(P1) Give patterns a body and a lifecycle, then promote.** Backfill `description`/`example`/`fix_pattern_id` for anti-patterns (a rework hotspot's fix is "extract/stabilize; add regression test"), and add a promotion job: a `rework:` file seen in ≥3 sessions, or a duplicate cluster of ≥4 copies, becomes a `sensei.memories` convention with `enforcement`. Build: `inference` analyzer + the existing `promote_memory` MCP path. Expected effect: closes the 943→11 consolidation gap and fixes the Patterns-page "can't tell what to do" complaint at the data layer.

5. **(P2) Make the recommendation surface actionable, and measure the action rate.** With a 0.07% action rate, `audit_stale`/rework recommendations need a one-click "resolve" (schedule a refactor task / open an issue / snooze) and an `acted`/`dismissed` outcome written back. Build: recommendations screen in `app/` + `record_outcome`. Expected effect: turns the write-only ledger into a loop; the action-rate metric becomes a real KPI instead of a rounding error.

**Proposed metrics & instrumentation.**

| Metric | Definition / formula | Source (table.column) | Cadence | Current gap |
|---|---|---|---|---|
| Rework rate (file) | `(Σ total_edits − files) / Σ total_edits` = re-edits ÷ all edits | `inference.detected_patterns.instances[].total_edits` (name LIKE `rework:%`) | project / daily | Computable now (=92.3%); not surfaced as a metric |
| Cross-session rework % | `count(total_edits > max_session_edits) / count(*)` | same instances jsonb | project / weekly | Computable now (=39.4%); not surfaced |
| Rework concentration (Pareto) | share of re-edits in top-decile files | same | project | Computable now (=32.8%); not surfaced |
| Session rework outcome | `sessions[outcome='corrected'] / sessions`; corrections/session | `activity.sessions.outcome`, `.corrections` | session / daily | Present (27.5%); no per-feature keying |
| Duplication ratio (new) | new symbols this session whose name/embedding matches an existing node in another file ÷ new symbols | `sensei.nodes` (name, embedding) diffed by `modified_at` | session | Detector exists (`get_duplicates`); no new-vs-existing diff persisted |
| Duplication debt (existing) | count of symbol clusters (size ≥2) across files per project | `sensei.nodes` GROUP BY name/kind; `get_duplicates` | project / weekly | Computable now; never stored or trended |
| Anti-pattern density | anti-patterns ÷ KLOC per project | `inference.detected_patterns` ÷ SLOC | project | Blocked — no SLOC/LOC column exists |
| Quality delta / session | `end_snapshot − start_snapshot` for complexity, dup_blocks, lint, coverage | proposed `activity.quality_snapshots` | session (start/end) | **No deterministic quality data stored at all** |
| Enforcement action rate | `recommendations[acted_at NOT NULL] / recommendations` | `inference.recommendations.status`, `.acted_at` | weekly | Present but ≈0.07% — no resolve UI/outcome writeback |
| Consolidation ratio | durable quality memories ÷ (anti-patterns + rework recs) | `sensei.memories` vs `detected_patterns`/`recommendations` | monthly | 0 quality memories today; no promotion job |
