# Task worker system — analysis and consolidation plan

Status: **analysis only**, no code changed. Evidence is from the live daemon DB
(`localhost:5432/sensei`, 2026-08-24) and the current tree on `develop`.

---

## 1. What exists today

**35 task kinds**, dispatched by a 35-arm `match` in `executor.rs`, all handlers
sharing one shape:

```rust
async fn(ctx: &TaskContext, task: &Task) -> Result<u32, String>
```

That uniformity is the good news — the shape is already trait-compatible. Every
other structural property is ad hoc.

The kinds fall into **five de-facto pipelines** that nothing in the code names:

| Pipeline | Stages (in order) | Kinds |
|---|---|---|
| **Index** | discover → walk → parse → link → embed → cluster | ScanRoot, ProcessGitFolder, ProcessFolder, ProcessFile, DeleteFile, DeleteFolder, BranchSwitch, ExtractDeps, BuildConnections, EmbedNodes, DetectCommunities |
| **Library** | resolve → fetch → index | ResolveLibs, ImportLib, IndexLibrary, IndexLibraryPage |
| **Activity** | ingest → synthesize → enrich | BackfillTranscripts, BackfillTranscriptFile, AnalyzeProject, AnalyzeSessionProcess, ReconcileIdentity |
| **Metrics** | plan → compute → seal | ComputeProjectMetrics, ComputeGroupMetrics, ComputeHealth, BackfillCoverage |
| **Inference** | aggregate → classify → consolidate → publish | MeasureVerdicts, AggregateCorrections, AggregateToolInsights, ClassifyPendingVerdicts, ConsolidateGovernance, WarmInsightCopy, LearnPlaybooks, ScanDocDrift, PublishRelaySegments, AdvanceRun, PublishRun |

The pipelines exist only as convention. There is no type, trait, module
boundary, or grouping that says so — which is why a new kind gets bolted onto
the `match` and a 4-place ripple (enum, Display, watchdog bucket, test list).

---

## 2. Findings

### F1 — The task payload is stringly-typed, and every handler re-parses it differently

`Task` carries **four** general-purpose input slots plus three special-cases:

```
folder_path: String     path: String        as_of: Option<NaiveDate>
module_id: Option<String>   branch: Option<String>   url: Option<String>
```

There is no schema per kind. Each handler opens by re-deriving its inputs from
whichever slot the enqueue site happened to use:

- `analyze_project` — `task.path` is a project **uuid**
- `embed_nodes` — `task.folder_path` is a folder **abs path**
- `process_file` — `task.path` is a file abs path
- `BackfillTranscriptFile` — `folder_path` is a **capture source name**, `path` is a file
- `BackfillCoverage` — `folder_path` is a **stringified week count** (mine, last week — the clearest symptom)

The same field means five different things. Nothing checks it; a mismatch is a
runtime parse error at best and silent wrong-target work at worst. `module_id`,
`branch`, `url` exist because three kinds each needed one more field and adding a
column to the shared struct was easier than modelling a payload.

**This is the root cause of most of the rest.** Backfill can't be "just a
parameter" when there is nowhere typed to put the parameter.

### F2 — Backfill is a parallel code path instead of a parameter

Confirmed twice in the last three commits. The pattern repeats per pipeline:

| Pipeline | Live path | Backfill path | Shared? |
|---|---|---|---|
| Activity | hook events → `assistant_events` → synthesize | `BackfillTranscripts` → `BackfillTranscriptFile` | **No** — different sources, different tables |
| Metrics | `ComputeProjectMetrics` (today) | same kind with `as_of: Some(day)` | **Yes** — the one place it's already right |
| Coverage | snapshot of current checkout | `BackfillCoverage` (separate kind) | **No** |

Metrics already proves the design works: one kind, `as_of: None` = today,
`as_of: Some(d)` = that day. Coverage and transcripts each grew a second kind
instead. The user's framing — *"backfilling should be achieved by reusing the
regular processing pipeline"* — is exactly the metrics model generalised.

