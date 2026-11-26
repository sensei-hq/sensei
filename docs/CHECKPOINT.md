# Checkpoint — 2026-11-26 (#541 deployed + verified)

**Slice:** #541 merged-workspace governance fix (4 parts).
**Branch:** `develop` (pushed). Daemon 0.28.0 live via brew.

## Done
- All 4 fixes merged; full suite green.
- Deployed: release binaries built, installed over Cellar
  (fresh inodes — truncate-in-place trips macOS CS_KILL).
- Indexer crash fixed en route (Vue multibyte slice).
- Live heal split 3 merged workspaces; doctor reads 0;
  website session followed its repo; rules resolve.

## Remaining
- Close #541 after user confirms.
- Next release tags a signed build (this deploy is adhoc-signed;
  Keychain-backed remote features may degrade until then).

## Next command
None — slice complete and live.

## Open questions
- None.

## Known-broken
- None.
