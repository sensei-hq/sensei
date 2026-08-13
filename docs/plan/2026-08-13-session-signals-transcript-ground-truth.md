# Session Signals — Transcript-Ground-Truth Redesign

_Plan of record. Status: draft (pre-implementation). Author date: 2026-08-13.
Supersedes the observation half of
[`docs/analysis/metric-explainability-generation.md`](../analysis/metric-explainability-generation.md)._

## 1. Why

The metric drill-down (shipped v0.7.5–0.7.6) did its job **too well**: it surfaced
that the session signals feeding the metrics are unreliable and the LLM narratives
attached to them are confabulated. Concretely, on the live `sensei` project:

- Sessions are labelled **`abandoned`** that the user never abandoned.
- The per-(session, metric) observations read as **self-contradictions**
  ("high FTR **and** abandoned due to difficulty").

Both trace to one architectural mistake: **the analyzer derives session signals
from sparse hook events and ignores the transcript, which is far richer.**

## 2. Core principle (guiding the whole redesign)

> **The transcript is the primary, ground-truth source of what happened in a
> session. Hook events (`activity.assistant_events`) are supplementary — used to
> corroborate and enrich, never as the sole basis for a signal.**

`activity.transcript_turns` carries the real per-turn `user_text` +
`assistant_text` (+ `turn_index`, `char_count`, `model`, `provider`). Hook events
are frequently incomplete (a session can miss `SessionEnd`, undercount turns, or
capture nothing at all) — so any signal derived from them alone is unsound.

## 3. Findings (data-backed, live `sensei` DB, 2026-08-13)

**How the drill-down's two statements are generated.** Each session shows an
observation **title** (header) + **detail** (inline) from one
`SessionMetricObservation` insight-copy call, fed
`{metric, meaning, outcome, ftr, corrections, turns, task, summary}` where
`meaning` = the metric's **generic `how_to_read`**. The model mashes that generic
reading rule onto the (wrong) `outcome` label and **invents a causal "why"** →
"abandoned due to high FTR." It is (a) conflating a general rule with this
session, (b) inventing causality absent from the data, (c) circular (fed
"abandoned", concludes "abandoned"), (d) citing **no evidence**. The
`session_retro` "Abandoned Task — …" summary is the same class of confabulation.

**The outcome classifier.** `derive_outcome` (`tasks/handlers/analyze.rs`):
clean end (`Stop`/`SessionEnd`) → `corrected`/`completed`; else tail error
cluster → `blocked`; **else → `abandoned`.** So `abandoned` ≡ "no end event +
no tail errors" — an *absence*, not a signal.

**Scale of the mislabel.** Global hook events: `SessionStart` **139** vs
`SessionEnd` **81** → **~42% of sessions have no end event.** A missing
`SessionEnd` is the crash / window-close / not-captured case — never evidence of
abandonment.

**The 6 "abandoned" sessions (sensei project):**

| session | session.turns | transcript turns | reality |
|---|---|---|---|
| 9248C958, 9E219F05, c63f15c7 | 0 | 0 | **empty** — nothing attempted |
| 792d7ce4 | 1 | **94** | real 94-turn working session (first line: *"you are absolutely right. scan is a read operation…"*) |
| f139b975 | 1 | 33 | real 33-turn session |
| 6783dda9 | 5 | 0 | hook/transcript mismatch |

- `792d7ce4`: **94 real transcript turns** but tagged `abandoned, turns=1` — the
  turn count came from sparse hooks, not the transcript.
- `9E219F05`: a **`command_invoked{action:"resume", session_id:"9E219F05…"}`**
  event references it → it was **resumed**, not abandoned.
- All 6 are `backfilled=f` (live), with `tokens_in/out` NULL — the token capture
  didn't fire either.

**Signals available to correlate (answering "what props do we have"):**

- **Context size:** `sessions.tokens_in`/`tokens_out`; `transcript_turns.char_count`;
  **`PreCompact` events (45 captured)** — compaction = hard context-pressure signal.
- **Lifecycle:** `SessionStart`/`SessionEnd`, `sessions.duration`, `sessions.backfilled`.
- **Linkage:** `command_invoked{action:"resume"|…, command:"session", session_id}`
  ties a resume to a prior session; `checkpoint` events (9) mark resume points.
- **Claude trouble hints:** detectable in `assistant_text` (16 turns match
  "resume later / running low on context / start a new session / abandon").
