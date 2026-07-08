# Observatory · Sessions (digest) — follow-ups (non-blocking)

Slot 6 shipped 2026-07-08 (final target slot): spec-doc-reviewer ✅ (2 rounds),
done-gate-verifier all 9 code/API gates PASS, wrong-gate-hunter CLEAN (session-id resolution
regression clean — 148 replay calls), sensei-persona-reviewer. Screen at `(observatory)/sessions/`;
Replay deep-link wired into `(observatory)/instruments/+page.svelte`.

## FIXED before commit (persona P0 — latent correctness in new chart code)
- **BandsChart/StreamChart scaled + captioned by total `mins` (incl. neutral) while drawing only
  good+bad+ugly segments** — so with any blocked/partial session the axis would inflate and the
  caption overstate. Added `chartedMins` (= good+bad+ugly minutes) to the day bucket and used it for
  both chart scales + the bands caption. Latent today (no neutral sessions), now correct. +1 test.

## Deferred (persona enhancements / spec-signals gaps, none blocking)
- **`all` range chip missing.** Spec signals table lists 7d/30d/90d/**all**; UI has 7d/30d/90d.
  Backend already supports no-filter (`range_to_days(None)→None`). Adding `all` needs the chart
  day-axis built from actual session dates (not a fixed count), so it's more than a one-liner. Some
  transcript-synthesized sessions may be >90d old and are currently unreachable.
- **Range selection not persisted in the URL.** `setRange` refetches but doesn't `goto`/`pushState`,
  so a reload resets to 7d and 30d/90d can't be deep-linked. (The persona read this as a spec
  wrong-gate; the wrong-gate-hunter didn't list it — treat as enhancement: the chip DOES filter.)
- **Mini-cycler headline is always "N sessions"** regardless of mode — the mockup computed a
  per-mode headline (trend→"% first-try today", etc.). Low-cost UX win.
- **Quality tally omits neutral** (good·corrected·abandoned only) — when a blocked/partial session
  exists the tally won't sum to the total count. Add a neutral tally when >0.
- **Collapse-on-scroll header not implemented** — the mockup promotes the mini-cycler into a
  collapsed header past 30px scroll; current header is static. Simplification, not a regression.
- **Retrospective section** (Going well / Not going well / Insights lanes below the chart) is not
  built — scoped for a later slot; the screen ends at the session list.
- **Accessibility**: quality dot in SessionRow is color-only — add a text label for color-blind users;
  add `aria-label` to the chart region.
- **`SessionsDigestState` has no explicit state-transition spec** (setRange loading/error path).
- **`GET /api/sessions/{id}` returns snake_case** keys (`started_at`) vs the list's camelCase
  (`startedAt`) — latent; current screens read the list. Normalize when a consumer needs the single
  endpoint's timestamps.
- **Wire field is `agent`, spec said `assistant_family`** — there's no `assistant_family` column;
  `agent` = `activity.sessions.acp_id` (the harness, "claude"/"zed"). Backend↔frontend agree; align
  the spec's field name in a future cleanup. `provider`/`model` exist if per-model labels are wanted.

## Backend data quality (not this screen)
- Some sessions show implausibly long durations (20h+/35h) — likely `completed_at` set by a later
  Stop event or a daemon restart, not the session's own stop. The client reflects the data faithfully;
  fix session-lifecycle close-time tracking, and/or cap the displayed duration with a flag.
