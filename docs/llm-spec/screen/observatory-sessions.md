# 録 · Observatory · Sessions

**Segment:** 03 · Observatory — daily use
**Route:** `/sessions`
**Source mockup:** [`lib/sessions-zen.jsx`](../../mockups/Sensei/lib/sessions-zen.jsx) → `SessionsDigestZen` (default variant: `trend`; variants: stream / constellation / bands / pulse)
**App file:** `app/src/routes/(observatory)/sessions/+page.svelte`

## Purpose

Sessions is where the user comes to answer **"what shape are my
sessions taking?"** Not a raw log — a **digest**. The mockup's
opinionated take: retro summary cards on top (7d / 30d totals),
then one quiet chart below that shows the shape at a glance. The user
can flip between four visual treatments (stream / constellation /
bands / pulse) depending on what question they're actually asking
(volume-over-time, correlation, per-day mix, or per-session micro-view).

The right variant for the daily-use case is **trend** — a compact
stacked area over the last 7 days that reads at a glance and matches
the mockup default. The other variants are reachable through a chip
group and should ship as a set — omitting three of them makes the
screen feel truncated.

Every session card and every chart element is colour-coded on
three quality states:
- **good** (success) — FTR-true, shipped
- **bad** (warning) — corrected, shipped
- **ugly** (accent) — abandoned

Kanji is 録 — *record*.

## Data invariants

- `GET /api/sessions?range=7d|30d|90d` returns a list of
  `activity.sessions` rows with:
  - `id`, `title` (nullable), `project` (name — resolved server-side),
    `when` (relative label), `time` (HH:MM), `duration` (compact
    "12m" / "1h 04m"), `corrections`, `ftr`, `agent`, `outcome`
  - The daemon may synthesize additional history for prior-week
    comparisons; each synthesized row is tagged `synthetic: true`
    so the UI can mute or hide them at the user's discretion. The
    mockup shows them; the spec keeps them.
- Session-id resolution: `activity.assistant_events.session_id`
  stores the **client-side session id**, not the observatory
  `sessions.id`. Any deep-dive that joins events must resolve
  through `activity.sessions.client_session_id`. This has burned
  us before and is called out in the wrong-gate below.
- `outcome` enum is `shipped | corrected | abandoned` — the daemon
  is the source of truth for these labels.

## Signals shown

| Element | Value | Meaning | Example |
|---|---|---|---|
| Range chips | `7d`, `30d`, `90d`, `all` | Time window | selected: `7d` |
| Totals row | `{count} sessions across {projects} projects · median {mins}m` | Header stat strip | `27 sessions across 4 projects · median 38m` |
| Quality tally | `{good} first-try · {bad} corrected · {ugly} abandoned` | Coloured pill trio | `19 · 6 · 2` |
| Chart chip group | `trend`, `stream`, `constellation`, `bands`, `pulse` | Visual variant switch | `trend` (default) |
| Chart body | shape depends on variant | See mockup for each | — |
| Session row | `time · project · duration · corrections · ftr · outcome` | Clickable to Replay | `09:14 · sensei · 42m · 0 corrections · ✓ FTR · shipped` |
| Sparkline per row | duration ticks by minute | Micro-shape at a glance | thin bars |
| Session hero (on hover) | title + first prompt snippet | Preview before drilling in | 2 lines |

## Done gate

- On Jerry's live data the range chips filter the query and update
  the totals row + chart + list in step.
- The default chart is `trend` (matching the mockup); switching to
  each of `stream / constellation / bands / pulse` renders without
  error and preserves the current range.
- Every session row's FTR badge agrees with the session's underlying
  `ftr` column — no rows with `FTR ✓` where Replay says "no tool
  calls".
- Clicking a session row navigates to Replay for that session, with
  the events populated (session-id resolution honored — the fix must
  survive).
- The synthesized-history rows visually distinguish from captured
  rows (mockup uses `synthetic: true`; UI should mute them or add a
  small chip).
- Dark-mode: all three quality tones remain distinguishable.

Optional check:
```
curl -s http://localhost:7744/api/sessions?range=7d | jq '{n: length, ftr_true: [.[] | select(.ftr)] | length, abandoned: [.[] | select(.outcome == "abandoned")] | length}'
# expected: ftr_true + non-ftr + abandoned = n
```

## Wrong gate

- **Session shows `FTR ✓` but the Replay panel says "no tool calls".**
  Session-id vs client-session-id join broken. Regression test:
  a session whose events are actually present must resolve.
- **Chart flips to a variant and the totals row goes stale.** Chart
  and header share the same filtered slice — one derivation source.
- **Range chip changes but the URL / state doesn't reflect it.** No
  deep-linkable session-list view.
- **Only two chart variants render.** Missing wire-up for the other
  three — do not ship a subset.
- **Every session tagged `agent: "claude-code"` regardless of the
  captured assistant family.** Fallback default masking real data.
- **`synthetic` rows counted in the totals but shown identically to
  real ones.** Either exclude synthetic from totals or mark them
  visually — pick one, do it consistently.
- **Session preview snippet leaks into a card title untruncated.**
  Layout bug that ruins the scan.

## Related

- [[pipeline/capture]] — populates `activity.sessions`
- [[pipeline/ftr]] — the FTR column and outcome derivation
- [[pipeline/analyzer]] — the enrichment that writes the columns
- [[screen/observatory-instruments-replay]] — where a session row lands
- [[screen/project-sessions]] — the project-scoped version of this screen