### F3 — Two turn tables, two ingestion pipelines, one concept

| | `activity.turns` | `activity.transcript_turns` |
|---|---|---|
| Derived from | hook event stream | transcript files |
| Rows | 4,529 | 4,239 |
| `session_id` | `uuid` FK → `sessions.id` | **`text`** (client session id, no FK) |
| Columns | 11 | 24 |
| Carries | timing, tool_calls, is_correction, segment, triage_signal | + prose, tokens, cache, model, provider, branch, skill, effort, attrs |

Overlap measured:

```
BOTH representations           297 sessions   (95%)
only hook turns                 16 sessions
only transcript turns            0 sessions
turn counts AGREE              228 / DISAGREE 69
```

**Answering the merge question directly:** yes, they should merge, but they are
not duplicates — they are two *definitions* of "turn". A hook turn spans
UserPromptSubmit→UserPromptSubmit (hence `segment`, `is_correction`); a
transcript turn is one exchange. That's why 69 sessions disagree on the count.
`transcript_turns` is a strict information superset except for four columns
(`segment`, `is_correction`, `triage_signal`, `tool_calls`) which are *derived*,
not captured — they can be computed onto transcript rows.

On the history hypothesis: both sources currently reach back to **2025-05-07**,
so there is no date gap. The real gap is **67 orphan transcript sessions** (534
rows, earliest 2025-05-11) that have no `sessions` row because their repos aren't
tracked — those are transcript-only history, but they present as orphans rather
than as an earlier date range.

Merge target: keep `transcript_turns`' richer shape, add the four derived
columns, fix `session_id` to a real uuid FK, retire `activity.turns`.

### F4 — The execution log is unbounded (the biggest operational finding)

```
activity.task_executions   4,797,442 rows   1,568 MB   69 days   ≈70k rows/day
```

Second-largest table in the database after `nodes` (2,103 MB), and **nothing
prunes it**. `activity_pruner.rs` handles sessions/turns/events and does not
mention it. At current rate it adds ~23 MB/day indefinitely.

Distribution shows why: 64% is index-pipeline chatter.

```
process_git_folder  1,383,118  28.8%
process_file        1,281,818  26.7%
process_folder        405,172   8.4%
embed_nodes           240,520   5.0%
```

Per-file index tasks do not need per-row permanent history; a rollup does.

### F5 — `task_kind` is free text, and renames have already orphaned history

`task_executions.task_kind` is `text` with no constraint. Three values in the log
match no current kind:

```
compute_metrics · plan_metric_days · resolve_edges
```

