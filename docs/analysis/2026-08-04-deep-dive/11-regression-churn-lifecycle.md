## Regression & Churn — the missing lifecycle metrics

_FTR measures whether work landed clean the first time. It says nothing about work that landed clean and later broke (regression), or work the pipeline keeps re-touching without progress (churn). Both are measurable today and neither has a home in the app._

**Context.** The verified baseline flagged three shapes that FTR alone hides: 273 unresolved `broken` drift items, 19 `corrected` sessions, and a bulk-index cohort that never got community assignment. Digging in, the dominant story is not a one-off — it is *churn*: the indexer is re-processing the same files tens to thousands of times, 92% of that work produces zero graph changes, and it began the day the version-rescan feature (D2) shipped. Regression, by contrast, is real but small and concentrated in a handful of doc-heavy repos and one recurring source module (`senseid/src/db`).

Treat REGRESSION (something that worked later breaks) and CHURN (repeated modification without settling) as first-class lifecycle issues sitting at the opposite ends of FTR/rework, each with its own metric family and a health screen.

---

### What the data shows

**Regression signals**

- **Drift is 14% unresolved and highly concentrated.** 273 of 1,947 drift rows are `broken` (current status); the rest are `current` (resolved). But broken-rate is bimodal by repo — `torii` 97.9%, `alert-platform` 100%, `gateway` 96.0%, `dbd` 73.9% versus `rokkit` 3.0%, `swarco` 0.7%, `website` 0.0%. A few repos carry essentially all the doc-drift debt.

  ```sql
  SELECT f.path,
    count(*) FILTER (WHERE d.status='broken')  AS broken,
    count(*) FILTER (WHERE d.status='current') AS resolved,
    round(100.0*count(*) FILTER (WHERE d.status='broken')/count(*),1) AS broken_pct
  FROM inference.drift_items d JOIN sensei.folders f ON f.id=d.folder_id
  GROUP BY f.path ORDER BY broken DESC LIMIT 10;
  ```

| folder | broken | resolved | broken_pct |
|--------|-------:|---------:|-----------:|
| torii | 95 | 2 | 97.9 |
| sensei | 55 | 335 | 14.1 |
| gateway | 48 | 2 | 96.0 |
| rokkit | 26 | 842 | 3.0 |
| dbd | 17 | 6 | 73.9 |
| kavach | 15 | 197 | 7.1 |
| alert-platform | 15 | 0 | 100.0 |

- **The drift table itself churns 4×.** 1,947 rows cover only 487 distinct `(doc_node_id, code_node_id)` pairs — detection *inserts* a fresh row per scan instead of upserting, so the same doc is re-recorded ~4× on average. `scan_doc_drift` ran 832 times over 12 folders. The physical table is a churn artifact, not a clean state store.
  <br>`SELECT count(*), count(DISTINCT (doc_node_id,code_node_id)) FROM inference.drift_items;` → `1947 | 487`.

- **True re-breaks (flip current→broken→current) exist and point at live docs.** Grouping each doc by how many times it was recorded `broken` vs `current`: `docs/analysis/2026-07-27-mock-vs-impl-gap-analysis.md` flipped to `broken` **15 times** and `current` twice over 2026-07-27→30; `docs/spec/park/_dojo-build-plan.md` 7 broken / 5 current (2026-07-14→24); `docs/design/03-forms.md` 2 broken / 37 current. These are the real "worked, then broke again" regressions — a doc kept diverging from code faster than it was reconciled.

  ```sql
  SELECT n.file_path,
    count(*) FILTER (WHERE status='broken')  AS times_broken,
    count(*) FILTER (WHERE status='current') AS times_current
  FROM inference.drift_items d LEFT JOIN sensei.nodes n ON n.id=d.doc_node_id
  GROUP BY n.file_path
  HAVING count(*) FILTER (WHERE status='broken')>0
     AND count(*) FILTER (WHERE status='current')>0
  ORDER BY times_broken DESC LIMIT 5;
  ```

