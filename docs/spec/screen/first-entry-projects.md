# 場 · First run · First entry — Projects

**Segment:** 02 · First run & Preferences
**Route:** `/projects` (with the first-entry banner)
**Source mockup:** [`lib/observatory-today.jsx`](../../mockups/Sensei/lib/observatory-today.jsx) + [`lib/navigation.jsx`](../../mockups/Sensei/lib/navigation.jsx) → shared with the daily Projects screen
**App file:** `app/src/routes/(observatory)/projects/+page.svelte`

## Purpose

The user just added roots and hit "Enter your workspace". The
first thing they should see is **their own projects appearing**,
not a tutorial. Value before setup — the mockup's #1 theme.

This screen is the same primitive as
[[screen/observatory-projects]], with three additions specific
to first-entry:

1. **A banner strip** at the top acknowledging the scan is still
   completing and inviting the user to explore what's already
   there.
2. **A guided first session prompt** — a small quiet-day nudge
   suggesting the user open their assistant and try a real
   session so sensei has activity to work with.
3. **A "return to setup" link** for adding more roots or
   revisiting the scan, without a wizard trap.

Kanji is 場 — *place*.

## Data invariants

- User arrived from [[screen/first-run-scan]] with at least one
  root added.
- Scan may still be running — the banner shows scan progress
  ("scanning 3 of 7 folders").
- `GET /api/projects` returns whatever's been discovered so far;
  the list grows as the scan completes.
- The first-entry banner disappears automatically once the scan
  reaches `watching` state for every added root, or on manual
  dismiss.
- `POST /api/preferences/first-entry-seen` records that this
  screen has been shown — subsequent visits go straight to the
  regular Projects screen.

## Signals shown

Same as [[screen/observatory-projects]], plus:

| Element | Value |
|---|---|
| First-entry banner | scan status + progress + "still working" note |
| Banner dismiss | small X |
| Guided-session card (bottom) | "open your assistant and try a real session — sensei is listening" |
| Add more roots link | small mono, links to Preferences → Scan |
| Return to scan link | small mono, links back to first-run-scan |

## Done gate

- Loading this screen after first-run-scan renders the discovered
  projects immediately, even while the scan continues.
- The banner shows real progress from the scanner, not a fake
  spinner.
- The guided-session card appears only when zero
  `activity.sessions` rows exist for this user (i.e. sensei has
  no activity yet). Once activity accrues, the card disappears
  and the peer [[screen/observatory-today]] takes over the
  early-state messaging.
- Dismissing the banner marks it dismissed for this session;
  it doesn't reappear until a new root is added.
- After the first-entry-seen flag is set, subsequent Projects
  visits skip the banner.
- Every other Projects primitive from
  [[screen/observatory-projects]] works — filters, search, view
  toggle, cards / list.

Optional check:
```
curl -s http://localhost:7744/api/preferences | jq '.first_entry_seen'
```

## Wrong gate

- **Projects appear only after the whole scan finishes.**
  Progressive rendering broken — value-before-setup fails.
- **Banner sticks around after every root has finished
  scanning.** Auto-dismiss regressed.
- **Guided-session card shows even after sessions accrue.**
  Detection query missing.
- **Return-to-scan link opens a modal that traps the user.**
  Must be a nav that keeps state.
- **First-entry banner covers the filter chips.** Layout bug —
  banner is a strip, not an overlay.
- **The first-entry-seen flag is set immediately on load.**
  Should be set on first meaningful interaction (click a card,
  dismiss banner) so a user who bounces still sees the banner
  next time.

## Related

- [[screen/first-run-scan]] — where the user arrived from
- [[screen/observatory-projects]] — the daily variant of this
  primitive
- [[screen/observatory-today]] — takes over early-state messaging
  once activity accrues
- [[screen/preferences]] — where "add more roots" leads
- [[pipeline/capture]] — the ongoing scan
