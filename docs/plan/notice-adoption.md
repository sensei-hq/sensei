---
name: Notice adoption
description: Where errors look and read differently across sensei and the dōjō today, and the phased plan to route every one through the shared Notice
date: 2026-11-03
---

# Notice adoption — one way to say something went wrong

The component exists: `packages/ui/Notice.svelte` with `notice.js` and
`glyph-mode.js` ([architecture](../architecture/notice.md)). This plan moves every
error, refusal and wait in **sensei** (`app/`) and the **dōjō** (`dojo/`) onto it.

## What is inconsistent today

Counted on 2026-11-03 (`grep text-danger | role="alert" | ScreenState | LoadError`).

### Sensei

| # | Pattern | Where | What's wrong with it |
|---|---|---|---|
| S1 | `ScreenState status="error"` | 27 route files | One generic "Couldn't load this" with a faded 難 for every screen; no code, no remedy beyond a daemon hint; the raw message is the explanation |
| S2 | `<p class="text-xs text-danger">{error}</p>` | ~45 lines in 30 files (instruments 7, memories 4, tickets 4, overview 3, impact 2, about 3, activity-logs 4, settings panels, …) | Four phrasings ("Could not read X: …", "Could not load X", "Could not record that: …", bare `{error}`); text-xs vs text-sm at random; no mark, so kanji/icon mode is ignored; `role="alert"` on only 6 of them |
| S3 | Bold red title boxes | ProvidersSection, LibrariesSection, setup/scan | A third look: semibold danger heading, no actions |
| S4 | Full-width red strips | activity-logs (tasks, rows) | A fourth look: padded strip with a border |
| S5 | `DaemonStatusBanner` | root layout | Warning strip, its own copy; not a Notice tone and no mark |
| S6 | Health `Remedy` | bootstrap | The reference design — but private to `/health`, so nothing else could reuse it |
| S7 | Sync rows' skip/fail reasons | Settings · Dōjō (`detailLine`) | Coloured text lines; remedies joined from `reason_codes` but rendered by hand |
| S8 | `window.confirm()` | overview recommendation decide | A browser dialog in a Tauri app; out of scope here, noted for the dialog work |

Not errors, and staying as they are: traceability "N broken" counts, HealthRadar /
MetricCard / impact verdict tones, destructive buttons' `hover:text-danger`.

### Dōjō

| # | Pattern | Where | What's wrong with it |
|---|---|---|---|
| D1 | `LoadError` | 11 screens | "Could not load {what} — {error}": a red line, hard-coded English outside paraglide |
| D2 | `LoadError` copied by hand | ScrSharing, ScrMyMetrics, ScrContributions, ScrRulePacks | The same sentence re-typed, so it already drifts ("— no answer") |
| D3 | Boxed alerts | AzureConnect, ScrIntegrations (danger-soft fill), ScrMyDojos (paper fill, red text) | Two box styles for the same thing |
| D4 | `Banner tone="warning"` used for errors | ScrInbox ("Couldn't reach the dōjō"), RelayList (partial) | A notice band doing an error's job; no code, no retry |
| D5 | Fixed toast | org section `actionError` | A fifth look, bottom-right, only here |
| D6 | Inline action errors | InvitePanel, RelayAskCard, RelayNotifyToggle, ScrOrgHome, DojoSignIn | Red spans of varying size beside buttons |

### Both

- **No `+error.svelte`** in either app: a load that throws shows SvelteKit's default page.
- **Words:** "Could not" and "Couldn't" both in use; raw server messages as headlines; a remedy almost never; a code never.
- **Marks:** most error lines have none, so icon mode has nothing to switch.

## Plan

Each phase is independently shippable. Keep every existing `data-testid` /
`data-*` hook: Notice passes `data-*` to its root, so e2e selectors survive.

### P0 · The component — done (2026-11-03)

- `Notice.svelte`, `notice.js` (tones, marks, `noticeFromApiError`, `noticeFromReason`), exported from `@sensei/ui`.
- Glyph-mode context moved to `packages/ui/glyph-mode.js`; the app's `$lib/glyphs` re-exports it, and the dōjō's root layout now provides it.
- Vocabulary: 止 → forbidden-circle, 待 → hourglass.
- Tests: `app/src/lib/notice.spec.ts`, `app/src/lib/components/Notice.spec.svelte.ts` (every tone × size × glyph mode, actions, script copy, snapshots).

### P1 · The two shared fallbacks (one change each, many screens)

