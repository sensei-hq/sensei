# Analysis — Session-history recovery (folder renames orphan Claude transcripts)

**Status:** in progress · **Owner task:** #23 · **Last updated:** 2026-08-11

This is a self-contained checkpoint. It records the problem, the evidence, what
has already been done to the live DB, the existing machinery, the remaining
gaps, and the concrete course of action. Resume from "Course of action".

---

## Problem

Graphs for `dbd` (and others) show only ~2 weeks of history even though the
projects have months of work. Two independent causes:

1. **Session capture is young.** sensei's session recording began globally on
   **2026-07-13** — there is *no* session history before that for any project.
   The "months of history" a user remembers is git/code history, not sensei
   session observation.
2. **Folder renames/moves orphan the pre-rename Claude transcripts.** Claude
   Code stores transcripts under a **path-encoded directory**
   (`~/.claude/projects/-Users-Jerry-Developer-dbd-rs/<session>.jsonl`). sensei
   ingests the **current** path only. When a folder is renamed
   (`dbd-rs → dbd`, `strategos/monorepo → torii`, `strategos/gateway → gateway`),
   the old transcript directory is left behind and never attributed — months of
   real history sit on disk, un-ingested.

## Evidence (live DB + disk, 2026-08-11)

Raw data spans (dbd): `activity.sessions` 23 rows **7/30→8/11** (only
`acp_id='claude'`, **no Zed**); `activity.assistant_events` 13,539 rows
**7/04→8/11**; metric snapshots 7/28→8/10 (staggered per metric).

Orphaned transcript directories (on disk, un-ingested):

| dir under `~/.claude/projects/` | transcripts | span | belongs to | real cwds inside |
|---|---|---|---|---|
| `-…-dbd-rs` | 11–16 | May 11 → Jul 28 | **dbd** | `/…/dbd-rs`, `/…/dbd-rs/site` |
| `-…-strategos` | 7 | Jul 17 → Jul 29 | **torii** / gateway | `/…/strategos/monorepo`(→torii), `/…/strategos/gateway`(→gateway), `/…/strategos` |
| `-…-strategos-gateway` | 5 | Jul 13 → Jul 23 | **gateway** | `/…/strategos/gateway` |

`119` projects have 0 sessions — the tell-tale of renames/merges creating a new
project row while sessions stay attributed to the old one. The lineage the user
confirmed: `dbd-rs→dbd`, `strategos*`/monorepo→`torii`, `strategos-gateway→gateway`
(moved out), `sensei-hq*→sensei`, `rokkit*→rokkit`.

Current root paths (all exist on disk): dbd `/Users/Jerry/Developer/dbd` (git),
torii `/Users/Jerry/Developer/torii`, gateway `/Users/Jerry/Developer/gateway`.

## The machinery already exists (do NOT rebuild)