- **Reopened work: one source module was corrected three separate times.** `senseid/src/db` shows `corrected` sessions on 2026-07-07 (255 turns, 6 corrections), 2026-07-13 (102 turns), and 2026-08-01 (75 turns) — the same module reopened across 25 days. `senseid/src/api/handlers` was corrected on both 2026-07-27 and 2026-07-29. Two folder/module cells show the strict pattern "completed earlier, corrected later": `sensei .../dojo/src/lib/components/kit` (2 completed, 1 corrected) and `dbd .../dbd-core/src` (2 completed, 1 corrected) — the cleanest reopened-work regression signal in the session data.

  ```sql
  SELECT f.path AS folder, s.module,
    count(*) FILTER (WHERE outcome='completed') completed,
    count(*) FILTER (WHERE outcome='corrected') corrected
  FROM activity.sessions s LEFT JOIN sensei.folders f ON f.id=s.folder_id
  GROUP BY f.path, s.module
  HAVING count(*) FILTER (WHERE outcome='completed')>0
     AND count(*) FILTER (WHERE outcome='corrected')>0;
  ```

- **FTR is volatile day-over-day but the swings are low-N noise.** Day deltas hit −80.8pp (2026-08-01: a single `corrected` db-module session → 0% FTR), −66.7pp (2026-07-26), −50pp (2026-07-20, 2026-07-29). With 1–6 sessions on most days, one corrected session tanks the daily rate. Only 2026-07-31 (26 sessions, 80.8%) has the sample size to be a real number. A day-over-day FTR "regression" alert is meaningless until N-per-day is gated.

- **All 3 crashed runs exhausted recovery, and 2 re-attempt the same failing goal.** Every `crashed` run has `recovery_attempts=3`. Two of the three share `plan_ref = docs/analysis/2026-07-24-auto-buildout-readiness.md` (the "P1 phase bridge" / "Relay supervision live-drive" goals) — a goal re-attempted the same day after failing. Their event streams are 601 `housekeeping` events, 14 `stalled`, 11 `recovered`, 3 `crashed` — the runs cycled stall→recover→crash rather than progressing.

  ```sql
  SELECT r.status, e.kind, count(*) FROM activity.run_events e
  JOIN activity.runs r ON r.id=e.run_id WHERE r.status='crashed'
  GROUP BY 1,2 ORDER BY 3 DESC;
  ```

**Churn signals**

- **The indexer re-processes files 13.8× on average and 92% of that work is inert.** `process_file` has 924,709 executions over 67,049 distinct paths since 2026-06-17, and **851,974 (92.1%)** returned `items_processed = 0` — the file was dispatched, opened, and produced no graph change. This is the single largest waste signal in the daemon.

  ```sql
  SELECT count(*) total, count(*) FILTER (WHERE items_processed=0 OR items_processed IS NULL) zero,
    round(100.0*count(*) FILTER (WHERE items_processed=0 OR items_processed IS NULL)/count(*),1) zero_pct,
    round(count(*)::numeric/count(DISTINCT path),1) avg_reprocess
  FROM activity.task_executions WHERE task_kind='process_file';
  ```

- **71% of all indexing work is spent on binary blobs that yield nothing.** Splitting `process_file` by extension class: **1,218 binary/artifact files absorbed 652,154 executions (535 per file)** while 65,831 source/text files took 272,555 (4.1 per file). The extreme per-file offenders are certs and media: `.p12` 6,618×/file, `.epub` 6,345×, `.psd` 6,170×, `.jpg` 5,343×, `.lockb` 2,351×. None of these produce nodes.

  ```sql
  SELECT lower(substring(path from '\.([A-Za-z0-9]+)$')) ext,
    count(*) executions, count(DISTINCT path) files,
    round(count(*)::numeric/count(DISTINCT path),1) per_file
  FROM activity.task_executions WHERE task_kind='process_file' AND path<>''
  GROUP BY 1 ORDER BY executions DESC LIMIT 10;
  ```

