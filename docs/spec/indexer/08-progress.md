# Stage 8 — progress: a checkpoint per layer, with a sample

Whole-system spec: `docs/design/indexer-v2.md` R14 (the barrier gives the
denominator), R10.7b (folder rollup). Usable from stage 3 onward; it does not
need 4–7 to exist.

## 1. Purpose

Emit progress the UI already consumes, at every layer of the pipeline, with a
SAMPLE at each so a reader can eyeball the shape rather than trust a count.

Serves G2 directly: "how far along is this and is anything wrong" should be
answerable during a scan, not after it.

## 2. Inputs and outputs

    checkpoint(stage: Stage, payload: CheckpointPayload) -> ()   IO (append-only)

One JSON line appended to `~/.sensei/scan-progress.jsonl`. Append-only and
tailable, so a crash leaves everything written up to that point.

The progress SHAPE the UI consumes has two grains and both are required:
**file-level** and **repo-level**.

## 3. Requirements

- **S1.** A checkpoint per layer: `scan_root`, `scan_repo`, `structure_write`,
  `library_discovery`, `file_index`, `reconcile`. Each stage's spec fixes its
  own payload; this stage owns the mechanism and the file.
- **S2.** Every checkpoint carries a SAMPLE — one rendered instance of the
  thing being counted. A count of 48,646 says the loop ran; one rendered file
  row says it ran over the right population.
- **S3** (R14). The denominator comes from the structure barrier
  (`folders.props.expected_files`), never from a second count. A progress bar
  whose denominator is recomputed can disagree with the work actually queued.
- **S4** (R10.7b). Repo-level and folder-level rollup read
  `sensei.folder_completeness`, the recursive view that already exists,
  extended for unparseable counts. Do not add a second rollup.
- **S5.** Counts that must reconcile are ASSERTED, not merely reported —
  `grouped + ungrouped = packages_seen`, `nodes_after = nodes_before -
  deleted`, `reasons.sum() = unresolved`. A number that quietly fails to add
  up is worse than a missing one.
- **S6.** **No silent caps.** If any stage bounds coverage — top-N, sampling,
  no-retry — the checkpoint says what was dropped. Silent truncation reads as
  "covered everything" when it did not.

## 4. Failure modes

| input | this stage does |
|---|---|
| the progress file is unwritable | log once and continue. Progress reporting must never fail the scan — but it must not fail SILENTLY either, so the failure appears in the daemon log. |
| a stage produces no checkpoint | that is a defect in that stage, and it is visible as a missing line in a known sequence. |
| the denominator is 0 | report 0/0 honestly. Do not substitute 100%. |
| a count assertion fails (S5) | report the discrepancy WITH both numbers. Do not silently pick one. |
| the file grows unbounded | rotate by scan run. Each run is a bounded sequence with a known start. |

## 5. Checkpoint output

This stage's own line records that the mechanism ran:

    {"stage":"08-progress","at":"<iso8601>","run_id":"…",
     "stages_expected":["01-scan-root","02-scan-repo","02b-library-discovery",
                        "03-structure-write","04..07-file-index"],
     "stages_seen":[…],"missing":[],
     "assertions_checked":<n>,"assertions_failed":0,
     "caps_applied":[]}

`missing` and `caps_applied` both being empty is the pass condition, and both
are the kind of thing that is invisible unless named.

## 6. Verification

| test | mutation that must break it |
|---|---|
| every stage in the expected list emits exactly one line per run | remove any stage's `checkpoint` call |
| every line parses as JSON and carries `stage`, `at` and a sample | emit a line without a sample |
| the denominator equals `expected_files` at the barrier | recompute it with a second query |
| a failing count assertion is REPORTED with both numbers | let it pass |
| an unwritable progress file does not fail the scan, and IS logged | swallow the error |
| tailing the file during a scan yields lines incrementally | buffer and flush at the end |
| a stage that caps coverage records what it dropped | apply a top-N silently |

The incremental-tail test is the one that catches buffering, which turns a
live progress feed into a post-mortem report and is invisible in any unit test
that reads the file after the run.

## 7. Watch out

**The UI already consumes a progress shape.** Find it and match it before
inventing a new one — file-level and repo-level grains both exist there. A
second shape means the UI and the daemon disagree about what a scan is doing.

**A sample is not a debug aid to be removed later.** It is the difference
between "the loop ran 48,646 times" and "the loop ran over files". Several
defects in this design's history were counts that looked right over the wrong
population.

## 8. Definition of done

- One append-only, tailable file; one line per stage per run; every line
  carries a sample.
- The denominator comes from the barrier, verified by test.
- Reconciling counts are asserted, and a failure reports both numbers.
- The UI's existing progress shape is matched, not duplicated.
