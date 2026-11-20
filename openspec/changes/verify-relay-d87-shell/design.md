# Design: verify-relay-d87-shell

## Current state (verified on `lane/relay-open-items`, which contains `231cbc660`)
| #420 item | Where it lives now |
|---|---|
| Height owned by ShellScreen | `packages/ui/ShellScreen.svelte:48` (`h-full min-h-0`); sidebar focusable `:53` |
| Pinned bands harvested | `packages/ui/ShellScreen.svelte:34,67-69,79`; Relay pins info when empty at `app/src/lib/relay/relay-shell.ts:335` |
| Shell as data | `app/src/lib/relay/relay-shell.ts` (`toRelayShell`), drawn by `app/src/lib/relay/RelayShell.svelte:40` |
| Both windows draw through RelayShell | `app/src/lib/relay/RelayScreen.svelte:196` (used by `(observatory)/relay` and `(workspace)/workspace/[id]/relay`). The task page (`RelayTaskScreen.svelte:42`) and run page (`RelayRunScreen.svelte:29`) draw `ShellScreen` directly, not `RelayShell` |
| "not reported", never 0 | `app/src/lib/relay/relay-shell.ts:199` (info band Today), `:213` (Waiting "oldest not reported"); footer `:251-253,260,262` |
| Schema filters (#384) | `app/src/lib/relay/relay-shell.ts:281-292` (`filterSchema` → `FilterSchema`); `app/package.json:33` `@rokkit/forms` |
| Loading state | unit-pinned: `app/src/lib/relay/relay-shell.spec.ts:106,206-208` |
| Error and loaded in both windows; no crew in the workspace window | `app/e2e/tests/relay-conformance.spec.ts` (3 tests) |

## Decisions
- Loading stays pinned by unit specs, not e2e. The harness cannot hold a read open; the
  conformance spec header says so and the issue's last comment accepted 3 of 3.
- No crew is proven in the workspace window only. The main window lists every crew in the e2e DB,
  so it cannot be held empty; the no-run shell itself is window-independent and unit-pinned
  (`relay-shell.spec.ts` "with no run").
- Slice 1 does not reopen the phone findings: `update-phone-layout` owns the base layout. Slice 2 adds only
  what the user chose on top of it (see below), so the base fix is not specified twice.

## Slice 2 — the compact phone header (follow-up, user decision 2026-11-18)

### Where it stands (verified on `f880251b8`)
| Piece | Where |
|---|---|
| Phone info band starts closed | `packages/ui/ShellScreen.svelte:62-66` (`MediaQuery('(max-width: 767px)')`; a remembered choice and a pinned band still win) |
| Stats 2-up, labels wrap on phone | rokkit `screen-shell.css` / `stat-card.css` via `update-phone-layout` tasks 2.1, 2.2 (done) |
| Stat titles | `app/src/lib/relay/relay-shell.ts:226-229`: "Working", "Waiting", "Blocked", "Completed today" |
| Toolbar | `relay-shell.ts:165-190`: a full list the screen owns (Now, Queue, Crew, Message, Info); rokkit's default band toggles are not used, so there is no filters toggle today |
| Filters band | `relay-shell.ts:282-292` (`filterSchema`: repository, workspace), values from `filterState` `:297-301` (`repo`, `workspace`, plus `show` from a stat), changes routed by `routeFilter` `:304-313`; drawn by rokkit `ScreenShell` as a band of `FormRenderer` controls |
| Band ids | rokkit `@rokkit/app` 1.15.7 `PanelId = 'info' | 'stats' | 'filters' | 'side'` (`dist/types/screen-shell.d.ts:19`) |
| A drawer pattern below md | `app/src/lib/components/rail-drawer.svelte.ts` (open/toggle/close/navigated), used by `RailMenuButton.svelte` |
| Sheet component | none in `@rokkit/ui` 1.15.7 (`dist/components/`) or in the app |
| Window floors | main window `minWidth: 900` (`app/src-tauri/tauri.conf.json:21`); e2e capabilities `app/src-tauri/capabilities/e2e.json` grant only `playwright:default`; normal builds load only `default.json` (`app/src-tauri/build.rs:4-10`) |

### Decisions
- **Phone is the same breakpoint the shell already uses** (below `md`, `(max-width: 767px)`). `RelayScreen`
  reads it once and hands `compact: boolean` to `toRelayShell`, so the selector stays pure and unit-testable.
- **Filters behind a toolbar button on phone.** When `compact` and the view has filters, `toRelayShell`
  returns `filters: undefined` to the shell (no band) and adds a `filters` toolbar button (kind `toggle`, icon
  the shell's own filters glyph `i-glyph:search-filter` (rokkit `ScreenShell.svelte:45`), label "Filters") before Info, with `count` = the number of active filter values
  (non-empty `repo`, `workspace` and `show` in `filterState`). The badge is rokkit's toolbar count, as Queue
  and Crew use (`relay-shell.ts:150-163`). The button opens a sheet.
- **The sheet is ours, built from the same schema.** `RelayFilterSheet.svelte` (beside `RelayShell`) draws a
  bottom sheet (`role="dialog"`, `aria-modal`, labelled "Filters") holding `FormRenderer` over the same
  `FilterSchema` and `FilterState` the band uses, and Apply / Clear. Apply routes the values through the same
  `onfilter` (`routeFilter`) and closes; Escape, the backdrop and navigation close it without applying (the
  `rail-drawer` behaviour). One schema, one router: the sheet adds no second filter model. Should rokkit gain a
  sheet, this moves to it.
- **Short stat titles on phone.** `compact` swaps the titles to "Working", "Waiting", "Blocked", "Done today"
  (only the fourth changes) so none truncate at 2-up. Desktop keeps "Completed today".
- **Desktop unchanged.** With `compact` false the toolbar, the filters band and the titles are exactly as
  today; the unit specs pin it.
- **The e2e sizes the window in test mode only.** The main window cannot be narrower than 900px, so the e2e
  build's capability (`capabilities/e2e.json`, never loaded by a normal build) gains window min-size and size
  permissions, and `app/e2e/lib/phone.ts` lifts the floor, sizes the window to 430 × 932 for the test, and
  restores both after. `update-phone-layout` task 5.1 (its 430px pass) reuses this helper.

## Decisions defaulted (user may override)
- The two phone findings ("left for the user's call") are resolved by `update-phone-layout`'s
  mobile-first ScreenShell/StatCard (info band collapsed on phone, stats 2-up, labels wrap), not by
  a Relay-specific override. #420 closes with a pointer to that change. The user's further choice
  (filters behind a button, short titles) is slice 2 above, tracked on #420's follow-up.
- Slice 2 counts the stat filter (`show`) as an active filter in the badge, since it narrows the list
  too.
- The sheet opens from the bottom and holds an explicit Apply; filters do not apply as they change.
- `lane/relay-screen-conformance` retirement stays the user's call; this change does not delete
  branches.

## Risks (slice 2)
- A desktop window at its 900px floor is never `compact`, so a person sees the phone header only on a phone
  surface (the dōjō, a narrow web view) or in `/preview`; the e2e reaches it by lifting the floor in test mode.
- Changing the window's size in e2e must be undone even when the test fails (`afterEach`), or later specs run
  in a phone-size window.