| ext | executions | files | per_file |
|-----|-----------:|------:|---------:|
| p12 | 119,130 | 18 | 6,618.3 |
| ckp / si / tlog | 65,110 ea | 170 ea | 383.0 |
| cfs / cfe | 61,280 ea | 160 ea | 383.0 |
| jpg | 37,403 | 7 | 5,343.3 |
| epub | 25,383 | 4 | 6,345.8 |
| lockb | 14,104 | 6 | 2,350.7 |
| **ts** (source) | 13,822 | 8,068 | **1.7** |
| **java** (source) | 12,806 | 8,925 | **1.4** |

- **Churn is Pareto-extreme: the top 20% of files are 94% of the work.** Of 67,049 files, the busiest 13,409 account for 94.0% of `process_file` executions. Churn is not spread thin — it is a small set of directories re-scanned relentlessly.
  <br>(windowed `row_number()` over per-path counts; top-20%-by-rank share = 94.0%).

- **`process_git_folder` is even worse: 939,212 executions over 367 paths = 2,559× per folder**, with 310 `failed` (all carrying an `error_message`) and 37 stuck in `running`. `resolve_edges`, `embed_nodes`, `resolve_libs`, `build_connections` each ran ~150,000–166,000 times as single global tasks — they re-run wholesale on every rescan.

- **`nodes.modified_at` is NOT a usable churn signal.** The June-15 bulk cohort is 451,181 nodes; subsequent weeks add only 2,436→9,305. Yet `process_file` runs ~14K/day. Re-scans that hash-match don't rewrite the node, so `modified_at` stays frozen (top files still show `last_mod = 2026-06-21` despite daily re-scan). Node churn must be measured from `activity.task_executions`, not `nodes.modified_at`.

- **Rework markers confirm churn is cross-project.** 922 `rework:` patterns exist, every one `instance_count = 1` (a one-shot marker, so the field is useless for ranking). By repo root: `sensei-hq` 399, `strategos` 211, `rokkit` 93, `dbd-rs` 72, `torii` 43, `gateway` 25 — churn/rework spans the whole workspace, not just sensei.

  ```sql
  SELECT substring(replace(name,'rework: ','') from '/Users/Jerry/(?:Developer|Work)/([^/]+)') proj,
    count(*) rework_files FROM inference.detected_patterns
  WHERE name LIKE 'rework:%' GROUP BY 1 ORDER BY 2 DESC LIMIT 8;
  ```

- **The churn started on a specific day.** `process_file` ran **4,038/day before 2026-07-12** and **34,828/day on/after** — an 8.6× step change. `scan_root` executions jumped from 9/week to **5,059/week** the week of 2026-07-13 (a 560× jump). Worst single day: 2026-07-14 with 329,748 executions over 1,903 paths (173× reprocess).

  ```sql
  SELECT CASE WHEN started_at<'2026-07-12' THEN 'before D2' ELSE 'on/after D2' END era,
    round(count(*)::numeric/count(DISTINCT date_trunc('day',started_at)),0) execs_per_day
  FROM activity.task_executions WHERE task_kind='process_file' GROUP BY 1;
  ```

| era | process_file execs | execs/day |
|-----|-------------------:|----------:|
| before D2 (< 2026-07-12) | 88,828 | 4,038 |
| on/after D2 (≥ 2026-07-12) | 835,881 | 34,828 |

---

### Root cause / interpretation