1. **ScreenState's error branch → `<Notice size="screen">`** (S1, 27 screens). Add an optional `notice` prop (props from `noticeFromApiError`); without it, build one from `error` + `onretry` so no caller has to change on day one.
2. **Dōjō `LoadError` → `<Notice size="inline">`** (D1, 11 screens), `what` becoming the title through a paraglide message. Then delete the hand copies (D2).
3. **Health `Remedy` → `<Notice size="panel" tone="action" script=…>`** (S6). The bootstrap keeps its look; its spec asserts the copy/verify behaviour still holds.
4. **`+error.svelte` in both apps** using `size="screen"` with the status as the code.

### P2 · Sensei, screen by screen (S2–S4, S7)

Each screen's state module gains `notice` getters (via `noticeFromApiError`), and
the template swaps its red line for `<Notice size="inline">` (or `panel` where the
failure blanks a whole section). In order of how often people hit them:

1. Settings · Dōjō — the connect / renew errors from the mockup, and S7's sync reasons through `noticeFromReason`
2. Tickets (4) · Integrations (2)
3. Instruments, observatory (7) and workspace (2)
4. Memories (4) · Overview (3) · Impact (2) · About (3)
5. Activity logs (4, S4) · Traceability drift · Insights · Share review · Upgrades
6. Settings panels: Providers, Libraries (S3), Roots, Local models, Inference, Metrics, Extensions, Workspaces
7. setup/scan (S3) · SignInPending's copy error · the observatory FTR line

`DaemonStatusBanner` (S5) keeps its place as a strip but renders `<Notice size="inline" bare tone="normal|fault">`, so its mark and words follow the same rules.

### P3 · The dōjō (D3–D6)

1. Boxed alerts (D3) → `size="panel"`.
2. `Banner` used for errors (D4) → Notice; `Banner` stays for neutral notices.
3. The org toast (D5) keeps its position but renders `size="panel"`; one toast host in the console layout rather than one per section.
4. Inline action errors (D6) → `size="inline" bare`.
5. All dōjō copy through paraglide (`m.*`); `noticeFromApiError` gets the localized title.

### P4 · Reason codes carry the words

- The daemon and dōjō APIs join `kind`, `summary`, `detail`, `remedy`, `actor` (not only `remedy`) wherever they return a reason, so `noticeFromReason` needs nothing else.
- Register the dōjō connect codes (`account_mismatch`, `account_already_bound`, `oauth_timeout`, `dojo_unreachable`, `keychain_denied`, `org_access_restricted`, `session_ended`, `no_memberships`) in a `dojo_connect` domain.

### P5 · Keep it that way

- A spec in each app, like the glyph-vocabulary one, that fails on a new `text-danger` / `role="alert"` text line outside `Notice`, with an allowlist for the non-error uses above.
- Frontend guidelines §3: "Errors, refusals and waits render through `Notice`."
- A dōjō smoke spec mounting Notice under the dōjō's own vitest config, so the shared component is checked in both builds.

## Progress (2026-11-03)

- P0–P2 done in the app; its guard (`app/src/lib/error-surfaces.spec.ts`) is in place.
- P3 done in the dōjō: D3 boxes, D4 banners, D5 toast and D6 inline errors all render `Notice`. The copy comes from state getters (`errorNotice`, `incompleteNotice`, `actionNotice`, `partialNotice`) or, for prop-only screens, `dojo/src/lib/dojo-notices.ts`. The relay inbox copy is paraglide; the other new dōjō titles are English and still need `m.*` messages (P3.5).
- Inline dōjō notices pass `expanded`: next to a button, the reason is the point, so it is not hidden behind "Why?".
- P5 guard in the dōjō: `dojo/src/lib/error-surfaces.spec.ts`.
- Plan & billing is two plans (Free; Org at $12 per active contributor). Plan changes say they open once payments are connected (D-BILLING).

- The family is one set in `@sensei/ui` (L4 item 1): `StateView`, `EmptyState`, `Banner` and `Loading` beside `Notice`. The app's `ScreenState`, `EmptyState` and `DaemonStatusBanner` and the dōjō kit's `EmptyState`, `LoadError` and `Banner` are deleted, and every caller uses the shared ones. A failed screen always names a next step (`setStateDefaults` at the app root names the daemon). 401/403/404 read as refusals. A relay run that could not be read is a fault with Retry, no longer an empty state.
- Still open: the daemon-down case (the app cannot yet tell "down" from "not known yet"; `daemon-health.svelte.ts` needs a reachable flag), and remedies on the ~58 inline `failureNotice` calls.

## Done when

`grep -rn 'text-danger' app/src dojo/src --include=*.svelte` returns only the
allowlisted non-error uses, ScreenState and LoadError have no markup of their own
for errors, and both apps have an `+error.svelte`.
