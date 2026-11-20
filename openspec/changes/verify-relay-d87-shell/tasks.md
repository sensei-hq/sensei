# Slice 1 — verify and close (done)

## 1. Re-prove on current develop
- [x] 1.1 `cd app && bunx vitest run src/lib/relay/relay-shell.spec.ts src/lib/relay/relay-shell-input.spec.ts src/lib/components/ShellScreen.spec.svelte.ts` — green
- [x] 1.2 Check `pgrep -f "instance e2e"` is empty, then run the Tauri e2e: `make test-app-e2e` (or, with the bundle built, `cd app && bunx playwright test --config e2e/playwright.config.ts --project=tauri e2e/tests/relay-conformance.spec.ts`) — 3 of 3 green
- [x] 1.3 Also run `relay.spec.ts`, `relay-run.spec.ts`, `relay-timeline.spec.ts`, `zz-a11y.spec.ts` — green (no serious axe finding on the shell sidebar)
- [x] 1.4 Confirm `brew services start sensei` restored the real daemon on :7744

## 2. Only if 1.x fails
- [x] 2.1 (not needed: every 1.x proof passed on develop `f880251b8`) Commit a failing test that isolates the regression (red)
- [x] 2.2 (not needed) Fix it in a separate commit (green); re-run 1.1–1.3

## 3. Close
- [x] 3.1 Comment on #420 with the run results and evidence path `app/test-results/relay-conformance/`; name `update-phone-layout` as owner of the phone header height and stat label wrap
- [x] 3.2 Close #420
- [x] 3.3 `openspec validate verify-relay-d87-shell --strict`

# Slice 2 — compact phone header (follow-up; starts after slice 1 closes)

Red commit first for each numbered group; green in the next commit.

## 4. Selector (red first)
- [ ] 4.1 Red: `app/src/lib/relay/relay-shell.spec.ts` — with `compact: true` on a loaded Now view in the main
  window: `filters` is undefined, the toolbar has a `filters` toggle before Info whose `count` is the number of
  active filter values (0 with none; 2 with a repository and a stat filter set), and the stat titles read
  "Working", "Waiting", "Blocked", "Done today". With `compact: false`: the toolbar, `filters` and the titles
  equal today's (no `filters` button, "Completed today"). In the workspace window (no filters), `compact` adds
  no button.
- [ ] 4.2 Green: `RelayShellInput.compact` (`relay-shell.ts:21`), the toolbar (`:165-190`), the stat titles
  (`:226-229`), `filters` (`:282-292`, `:336-339`); the button's press is a new `actions.filterSheet()`.
  `relay-shell-input.ts` and `relay-shell-input.spec.ts` carry `compact`.

## 5. Sheet (red first)
- [ ] 5.1 Red: `RelayFilterSheet.spec.svelte.ts` + harness — closed draws nothing; open draws a dialog labelled
  "Filters" with one control per schema field and the current values; Apply calls `onfilter` once with the
  edited values and closes; Clear empties the band values (keeps none) and applies; Escape and the backdrop close
  without calling `onfilter`; focus moves into the sheet on open and back to the Filters button on close.
- [ ] 5.2 Green: `app/src/lib/relay/RelayFilterSheet.svelte` (state in a `.svelte.ts` beside it, like
  `app/src/lib/components/rail-drawer.svelte.ts`); `RelayScreen.svelte` reads the phone media query once
  (the breakpoint `ShellScreen.svelte:65` uses), passes `compact` to `toRelayShell` (`RelayScreen.svelte:86`),
  and mounts the sheet; navigation closes it.
- [ ] 5.3 `/preview` fixtures in `app/src/lib/relay/catalogue.ts`: `RelayShell/phone` (filters button with a
  badge of 2) and `RelayFilterSheet/open`; capture at phone width in light and dark.

## 6. Phone e2e (Tauri Playwright, red first)
- [ ] 6.1 Test-mode window sizing: add window min-size and size permissions for `main` to
  `app/src-tauri/capabilities/e2e.json` (e2e builds only; normal builds load `default.json`,
  `app/src-tauri/build.rs:4-10`), and `app/e2e/lib/phone.ts` with `toPhone(tauriPage)` (lift the 900px floor,
  `tauri.conf.json:21`; size to 430 × 932) and `restore(tauriPage)` (size and floor back), called from
  `afterEach` even on failure. Shared with `update-phone-layout` task 5.1.
- [ ] 6.2 `app/e2e/tests/relay-phone.spec.ts` "the body starts near the top": with a crew minted on the e2e
  broker (the `RelayCrew` driver, `app/e2e/lib/relay-crew.ts:84`) and the main window at phone size, Relay's
  `[data-screen-body]` top is at most 320px from the window top, no stat title is truncated
  (`scrollWidth <= clientWidth` on each title), and the fourth reads "Done today".
- [ ] 6.3 Same spec "the filter sheet opens and applies": the toolbar shows Filters with no badge; pressing it
  opens the sheet; picking a repository and Apply closes it, narrows the Now view (the URL carries the repo) and
  the badge reads 1; reopening shows the repository selected; Clear removes it and the badge goes.
- [ ] 6.4 Same spec "desktop is unchanged": restored to 1280 × 820, the filters band shows, no Filters button,
  the fourth stat reads "Completed today".
- [ ] 6.5 `relay-conformance.spec.ts`, `relay.spec.ts`, `relay-run.spec.ts` and `zz-a11y.spec.ts` stay green
  (the sheet adds no serious axe finding). Check `pgrep -f "instance e2e"` is empty before the run; confirm the
  brew daemon is back on :7744 after. Evidence DOM dumps under `app/test-results/relay-phone/`.

## 7. Close slice 2
- [ ] 7.1 Gates: `bun run check`, `bun run test` and lint in `app/`; `openspec validate verify-relay-d87-shell
  --strict`
- [ ] 7.2 Slice 1 closed #420: open a follow-up issue for slice 2 first (it references #420), post the phone e2e
  evidence there, and close it on merge to develop
