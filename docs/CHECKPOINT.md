# Checkpoint — 2026-11-26 (#541 merged to develop)

**Slice:** #541 merged-workspace governance fix (4 parts).
**Branch:** `develop` (merged b41ed6b9c, pushed).

## Done
- Fix 1: MCP `get_rules` sends `folder=<cwd match>`.
- Fix 2: `resolve_folder` 409s on multi-repo workspaces.
- Fix 3: scan mints distinct workspaces for non-nested same-name repos.
- Fix 4: heal splits merged workspaces; index doctor Class 8.
- Full suite green (5262 senseid + all crates, 0 failed).
- Live: new MCP resolves caller's repo; 3 merged found (heal on deploy).

## Remaining
- Deploy → periodic audit splits the 3; doctor reads zero.
- Close #541 after user confirms; main merge is user's call.

## Next command
None — slice complete. Deploy when ready.

## Open questions
- None.

## Known-broken
- None.