**Churn root cause — version-rescan amplifies an incomplete binary filter (pinned to 2026-07-12).** Commit `2f6f1de9` ("feat(daemon): re-scan + re-analyze on a binary-version change (D2)", 2026-07-12) added `crates/senseid/src/tasks/version_rescan.rs`. On every daemon boot where the running binary's version differs from `daemon.last_version`, it re-scans *every* indexed root and forces a full re-analysis. During active development this repo bumps versions constantly (`make bump` / `make install` on each iteration), so D2 fires a full-workspace rescan many times a day. That is exactly the step change the data shows: `scan_root` 9/week → 5,059/week, `process_file` 4,038/day → 34,828/day, both starting the week of 2026-07-13.

A full rescan alone would be tolerable if the per-file gate were tight. It is not. The scan walker (`scan_logic.rs`) respects `.gitignore` and an extension allowlist via `classifiers::file_classifier().is_binary()`, but that binary list (`crates/senseid/src/classifiers.rs`) omits the long-tail extensions that dominate the churn — `p12`, `epub`, `ckp`, `si`, `tlog`, `cfs`, `cfe`, `frm`, `psd`, `lockb`, `npz`, `aar`, `dump`. Those files pass the cheap extension filter, get a `ProcessFile` task enqueued, and are only caught inside the processor by the `is_probably_binary` content sniff — *after* a `task_executions` row is recorded and the file is opened. Result: 652,154 executions (71% of all indexing work) burned on 1,218 binary files, every one returning `items_processed = 0`. The `.ckp/.si/.tlog/.cfs` group pinned at exactly 383×/file are build/index segment directories (Xcode DerivedData, Lucene) inside repos that don't `.gitignore` them; the `.p12` certs live under Dayamed `resources/`. Note the mtime gate exists but doesn't help here: these vendored/artifact files never change, so mtime should skip them — the leak is that a `task_executions` row is still written per dispatch, so the *accounting* of churn (and likely the queue scheduling) treats each rescan as fresh work.

**Regression root cause — no reconciliation loop closes the gap.** Drift detection is insert-only (1,947 rows for 487 pairs), so `broken` items accumulate in repos where docs aren't actively maintained (`torii`, `gateway`, `alert-platform`, `dbd` all 74–100% broken) while well-tended repos (`rokkit`, `swarco`, `website`) sit near zero. There is no "mean time to regression-resolution" because nothing tracks a broken item's lifetime — `resolved_at` exists on the row but the insert-per-scan pattern means a "resolution" is just a newer row with a different status, not an update to the original. The reopened-work regressions (`senseid/src/db` corrected 3×, `api/handlers` 2×) are invisible to FTR because FTR is computed per-session: each of those three db sessions can individually be FTR-true while the *module* is objectively unstable. The crashed autonomous runs are the run-level analog — two share a plan_ref and re-attempt a failing goal, exhausting `recovery_attempts=3` without a "this goal already failed, stop re-attempting" guard.

**Why this matters architecturally.** sensei sells "first-time-right" as the headline metric, but FTR is a per-event snapshot that structurally cannot see regression (cross-time) or churn (cross-repetition). The daemon already emits every fact needed for both — `activity.task_executions` (churn), `inference.drift_items` (doc regression), `activity.sessions.outcome` + `module` (reopened work), `activity.runs.recovery_attempts` (run regression). What's missing is (a) an aggregation layer that keys on file/module/goal over time, and (b) an app surface. Today none of the app screens render churn or regression; the data dies in the tables.

---

### Recommendations

**P0 — stop the bleeding (churn):**
1. **Add the long-tail binary extensions to `classifiers.rs`** (`p12`, `epub`, `ckp`, `si`, `tlog`, `cfs`, `cfe`, `frm`, `psd`, `lockb`, `npz`, `aar`, `dump`, `pfx`, `keystore`, `jks`, `stats.json`-style generated blobs) and back it with a unit test asserting each is `is_binary`. Immediate ~71% reduction in `process_file` volume.
2. **Skip the enqueue, not just the processor.** Move the `is_binary_ext` + `is_probably_binary` check to the walk/enqueue site in `root_watcher.rs`/`scan_logic.rs` so binary files never get a `ProcessFile` task or a `task_executions` row. A dispatched-then-skipped file should not be recorded as work.
3. **Debounce version-rescan.** `maybe_rescan_on_version_change` should not force a full re-walk on every patch bump during local dev. Gate on minor/major version change, or on an explicit `SENSEI_FORCE_RESCAN`, or diff the DDL/parser version rather than the binary version. This alone reverts the 8.6× day-rate step.

