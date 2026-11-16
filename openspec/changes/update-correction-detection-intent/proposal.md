# Change: Find corrections by intent, not key phrases (#447)

## Why
`correction_signal` (analyze.rs:112) is a substring match on fixed phrase lists; it is the only way a
user turn becomes a correction, so it bounds recall for every correction metric (per-turn flag,
session corrections, first-try-right, outcomes `corrected`/`abandoned`, correction-prone patterns,
recurring corrections). It misses plain corrections ("Hmm, I meant the other config file") and flags
plain requests ("add a revert button", "wait for the build, then deploy"). The LLM classifier that
exists can only remove phrase matches, never add one.

## What Changes
- Every user turn that follows an assistant turn is a candidate.
- The existing `correction | principle | neither` classifier labels candidates in batches on the
  local model (embedded gemma4, like the other analysis passes).
- With no model available, the phrase list is the fallback, and the label records its source
  (`intent` vs `phrase`) — never silently mixed.
- The per-turn flag, FTR and outcomes read the label; phrase-only metrics are re-derived on a backfill.
- A labelled set of real prompts (incl. the issue's examples) is a precision/recall test with a floor.

## Impact
- Specs: `session-analysis`. Code: `tasks/handlers/analyze.rs`, `prompt_classify.rs`, the analysis
  scheduler/backfill, `activity.turns` (label + source), metrics that read the flag.
