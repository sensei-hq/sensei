# Observatory · Today — follow-ups (non-blocking)

Slot 1 shipped 2026-07-07: spec-doc-reviewer ✅, done-gate-verifier **ready-to-ship**,
wrong-gate-hunter **clean**, sensei-persona-reviewer ran. These are the persona-review
items deliberately deferred (each evaluated, none block the screen — done+wrong gates pass).

## AWAITS: Jerry (needs a decision he owns)
- **Koan provenance is hollow on live data** — the flagship's trust story ("koan points at
  real sessions · noticed N days ago") is empty because:
  1. `inference.recommendations` has **no creation timestamp** (`acted_at`/`measured_at` only,
     both null while pending), so `noticed` can't be derived. Powering it needs a
     `created_at`/`detected_at` column (a DDL change — off-limits this run). The
     `relative_when()` helper (observatory.rs:682) is ready once a timestamp exists.
  2. The top pending rec carries no session-id evidence, so `hero.source` is "". Needs the
     L2/L3 rec generator to link evidence sessions. Then the koan gets real receipts.

## Deferred to existing tracks
- **Insight cards are monotone** — all live insights render kanji `繰` + label
  "Recommendation". The mockup differentiated pattern-recurring / teaching-adopted /
  drift-detected. Needs recs to carry a category discriminator → **narration-cache pipeline
  (#65)**. Spec permits fallback copy until wired.
- **Adopted `what` is raw prose** — memory titles are captured verbatim (conversation
  fragments), not distilled teachings. Same narration-cache distillation pass. `adopted_row()`
  trims whitespace but doesn't cap length; `AdoptedCard.svelte` wraps rather than truncates.

## Small, self-contained (any future session)
- **Steady-state dead-end** — `steady_hero()` (mature + zero pending recs) has `action: null`
  and empty insights → no forward motion. Unreachable on current data (337 pending recs).
  Doing it right: add `actionHref` to the hero wire shape + `HeroKoan.svelte`, give steady a
  nav CTA (e.g. "Open sessions" → /sessions).
- **`RecentSessions.svelte` type drift** — declares a local `Session` interface instead of
  importing `RecentSession` from `recent-sessions.ts` (pre-existing; DRY). Import to close it.
- **Perf micro-opt** — `get_global_maturity_inputs()` + `list_all_sessions(20)` are
  unconditional + independent; could `tokio::try_join!`. Noise on localhost; flagged for the
  flagship.
- **UUID display form** — when `hero.source` populates from evidence it will show raw UUIDs;
  the mockup uses short `s-XXXX`. Truncate for display (e.g. first 8 chars) when it goes live.

## Evaluated, no action
- **`good` insight tone unreachable** — `insight_tone()` only emits warn/mute from rec
  urgency; the success path exists + is unit-tested, just not produced by rec data. Future-proof.
- **Asymmetric error handling** — maturity uses `?` (logs + 500), recs/mems use
  `unwrap_or_default()`. Harmless: the client `getObservatoryToday()` falls back to the early
  state on 500, so the user sees "still listening", not a blank screen, and the server still
  logs the failure (better observability than degrade-in-place). Left as-is intentionally.