Past renames silently detached their history. (The existing
`sensei.task_type_kind` enum is unrelated — it's commit types: feat/fix/docs/…)

### F6 — No trait, no grouping, so every new kind is a 4-place ripple

Adding `BackfillCoverage` last week required edits in: the enum, `Display`, the
watchdog bucket `match`, and a test enumerating every kind. Nothing enforces that
list — a missed watchdog arm is a compile error only because the match is
exhaustive; a missed *test* entry is silent.

### F7 — Seven schedulers, each hand-rolling the same loop

`advance_run`, `analyzer`, `contribute`, `library_update`, `metrics`,
`reconcile`, `watchdog` — each a bespoke `loop { sleep; enqueue }` with its own
interval, its own overlap guard (or none), and its own logging.

### F8 — Two incompatible resumability models

| `sensei.metric_watermarks` | `activity.transcript_cursor` |
|---|---|
| `(repository_id, metric_group)` | `(source, file_path)` |
| `sealed_through date`, `last_sha` | `last_mtime_ns`, `turns_ingested` |

Same idea — "how far have I got" — modelled twice with no shared vocabulary.
Coverage/index have no watermark at all.

### F9 — Names that mislead

Beyond the two fixed last commit:

- `activity.turns` vs `activity.transcript_turns` — neither name says *hook-derived* vs *file-derived*
- `activity.task_sessions` — unrelated to `activity.sessions`; it's run/agent sessions
- `BackfillTranscripts` / `BackfillTranscriptFile` — "backfill" is the *mode*, not the work; the work is ingestion
- `transcript_cursor` vs `metric_watermarks` — same concept, two words
- `ProcessFolder` vs `ProcessGitFolder` vs `ScanRoot` — the distinction is real but unguessable

### F10 — Deleted folder: branch switch vs real delete is not discriminated

`root_watcher.rs:546` enqueues `DeleteFolder` on any directory removal.
`BranchSwitch` exists as a kind but nothing in the watcher decides between them —
it is enqueued elsewhere. A branch switch that removes a directory therefore
looks like a deletion, and the index drops nodes it will immediately re-scan.
The signal to discriminate (does the path still exist in another ref? did
`HEAD` change?) is available and unused.

---

## 3. Proposed architecture

### 3.1 Typed payload, one enqueue vocabulary

Replace the four ad-hoc slots with a per-kind payload enum, serialized into a
single `payload jsonb` column and parsed **once**:

```rust
pub enum TaskPayload {
    Repo    { repository_id: Uuid },
    RepoDay { repository_id: Uuid, day: NaiveDate },
    RepoRange { repository_id: Uuid, from: NaiveDate, to: Option<NaiveDate> },
    File    { repository_id: Uuid, path: PathBuf },
    Capture { acp: CaptureSource, unit: String },
    Metric  { repository_id: Uuid, metric: String, day: NaiveDate },
    …
}
```

`folder_path`/`path`/`module_id`/`branch`/`url` collapse into this. Handlers stop
re-parsing; a wrong payload is a deserialization error at enqueue time, not a
mystery at run time.

### 3.2 A `Processor` trait per pipeline stage

```rust
trait Processor {
    type Input:  DeserializeOwned;   // its slice of TaskPayload
    type Output: Persistable;        // standardized, not per-handler SQL

    const KIND:     TaskKind;
    const PIPELINE: Pipeline;        // Index | Library | Activity | Metrics | Inference
    const STAGE:    Stage;           // Discover | Ingest | Derive | Aggregate | Publish
    const BUDGET:   Duration;        // replaces the watchdog match arm

    async fn run(&self, ctx: &TaskContext, input: Self::Input) -> Result<Self::Output, String>;
}
```

The watchdog bucket, the pipeline grouping, and the test enumeration all become
properties **on the impl** instead of four lists to keep in sync. Registration
via `inventory`/a registry macro removes the 35-arm match.

### 3.3 Coordinator → worker, with backfill as a parameter

The user's model, applied uniformly. Each pipeline gets one **coordinator** that
decides *what needs doing* and one **worker** that does one unit:

| Coordinator (scheduled or on-demand) | Worker (one unit, idempotent) |
|---|---|
| `IndexRepo { repository_id }` | `IndexFile { repository_id, path }` |
| `IngestCaptures { acp, since }` | `IngestTranscript { acp, unit }` |
| `PlanMetrics { repository_id, from }` | `ComputeMetric { repository_id, metric, day }` |

Backfill stops being a kind. It is the **same coordinator with an earlier
`since`/`from`**. "Backfill coverage for repo X from 2025-01-01" is
`PlanMetrics { repository_id: X, from: 2025-01-01 }` — no `BackfillCoverage`,
no `BackfillTranscripts`, no second code path to drift.

Standardized output: every ingest worker emits the same
`{ sessions[], turns[], events[] }` shape and hands it to **one** persistence
function, rather than each adapter writing its own SQL. That is what makes the
hook path and the transcript path converge (F3) instead of maintaining two.

### 3.4 Watermark as a single concept

One table keyed by `(pipeline, scope_id, unit)` carrying `sealed_through` plus a
source-specific `cursor jsonb` (mtime for files, sha for git, day for metrics).
`metric_watermarks` and `transcript_cursor` both become rows in it.

Worker updates the watermark **at the end of its own unit**. Combined with
single-flight per `(kind, payload)` — which the queue can enforce once payloads
are typed — the cross-cutting watermark race the user flagged is structurally
prevented rather than reasoned about.

### 3.5 Event bus / tracking

`TaskEvent` already exists and now has a per-task follow API. What's missing:

- **Stage-level progress** — a long worker emits `Progress { done, total }`, not just start/end
- **Correlation** — `trace_id` on the payload so a coordinator and its N workers are one queryable unit (today only `parent_task_id`, which the transcript dispatcher didn't even set until last commit)
- **Archival** — see below

### 3.6 Log archival

Split the concern:

- `task_executions` keeps **coordinator + failed** rows indefinitely (low volume, high value)
- Per-file worker rows roll up nightly into `task_execution_daily` (`kind, day, count, p50/p95 duration, failures`) and the raw rows are pruned past N days
- Add `task_kind` as a **checked enum** so a rename is a compile+migration event, not silent orphaning

On today's data that turns 4.8M rows / 1.5 GB into roughly 70k rows plus a small
daily rollup.

---

## 4. Tables involved

**Directly restructured**

| Table | Rows | Disk | Action |
|---|---:|---:|---|
| `activity.task_executions` | 4,797,442 | 1,568 MB | prune + roll up; enum `task_kind`; add `trace_id`, `payload` |
| `activity.turns` | 4,529 | — | **merge into** `transcript_turns`, then retire |
| `activity.transcript_turns` | 4,239 | 126 MB | absorb derived columns; `session_id` text → uuid FK |
| `activity.transcript_cursor` | — | — | fold into unified watermark |
| `sensei.metric_watermarks` | — | — | fold into unified watermark |

**Read/written by the pipelines (unchanged shape)**

`activity.sessions` (334) · `activity.assistant_events` (298,353 / 1,229 MB) ·
`activity.session_process_evidence` · `activity.snapshots` · `activity.runs` ·
`activity.run_events` · `activity.memory_loads` · `activity.task_sessions` ·
`sensei.project_metrics` · `sensei.metrics` · `sensei.repositories` ·
`sensei.folders` · `sensei.nodes` (2,103 MB) · `sensei.edges` (256 MB) ·
`sensei.symbol_names` · `sensei.scan_state` · `sensei.communities`

**Proposed renames** (cleanup cycle)

| From | To | Why |
|---|---|---|
| `activity.turns` | *(retired)* | merged |
| `activity.transcript_turns` | `activity.session_turns` | it is *the* turn table once merged; source is a column |
| `activity.transcript_cursor` + `sensei.metric_watermarks` | `sensei.pipeline_watermarks` | one concept, one name |
| `activity.task_sessions` | `activity.agent_sessions` | disambiguate from `activity.sessions` |
| `TaskKind::Backfill*` | `Ingest*` | backfill is a parameter, not a kind |

---

## 5. Sequencing

Forward-only; no step depends on a later one.

1. **Archival + enum on `task_executions`** — standalone, immediate 1.5 GB win, no other subsystem touched.
2. **Typed `TaskPayload`** — mechanical, per-kind, compiler-guided. Unblocks 3–5.
3. **`Processor` trait + registry** — collapses the match and the 4-place ripple.
4. **Unified watermark table** — needs 2 for `RepoRange`.
5. **Coordinator/worker split per pipeline**, metrics first (already closest), then activity, then index. Retires the `Backfill*` kinds.
6. **Turn merge** — needs 5's standardized persistence so both sources write one shape.
7. **Branch-switch discrimination** in the watcher — independent, can land any time.

Steps 1 and 7 are independently shippable and worth doing first regardless of
whether the rest proceeds.

---

## 6. Open questions

1. **Turn definition** — after the merge, is a "turn" the hook definition (prompt-to-prompt) or the transcript definition (one exchange)? 69 sessions disagree today. Metrics that count turns will shift; that shift needs to be deliberate and dated.
2. **Retention window** for raw worker execution rows — 7 days? 30?
3. **`assistant_events` (1.2 GB)** — is the raw event stream still needed once turns are merged, or does it become derivable/prunable?
