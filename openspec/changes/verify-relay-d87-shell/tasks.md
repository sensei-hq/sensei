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
