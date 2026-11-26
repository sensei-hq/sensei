# Checkpoint — 2026-11-26 (#541 fix 1 done)

**Slice:** #541 merged-workspace governance fix (4 parts).
**Branch:** `fix/541-merged-workspaces` (pushed).

## Done
- Fix 1: MCP `get_rules` sends `folder=<cwd match>`;
  name only without a match (db5936808 red, 1b49e51a9 green).

## Remaining
- Fix 2: daemon `resolve_folder` 409 on multi-repo workspace.
- Fix 3: scan mints distinct workspace for non-nested same-name repos.
- Fix 4: heal splits merged workspaces; index doctor reports zero.
- Live-data verify + full suite + merge to develop.

## Next command
`cargo test -p senseid --bin senseid -- resolve_folder_ambiguous` (fix 2 red).

## Open questions
- None (index doctor is the "doctor" surface for fix 4).

## Known-broken
- None.