**P1 — make regression & churn first-class state:**
4. **Convert `drift_items` to upsert-on-`(doc_node_id, code_node_id)`** with a real `status` transition and populated `resolved_at`, so re-breaks are countable and mean-time-to-resolution is derivable. Add a `break_count` column incremented on each current→broken flip.
5. **Add a `module_stability` rollup** keyed on `(folder_id, module)` that counts `completed` vs later `corrected` sessions — surfacing `senseid/src/db` and `api/handlers` as unstable modules. Compute in an analyzer task (mirror `aggregate_corrections`, which already runs 524×).
6. **Add a run-regression guard**: before re-attempting a `plan_ref` that previously ended `crashed`/`failed`, require an explicit override; record `re_attempt_of` on the run. Stops the two auto-buildout re-attempts from silently burning recovery budget.

**P2 — surface it:**
7. **Build a "Lifecycle / Health" screen** in the app with three panels: (a) **Churn** — top re-processed paths, binary-waste %, execs/day trend with the D2 step annotated; (b) **Regression** — broken-drift by repo, re-break leaderboard, unstable-module list; (c) **Run health** — recovery-exhausted and re-attempted goals. Source directly from the tables below.
8. **Add a daily-N gate to any FTR-drop alert** — suppress day-over-day FTR regression flags below ~10 sessions/day to kill the −80pp low-N noise.

---

### Proposed metrics & instrumentation

| Metric | Definition / formula | Source (table.column) | Cadence | Current gap |
|--------|----------------------|-----------------------|---------|-------------|
| Drift regression rate | `broken / (broken + current)` distinct pairs | `inference.drift_items.status` | per scan | Computed ad-hoc; insert-per-scan inflates denominator (1947 vs 487) |
| Re-break count | # of current→broken flips per doc pair | `inference.drift_items` (need `break_count`) | per scan | Only derivable by counting rows; no dedicated column |
| Mean-time-to-regression-resolution | `avg(resolved_at − detected_at)` for pairs that flipped back | `inference.drift_items.detected_at/resolved_at` | weekly | `resolved_at` not maintained (insert-only) |
| Module instability index | `corrected_sessions / (completed + corrected)` per module | `activity.sessions.outcome, module, folder_id` | weekly | No rollup table; must join ad-hoc |
| Reopened-work count | modules with `completed` earlier + `corrected` later | `activity.sessions.outcome, started_at` | weekly | Not computed; FTR hides it |
| Run regression rate | `crashed+failed runs / total runs`; re-attempts of prior-failed `plan_ref` | `activity.runs.status, plan_ref, recovery_attempts` | per run | No `re_attempt_of` link |
| Churn rate | `process_file execs / distinct files / week` | `activity.task_executions.task_kind='process_file'` | daily | Not surfaced anywhere |
| Inert-work ratio | `execs where items_processed=0 / total execs` | `activity.task_executions.items_processed` | daily | 92.1% today; no alert |
| Binary-waste share | `binary-ext execs / total process_file execs` | `activity.task_executions.path, task_kind` | daily | 71% today; no filter at enqueue |
| Churn concentration (Pareto) | share of execs from top-20%-busiest files | `activity.task_executions.path` (windowed) | weekly | 94% today; not tracked |
| Rescan amplification | `process_file execs/day`, annotated by `scan_root` triggers | `activity.task_executions.task_kind IN ('process_file','scan_root')` | daily | D2 step (4k→35k/day) undetected until now |
