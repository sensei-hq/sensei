# Checkpoint — 2026-11-26 (#541 all 4 fixes done, verifying)

**Slice:** #541 merged-workspace governance fix (4 parts).
**Branch:** `fix/541-merged-workspaces` (pushed, 79a1e137f).

## Done
- Fix 1: MCP `get_rules` sends `folder=<cwd match>`.
- Fix 2: `resolve_folder` 409s on multi-repo workspaces.
- Fix 3: scan mints distinct workspaces for non-nested same-name repos.
- Fix 4: heal splits merged workspaces; index doctor Class 8.
- Live: old MCP→another repo's rules ✗, new MCP→caller's repo ✓.
- Live detect: 3 merged (website, database + 1 client repo);
  other 5 same-name pairs share one repo (Class 5, not merged).

## Remaining
- Full suite green → merge to develop → push.

## Next command
`cargo test --workspace` (background), then merge.

## Open questions
- None.

## Known-broken
- None.
