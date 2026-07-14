# 完 · Bootstrap · All green

**Segment:** 01 · Bootstrap
**Route:** initial splash window, terminal state
**Source mockup:** [`lib/setup/bootstrap-splash.jsx`](../../mockups/Sensei/lib/setup/bootstrap-splash.jsx) → `Splash` (state = `green`)
**App file:** `app/src/routes/bootstrap/+page.svelte`

## Purpose

The all-green state is the payoff. Six gates checked, every one
green, one calm line telling the user the machine is ready. No
"click continue" — the transition into the Observatory first-run
happens on its own within a beat.

**Calm by default.** Big kanji (`完`), a short line ("everything
ready · opening your workspace"), and an animation that ushers
the user into the next step. If the user needs the health details
back later, the Observatory always exposes a **Re-check** entry
that returns to the probing surface.

Kanji is 完 — *complete*.

## Data invariants

- Reached when every gate in `GET /api/bootstrap/status`
  reports `status == "pass"`.
- Transition to the Observatory or the First-run scan step
  ([[screen/first-run-scan]]) happens after a bounded delay
  (default 800ms) so the user reads "ready".
- If bootstrap was triggered via **Re-check** from within the
  Observatory (not a fresh install), the green state closes back
  into the Observatory main window rather than the first-run scan.

## Signals shown

| Element | Value |
|---|---|
| Kanji hero | `完` — large |
| Line 1 | "everything ready" or the mockup's phrase |
| Line 2 | "opening your workspace" (fresh install) or "you may continue" (re-check) |
| Elapsed timer | mono, top-right, frozen at final elapsed |
| Continue link | small, keyboard-focusable, in case the auto-transition stalls |

## Done gate

- Reached only when every gate is `pass` — a single `remedy` or
  `fail` gate holds the probing state.
- Renders no more than 800ms before the auto-transition fires.
- The context (fresh install vs re-check) picks the correct next
  destination.
- The Continue link is present as an escape hatch and receives
  focus when the auto-transition doesn't fire within 3s.
- No lingering `remedy` remediation modals from the probing
  state.

## Wrong gate

- **All-green state renders while a `remedy` gate is still
  pending.** Gate aggregate wrong; some gates were counted as
  pass when they shouldn't have been.
- **Auto-transition never fires and Continue link is absent.**
  User trapped on the splash.
- **Fresh install lands directly in the Observatory home
  instead of first-run scan.** Context detection wrong; the
  user gets no chance to add roots before projects appear
  empty.
- **Re-check from Observatory closes the app.** Wrong context
  handler.
- **`完` glyph rendered as a fallback square box.** Font
  package didn't load.

## Related

- [[screen/bootstrap-probing]] — the state we came from
- [[screen/first-run-scan]] — where fresh install goes next
- [[screen/observatory-today]] — where re-check returns to