- **Extensibility:** `sessions.props` + `assistant_events.payload` (jsonb).

## 4. The redesigned signal model

### Phase A — Transcript-first derivation + honest outcome taxonomy
Re-source the per-session signals from the transcript, hooks corroborating:
- `turns` = transcript turn count (`792d7ce4` → 94, not 1).
- `corrections`/`ftr` = derived from transcript **content** (user correction turns
  in `user_text`), not hook counts. (Detection rule is an open question — §6.)
- Outcome taxonomy (no signal invented from an absence):
  - **empty** — 0 transcript turns (and 0 hook turns): not a measured outcome;
    **excluded** from throughput/ftr (never rendered as a signal card).
  - **incomplete** — real work, no clean end, no resume-link: neutral (crash /
    close), **not** a failure, **not** abandoned.
  - **completed** / **corrected** — the transcript shows completion (± user
    corrections), or a clean `Stop`/`SessionEnd`.
  - **abandoned** — ONLY on a **positive** signal: an explicit user/Claude
    abandonment in the transcript, AND no resume-link. Never from a missing end.

### Phase B — Session lineage (crashed ↔ resumed)
- Link `command_invoked{action:resume, session_id:X}` (and resume/restart user
  messages) → prior session `X`. Reconstruct crashed→resumed **chains**.
- A session that was later resumed is **continued**, not abandoned; the chain's
  terminal session carries the outcome. Store the lineage in `sessions.props`
  (e.g. `props.resumed_from` / `props.resumed_by`) or a dedicated link.

### Phase C — Evidence field (replaces the confabulated observation)
- Every datapoint/session carries a **deterministic `evidence`** field: the core
  transcript turn(s) that drive the signal — the actual correction turn (ftr), the
  resume command (lineage), Claude's suggestion (trouble), or the completion —
  quoted/verifiable, sourced from `transcript_turns`. **No invented causality.**
- The per-metric observation becomes a plain factual contribution line, OR (if it
  stays LLM) is **strictly grounded in the cited evidence** — never the generic
  `how_to_read` remix, never a causal "why" the transcript doesn't support.

### Phase D — Trouble-signal capture + context correlation
- Collect Claude's **abandon / stop / resume-later / out-of-context** suggestions
  (from `assistant_text`) as "something's going wrong" cases.
- Correlate each with context pressure: `PreCompact` count, tokens, `duration`,
  transcript turn count → surface *why* (context exhaustion, stuck loop).
- A new signal family (not a metric value): a case list on the drill-down / a
  dedicated view.

### Phase E — Metric-specific action items
- Replace the project-wide recommendations panel on the metric detail (same on
  every metric — redundant with Impact) with **metric-specific** actions or the
  per-metric inference guidance. (Confirmed direction; mechanism TBD in §6.)

## 5. Acceptance criteria (observable)
- `792d7ce4` reads `turns=94` and outcome ≠ `abandoned`.
- `9E219F05` reads as **resumed/continued** (linked to its resume), not abandoned.
- Every 0-turn/0-transcript session is **excluded** from throughput/ftr (no card).
- No session is `abandoned` without a positive transcript signal.
- The drill-down shows an `evidence` field quoting a **real transcript turn**;
  no "high FTR + abandoned"–style contradictions remain.
- Trouble-cases: Claude stop/abandon suggestions are collected and each carries
  its `PreCompact`/token/duration correlation.

## 6. Open questions / risks (resolve before the phase that needs them)
- **Correction/FTR detection from transcript text** (Phase A): what rule marks a
  `user_text` turn as a "correction"? Heuristic (negation/redo cues) vs a small
  classifier vs reusing the existing correction detector on transcript instead of
  hooks. Risk: a noisy new heuristic. **Must pin before Phase A.**
- **Backfill re-derivation:** Phases A–B change existing rows → a one-time
  re-derive over historical sessions (cost: transcript scan over all sessions).
- **Cost:** transcript-first derivation is heavier than hook counting; keep it in
  the off-wire analyzer/backfill, never on a read path.
- **Metric→action mapping** (Phase E): recommendations have no per-metric link;
  decide heuristic-relevance vs a new per-metric inference field.

## 7. Sequencing (forward-only — no earlier phase depends on a later one)
A (transcript-first derivation + taxonomy) → B (lineage) → C (evidence field) →
D (trouble-signals) → E (metric-specific actions). A is the foundation: correct
outcomes + transcript-sourced turns unblock everything downstream.
