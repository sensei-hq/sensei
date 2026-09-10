# Stage 8 — runtime progress: extend what exists, invent nothing

Whole-system spec: `docs/design/indexer-v2.md` R14 (the barrier gives the
denominator), R10.7b (folder rollup). Usable from stage 3 onward.

> **This stage is NOT about the "Stage report" section in every other spec.**
> Those are a DEVELOPMENT artifact — printed when a stage is built, so its
> result can be reviewed and redirected before the next stage starts. This
> stage is the RUNTIME progress the app consumes while a scan is running. Two
> different things with two different audiences; an earlier draft conflated
> them into one invented JSONL file.

## 1. Purpose

Make the new pipeline stages visible in the progress stream the UI already
subscribes to, so a running scan can be watched rather than waited on.

## 2. What already exists — read this before writing anything

`crates/senseid/src/tasks/progress.rs`:

    pub enum TaskEvent {              // SSE, tagged "event", snake_case
        Queued      { task_id },
        Started     { task_id, folder_path, kind, path },
        Completed   { task_id, folder_path, kind },
        Failed      { task_id, folder_path, kind, error },
        FolderQueued{ folder_path, files_total },   // the denominator
    }

    pub struct RepoProgress {
        total, pending, running, completed, failed,
        current_file: Option<String>,
    }

Plus `crates/senseid/src/tasks/progress_emitter.rs`.

**Both grains are already here.** File-level is `Started { path }` and
`RepoProgress::current_file`; repo-level is `RepoProgress` and
`FolderQueued { files_total }`. There is no missing mechanism, no missing
shape, and nothing to design. **An earlier draft of this spec proposed a new
append-only `~/.sensei/scan-progress.jsonl`. That is withdrawn** — it was a
second progress mechanism over a working one, and two mechanisms mean the UI
and the daemon can disagree about what a scan is doing.

## 3. Requirements

- **S1.** Extend `TaskEvent` for the stages that have no representation yet —
  `scan_root`, `scan_repo`, `library_discovery`, and the structure barrier.
  Add variants to the EXISTING enum; do not add a parallel channel.
- **S2** (R14). `FolderQueued { files_total }` is the denominator and it is
  already emitted at the right moment. Feed it from
  `folders.props.expected_files` as written at the structure barrier — never
  from a recount. A progress bar whose denominator is recomputed can disagree
  with the work actually queued.
- **S3** (R10.7b). Repo- and folder-level rollup reads
  `sensei.folder_completeness`, extended for unparseable counts. Not a second
  rollup, and not a status column a trigger maintains.
- **S4.** A stage that FAILS emits `Failed { error }` with the reason, not
  silence. A scan that stops emitting is indistinguishable from a scan that
  finished.
- **S5.** **No silent caps.** If a stage bounds coverage — top-N, sampling,
  no-retry — say so in the event. Silent truncation reads as "covered
  everything" when it did not.

## 4. Failure modes

| input | this stage does |
|---|---|
| no SSE subscriber | emit anyway. The emitter already handles this; a broadcast with no receiver is not an error. |
| a stage emits no event | that is a defect in THAT stage, visible as a gap in a known sequence. |
| the denominator is 0 | report 0/0. Never substitute 100%, which reads as complete. |
| a stage panics | `Failed` must still be emitted, or the UI shows a task running forever. |
| an event channel is full | drop and count the drops. Never block the scan on a UI channel — but never drop silently either. |

## 5. Stage report — what you SHOW when the stage is done

Not a file. Run a scan with an SSE client attached and paste the event
sequence, so the shape can be checked by eye:

    event=folder_queued  folder=…/sensei  files_total=48665
    event=started        kind=scan_repo   path=…/sensei
    event=started        kind=index_file  path=crates/…/fqn.rs
    event=completed      kind=index_file
    …
    RepoProgress { total: 48665, pending: 48200, running: 8,
                   completed: 457, failed: 0,
                   current_file: Some("crates/…/resolve.rs") }

The pass condition is that every new stage appears, `files_total` equals
`expected_files`, and `total = pending + running + completed + failed` holds
at every sample.

## 6. Verification

| test | mutation that must break it |
|---|---|
| every new stage emits at least one `Started` and one terminal event | remove a stage's emit |
| `FolderQueued.files_total` equals `expected_files` at the barrier | recompute it with a second query |
| `total = pending + running + completed + failed` at every observation | drop a decrement |
| a panicking stage still emits `Failed` | remove the catch |
| a subscriber attached mid-scan sees events incrementally | buffer and flush at the end |
| a stage that caps coverage says so | apply a top-N silently |
| `TaskEvent` gained variants and no second progress channel exists | add a JSONL writer — the test greps for one |

The incremental test is the one that catches buffering, which turns a live
feed into a post-mortem and is invisible to any test that reads after the run.

## 7. Watch out

**The mechanism exists. Extend it.** This is the sixth thing in this design
that was specified as new work and turned out to be built and wired — after
the file entity, the library level, manifest extraction, `.gitignore`
handling, and `node_kind`'s `field`/`enum_variant` values. Before writing any
"emit progress" code, read `tasks/progress.rs` and `tasks/progress_emitter.rs`
and say what is actually missing.

**A sample is not a debug aid to be removed later.** "The loop ran 48,646
times" and "the loop ran over files" are different claims. Several defects in
this design's history were counts that were correct over the wrong population.

## 8. Definition of done

- New stages appear in `TaskEvent`; no second progress channel exists,
  verified by test.
- The denominator comes from the barrier.
- Failures emit `Failed` with a reason; a panicking stage does not hang the UI.
- A live SSE capture of a real scan is pasted into the stage report.