A full Claude transcript importer + rename-history is already shipped (#73/#75):

- **Importer** — `crates/senseid/src/transcript/{mod.rs,claude.rs,zed.rs}`. Reads
  `~/.claude/projects/<dir>/<session_id>.jsonl`; `ingest_one` (`mod.rs:144`),
  `synthesize_session` (`mod.rs:200`; project resolution at `mod.rs:222–240`),
  `run_backfill` (`mod.rs:314`) → `run_backfill_file` (`mod.rs:349`). Sets
  historical timestamps + `backfilled=true` via `set_session_history`
  (`pg_store.rs:6770`). Idempotent: prose via `transcript_cursor` (mtime),
  synthesis gated by `session_has_events()`.
- **Endpoint / tasks** — `POST /api/transcripts/backfill`
  (`api/handlers/observatory.rs:330`, route `routes.rs:186`); TaskKinds
  `BackfillTranscripts` / `BackfillTranscriptFile` (`tasks/mod.rs:73,76`),
  dispatch `tasks/executor.rs:135`.
- **Rename history** — `sensei.folder_path_aliases`
  (`database/ddl/table/sensei/folder_path_aliases.ddl`): `alias_abs_path` → current
  `folder_id`, `reason ∈ {rename,detected,manual}`. Written by `remap_folder`
  (`pg_store.rs:2626`) and auto-detected via git-remote match in `reconcile_roots`
  (`tasks/handlers/scan.rs:231`, `find_live_root_by_remote` `pg_store.rs:~2591`).
- **Alias-aware attribution** — `get_folder_ids_by_path` (`pg_store.rs:6409`,
  exact), `find_folder_for_path` (`pg_store.rs:5238`, **nearest-ancestor prefix**,
  alias-aware — this is the subfolder→root rollup), `repo_root_for_path`
  (`pg_store.rs:5267`, strict git-root), `resolve_folder_by_path`
  (`pg_store.rs:6586`, metrics wrapper, honest `None` on miss).
- **CLI** — `sensei folder remap <old> <new>` (`crates/cli/src/main.rs:178`,
  handler `folder_remap` `:1175` → `POST /api/folders/remap`
  `api/handlers/workspace.rs:124`). Writes the alias (or moves the husk) + calls
  `repair_orphaned_sessions`.
- **Live record path** — `ingest_hook_event` (`api/handlers/sessions.rs:257`,
  route `/hook/event` `routes.rs:276`) → `find_folder_for_path(cwd)` →
  `record_session_event` (`pg_store.rs:5284`, idempotent per `client_session_id`).

## Already applied to the live DB (2026-08-11)

Aliases created via `sensei folder remap` (all `reason='manual'`):

- `/Users/Jerry/Developer/dbd-rs` → `/Users/Jerry/Developer/dbd` (3 sessions re-attached)
- `/Users/Jerry/Developer/strategos/monorepo` → `/Users/Jerry/Developer/torii`
- `/Users/Jerry/Developer/strategos/gateway` → `/Users/Jerry/Developer/gateway`

`POST /api/transcripts/backfill` triggered → **303 files enqueued**. Observed:
tasks drain slowly (queued behind remap-triggered re-scans); a sampled orphaned
`dbd-rs` session shows `events=0, session→<none>` — i.e. genuinely un-ingested and
ready to synthesize + attribute via the new aliases once the queue reaches it.
`backfilled` session count was still 0 at checkpoint time (queue not yet drained).

## Backfill drain — VERIFIED against live data (2026-08-11)

The queue has since drained. Per-project spans now:

| project | sessions | backfilled | earliest | latest |
|---|---|---|---|---|
| **dbd** | 38 | **15** | **2025-06-05** | 2026-08-11 |
| torii | 12 | 0 | 2026-07-23 | 2026-08-05 |
| gateway | 11 | 0 | 2026-07-30 | 2026-08-09 |

- **dbd — recovered.** 15 backfilled sessions; history reaches **June 2025**. The
  `dbd-rs→dbd` alias + synthesis machinery works end-to-end.
- **torii / gateway — "0 backfilled" is mostly a FALSE ALARM.** Their July sessions
  *are* attributed (torii 4 @ 7/23–27; gateway 5 @ 7/31), just live-captured
  (post-7/13 capture start), so `backfilled=false` is correct. The strategos
  transcripts span 7/13–29 — all after live capture — so their sessions already had
  events, `needs_synth` was false (`transcript/mod.rs:163`), and synthesis correctly
  skipped them. All 21,117 strategos-cwd events map to a session row → nothing for
  `repair_orphaned_sessions` to do either.
- **Real residual gap — 4 sessions stranded on the defunct `strategos` project.**
  Their resolving cwd is the **bare** `/Users/Jerry/Developer/strategos`, which has
  **no alias** (only `/strategos/monorepo`→torii and `/strategos/gateway`→gateway
  are aliased) and still resolves to the old `strategos` project (**161 live folder
  rows**). The stranded sessions:
  - `zed-45a8…` 2026-03-20 (backfilled, Zed) — bare `/…/strategos`
  - `1e0f9d7e…` 2026-06-17 (backfilled) — bare `/…/strategos`
  - `83b57e52…` 2026-07-17 (live) — bare `/…/strategos` + gateway subpaths
  - `e57060ae…` 2026-07-23 (live) — bare `/…/strategos`

  The two backfilled ones (Mar 20, Jun 17) are exactly the deep history that would
  extend **torii** back to March.

  **RESOLVED 2026-08-11 — decision: "remap root → torii (minimal)".** The
  `sensei folder remap` CLI was the WRONG tool: `remap_folder` (pg_store.rs:2626)
  calls `delete_folder_tree`, which `DELETE`s the old folder with parent_id CASCADE
  — remapping the bare `/…/strategos` root would have cascade-deleted strategos's
  other ~160 folders (the "retire" outcome, not "minimal"), and it never updates
  `sessions.project_id` anyway. Also note the metrics scope by **`folders.project_id`
  via `sessions.folder_id`** (session_outcomes.rs:76), not `sessions.project_id`, and
  `record_session_event` sets `project_id` **sticky** (`COALESCE(existing, new)`).
  So the minimal, correct fix was a targeted, transactional re-point of just the 4
  sessions (`folder_id → ea644f90` torii-root, `project_id → 945702a2` torii),
  leaving the strategos folder/project and the on-disk dir intact. Verified:
  torii's metrics span moved **7/23→8/05 (12) ⇒ 3/20→8/05 (16)**; strategos residual
  via the metrics join = 0. NOTE: session `83b57e52` (7/17) is actually gateway work
  (cwds are gateway subpaths) but was lumped to torii per the minimal choice —
  reassign its `folder_id`/`project_id` to gateway if that precision matters.
  CAVEAT: raw session span now reaches March, but the *charts* still won't render
  those months until §5 (historical snapshots + window-anchor) lands — the
  session-anchored 14-day window still excludes these historical rows.

## Remaining gaps → Course of action

Ordered. Each is independently resumable.

1. **Verify the backfill drained.** ✅ DONE 2026-08-11 — see "Backfill drain —
   VERIFIED" above. dbd recovered to June 2025; torii/gateway July sessions are
   correctly attributed (live, not backfilled). Residual: 4 sessions stranded on the
   defunct `strategos` project → blocked on the identity decision, then a
   `folder remap` of the bare `/…/strategos` root.

2. **Auto-schedule the backfill (missing seam).** Today it only runs on the
   manual endpoint and competes with other tasks. Add a scheduler that enqueues
   `BackfillTranscripts` on a tick — model on `tasks/reconcile_scheduler.rs`,
   boot-spawn next to the others in `api/server.rs` (analyzer :284, metrics :293,
   reconcile :325). Guard with `queue.has_pending_kind`. Consider a priority bump
   so backfill isn't starved by re-scan work.

3. **Root-project rollup precision.** `synthesize_session` attributes to the
   nearest *folder row* via `find_folder_for_path`, which may be a subfolder, not
   the repo root. If strict root-project rollup is required, switch its fallback
   to `repo_root_for_path` (`pg_store.rs:5267`). Small, testable change at
   `transcript/mod.rs:229–236`.

4. **Rename/move-aware watcher (user ask: "simplify the backfill").** Auto-detect
   today only fires on a git-remote match at reconcile; remote-less renames need
   the manual `folder remap`. Course: have the watcher mark old folder
   names/extra folders into `folder_path_aliases` on rename/move (FSEvents rename
   events, or a reconcile pass that records vanished→appeared roots), so backfill
   reads the map instead of a hand-run remap.

5. **Historical metric snapshots — the part that makes graphs span months.**
   THIS is what actually delivers "history going back months"; the backfill alone
   does not. Two anchors disagree today (`metrics.window_days` default 14,
   `metrics_scheduler.rs:49`):
   - **Session-anchored** metrics filter `sessions.started_at ≥ now()−14d`
     (`session_outcomes.rs:78/107/133/170`, `churn.rs:101`). Backfilled sessions
     carry their *historical* `started_at` → fall outside the window → do **not**
     appear in current daily FTR/quality/churn snapshots.
   - **Event-anchored** metrics filter `assistant_events.created_at ≥ now()−14d`
     (`tool.rs:131`, `autonomy.rs:103`). Synthesized events get `created_at=now()`
     → they **do** count immediately (an asymmetry to reconcile).
   Course: compute metric snapshots for **past `computed_on` dates** (backfill the
   daily series, not just forward), and align the session-vs-event window anchor
   (prefer session `started_at`/event call-time consistently). Only then do the
   charts render real months.

   ### §5 AGREED ARCHITECTURE (2026-08-11) — unified backfill + incremental

   Verified mechanism (live data): the daily computers already key each row by the
   session's **own day** (`computed_on = date_trunc('day', started_at)::date`) and
   upsert idempotently per `(project, metric, day, grain)`. dbd's `ftr` daily rows
   start exactly 14 days back while 15 dbd sessions (2025-06 → 2026-06) produced
   zero daily rows — so the ONLY thing hiding history is the
   `started_at >= now() - 14d` emit filter. There is **no need for an "as-of"
   recompute of point-in-time state per past day** for the day-keyed series.

   **Design (unify backfill + incremental as ONE mechanism):** every metric value is
   computed for a specific `computed_on` DAY and upserted per `(project, metric,
   day, grain)`. Backfill and incremental are the same operation over different
   day-sets. Three processors:
   1. **Transcript importer** (EXISTS — `BackfillTranscripts` → `BackfillTranscriptFile`):
      synthesizes sessions/events with historical timestamps. First install = full;
      forward = new transcripts.
   2. **Timeline→day-task planner** (NEW): per project, derive the data-day span from
      the feed (`min..max` of session `started_at` / event `ts`), diff against the
      `computed_on` days already in `project_metrics`, and enqueue one per-day compute
      task per **(project, metric-group, missing/stale day)**. First install → all
      days (backfill); daily → just the new day (incremental); gaps self-heal.
   3. **Per-day computer** (MODIFY the existing computers): a task computes its target
      day D and upserts `computed_on=D`. Per-day granularity so one bad day can't
      block the rest (same rationale as `BackfillTranscriptFile`).

   **Honesty constraints (firm):**
   - Anchor on true occurrence time — session `started_at` / event client `ts` —
     NEVER `created_at` (insert-time = now for synthesized rows), else all backfilled
     events collapse onto "today". (Event-anchored computers keying `created_at`
     must switch to `ts`.)
   - **Code-graph-state metrics** (`duplication_ratio`, and any "current state"
     snapshot that can't be reconstructed as-of a past day — no historical code
     graph exists) get a **today-only** task; the planner does NOT fabricate
     historical days for them. Session/event-derived state (e.g. `rework_density`
     over a trailing window) CAN backfill.
   - Idempotent upsert per `(project, metric, day, grain)`; a day's compute failure
     propagates `Err` (task retries), never a fabricated value; honest-empty day →
     no row.

   Per-metric classification (day-keyed backfillable vs forward-only point-in-time)
   is being finalized against the full computer map before implementation.

## Done-when

- Backfilled sessions attributed to dbd/torii/gateway spanning May–Aug.
- Backfill runs automatically (scheduler) and follows future renames/moves.
- Metric daily snapshots exist for historical dates; the metrics charts show
  months, not ~2 weeks.

Related: [metrics-review.md](./metrics-review.md).
