# Change: Verify and close Relay on the D87 shell (#420)

## Why
#420 put Relay on the D87 shell: the scroll fix in `ShellScreen`, the toolbar of icon views with
badges, the stats band, schema filters, and the info band in every state. The work merged to
develop in #452 (`231cbc660`) and every issue it closes (#380, #383, #384, #385, #388, #394) and
supersedes (#386) is already closed. #420 itself is still open. Its last comment leaves two phone
findings for the user: the header bands take about 1100px on a phone, and the fourth stat title
truncates ("Completed to…"). Both are owned by `update-phone-layout` (proposal, "rokkit
ScreenShell / StatCard"; tasks 2.1, 2.2 done, 5.1 open). This change only re-proves the shipped
behaviour on current develop and closes #420.

## What Changes
Two slices. Slice 1 is the verification already being built; slice 2 is a follow-up the user chose on
2026-11-18 and is built after slice 1 closes.

### Slice 1 — verify and close (in progress)
- No production code. Re-run the existing proofs on current develop: the shell unit specs and the
  Tauri e2e `app/e2e/tests/relay-conformance.spec.ts` (error and loaded in both windows; no crew in the
  workspace window only).
- Record the evidence on #420 and close it, pointing the phone findings at `update-phone-layout`.
- If a proof fails on current develop, the regression is fixed red-first in this change before
  closing (see tasks 2.x).

### Slice 2 — phone header: filters behind a button, short stat titles (follow-up)
- The phone base layout already shipped through `update-phone-layout` (info band collapsed on phone, stats
  2-up, labels wrap). The user's remaining choice for #420's phone findings (2026-11-18): on phone widths the
  schema filters band moves behind a toolbar filter button whose badge counts the active filters, and which
  opens a sheet; the stat titles use short labels ("Done today") so none truncate; the body starts near the
  top (about 300px). Desktop is unchanged.
- Proved by unit specs for the shell selector and a Tauri Playwright e2e at a phone-size window.

## Impact
- Specs: `relay-shell` (slice 1: requirements stated as they now hold; slice 2: one new requirement, the
  compact phone header).
- Code: slice 1 none expected. Slice 2: `app/src/lib/relay/relay-shell.ts`, `RelayShell.svelte`,
  `RelayScreen.svelte`, a new `RelayFilterSheet.svelte`, `app/src-tauri/capabilities/e2e.json` (e2e builds
  only), `app/e2e/lib/phone.ts` and `app/e2e/tests/relay-phone.spec.ts` (new).
- Related: `update-phone-layout` (phone header height, stat label wrap), `fix-relay-read-merge`,
  `add-relay-run-grouped-lanes` (Now view rows, out of scope here).
