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
- No production code. Re-run the existing proofs on current develop: the shell unit specs and the
  Tauri e2e `app/e2e/tests/relay-conformance.spec.ts` (error and loaded in both windows; no crew in the
  workspace window only).
- Record the evidence on #420 and close it, pointing the phone findings at `update-phone-layout`.
- If a proof fails on current develop, the regression is fixed red-first in this change before
  closing (see tasks 2.x).

## Impact
- Specs: `relay-shell` (requirements stated as they now hold; verification only).
- Code: none expected.
- Related: `update-phone-layout` (phone header height, stat label wrap), `fix-relay-read-merge`,
  `add-relay-run-grouped-lanes` (Now view rows, out of scope here).
