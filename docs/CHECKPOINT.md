# Checkpoint — 2026-11-26 (#541 fix 2 done)

**Slice:** #541 merged-workspace governance fix (4 parts).
**Branch:** `fix/541-merged-workspaces` (pushed).

## Done
- Fix 1: MCP `get_rules` sends `folder=<cwd match>`.
- Fix 2: `resolve_folder` 409s on multi-repo workspaces
  (top-level repos + candidates; same-repo checkouts still resolve).

## Remaining
- Fix 3: scan mints distinct workspace for non-nested same-name repos.
- Fix 4: heal splits merged workspaces; index doctor reports zero.
- Live-data verify + full suite + merge to develop.

## Next command
`cargo test -p senseid --bin senseid -- resolve_workspace_` (fix 3 red).

## Open questions
- None.

## Known-broken
- None.
