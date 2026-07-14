# 録 · Observatory · Sessions

**Segment:** 03 · Observatory — daily use
**Route:** `/sessions`
**Source mockup:** [`lib/observatory/sessions-zen.jsx`](../../mockups/Sensei/lib/observatory/sessions-zen.jsx) → `SessionsDigestZen` (full chart chip group: `trend / stream / constellation / bands`, default `trend`; mini-cycler adds `pulse`)
**App file:** `app/src/routes/(observatory)/sessions/+page.svelte`

## Purpose

Sessions is where the user comes to answer **"what shape are my
sessions taking?"** Not a raw log — a **digest**. The mockup's
opinionated take: retro summary cards on top (7d / 30d totals),
then one quiet chart below that shows the shape at a glance. The user
can flip between four full chart treatments (trend / stream /
constellation / bands) depending on what question they're actually
asking (FTR-over-time, volume-over-time, correlation, or per-day mix).
A fifth mode, `pulse`, is available in the collapsed-header mini-cycler
only — it is not one of the full chart chip options.

The right variant for the daily-use case is **trend** — a compact
line-and-area chart over the selected range that reads at a glance and
matches the mockup default. The other three full chart variants are
reachable through the chip group and should ship as a set — omitting
any of them makes the screen feel truncated.

Per-day aggregation for trend / stream / bands is computed CLIENT-SIDE
from `startedAt`, `outcome`, and `ftr` — there is no server-side
`/api/sessions/aggregate` endpoint.

Every session card and every chart element is colour-coded on
three quality states (plus a neutral for edge cases):
- **good** (success) — `completed` with `ftr = true`
- **bad** (warning) — `corrected`
- **ugly** (accent) — `abandoned`
- neutral (no tint) — `blocked` or `partial`

Kanji is 録 — *record*.

## Data invariants

**Prerequisite backend work (two items):**
- The `list_all_sessions` handler (`GET /api/sessions`) must be
  **extended** to accept an optional `?range=7d|30d|90d` query parameter
  that filters rows where `activity.sessions.started_at >= now() − range`.
  Without this the range chips have nothing to call. `?project=<name>`
  likewise scopes by project name. Both parameters are additive and may
  be combined.
- The SELECT must include `assistant_family` so the client can populate
  the `agent` display field. This is the only new column required.

**Wire shape** — the endpoint returns these fields per row:
`{ id, project, task, summary, outcome, ftr, turns, corrections,
   startedAt, completedAt, assistant_family }`

The following display values are **derived client-side**:
- `title` — display alias for the wire field `task`
- `when` (relative label, e.g. "yesterday") — derived from `startedAt`
- `time` (HH:MM) — derived from `startedAt`
- `duration` (compact "12m" / "1h 04m") — derived from `startedAt` and
  `completedAt`
- `agent` — wire field is `assistant_family`; displayed as-is

No server-side synthesis or derivation is expected for any of these fields.

**Real sessions only** — the screen shows only real captured sessions.
No client-side or server-side synthesis of history rows. ~216 real
sessions are present in the corpus.

**`outcome` enum** — the real `sensei.session_outcome` values are
`completed | corrected | blocked | partial | abandoned`. Quality mapping:
- **good** = `completed` where `ftr = true`
- **bad/warn** = `corrected`
- **ugly** = `abandoned`
- `blocked` and `partial` render neutral (no quality tint)

**Session-id resolution** — `activity.assistant_events.session_id`
stores the **client-side session id**, not the observatory `sessions.id`.
Any deep-dive that joins events must resolve through
`activity.sessions.client_session_id`. Row-click navigation uses
`GET /api/sessions/{id}` (shipped as of Slot 1) for the resolution.
This has burned us before and is called out in the wrong-gate below.

## Signals shown

| Element | Value | Meaning | Example |
|---|---|---|---|
| Range chips | `7d`, `30d`, `90d`, `all` | Time window | selected: `7d` |
| Totals row | `{count} sessions across {projects} projects · median {mins}m` | Header stat strip | `27 sessions across 4 projects · median 38m` |
| Quality tally | `{good} first-try · {bad} corrected · {ugly} abandoned` | Coloured pill trio | `19 · 6 · 2` |
| Chart chip group | `trend`, `stream`, `constellation`, `bands` | Full chart selector (4 chips) | `trend` (default) |
| Mini-cycler badge | `numbers`, `trend`, `stream`, `constellation`, `bands`, `pulse` | Collapsed-header cycling widget | cycles through 6 modes |
| Chart body | shape depends on variant | See mockup for each | — |
| Session row | `time · project · duration · corrections · ftr · outcome` | Clickable to Replay | `09:14 · sensei · 42m · 0 corrections · ✓ FTR · completed` |
| Sparkline per row | duration ticks by minute | Micro-shape at a glance | thin bars |
| Session hero (on hover) | title + first prompt snippet | Preview before drilling in | 2 lines |

## Done gate

- The backend `?range=` extension is in place; on Jerry's live data
  the range chips filter the query and update the totals row, chart,
  and list in step.
- The default chart is `trend` (matching the mockup); switching to
  each of `stream / constellation / bands` renders without error and
  preserves the current range. The `pulse` mini-cycler cycles in the
  collapsed header without error.
- Every session row's FTR badge agrees with the session's underlying
  `ftr` column — no rows with `FTR ✓` where Replay says "no tool
  calls".
- Clicking a session row calls `goto('/instruments?tab=replay&session={id}')`;
  the instruments page populates the replay panel for that session (the
  `session` query param is honored — cross-screen dependency on
  `(observatory)/instruments/+page.svelte`). Session-id resolution uses
  `GET /api/sessions/{id}`.
- No synthetic rows appear in the list or in the totals.
- Dark-mode: all three quality tones remain distinguishable.

Optional check:
```
# Confirm the range filter is active: 7d count must be less than the total
curl -s http://localhost:7744/api/sessions | jq 'length'
curl -s "http://localhost:7744/api/sessions?range=7d" | jq '{n: length, ftr_true: [.[] | select(.ftr)] | length, abandoned: [.[] | select(.outcome == "abandoned")] | length}'
# expected: n < total session count (confirms filter is reducing the set)
# expected: ftr_true + corrected + abandoned ≤ n (outcome categories sum correctly)
```

## Wrong gate

- **Session shows `FTR ✓` but the Replay panel says "no tool calls".**
  Session-id vs client-session-id join broken. Regression test:
  a session whose events are actually present must resolve.
- **Chart flips to a variant and the totals row goes stale.** Chart
  and header share the same filtered slice — one derivation source.
- **Range chip changes but the URL / state doesn't reflect it.** No
  deep-linkable session-list view.
- **Only some chart variants render.** All four chips (`trend / stream /
  constellation / bands`) must be wired — do not ship a subset. `pulse`
  is not a chip; if it appears in the chip group that is also a bug.
- **Every session tagged `agent: "claude-code"` regardless of the
  captured assistant family.** Fallback default masking real data.
- **Synthetic rows appear in the list or totals.** No synthetic history
  exists — if fabricated rows surface, a client-side synthesis function
  has been re-introduced by mistake and must be removed.
- **Session preview snippet leaks into a card title untruncated.**
  Layout bug that ruins the scan.

## Related

- [[pipeline/capture]] — populates `activity.sessions`
- [[pipeline/ftr]] — the FTR column and outcome derivation
- [[pipeline/analyzer]] — the enrichment that writes the columns
- [[screen/observatory-instruments-replay]] — where a session row lands
- [[screen/project-sessions]] — the project-scoped version of this screen
