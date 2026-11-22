# Checkpoint — 2026-11-22 (v0.26.0 released; large-root perf pass done)

**Slice:** release v0.26.0 + perf recon on ~/Developer + ~/Work.
**Branch:** `develop` @ `c46b052ee` (pushed). Tag `v0.26.0` on main.

## Done
- v0.26.0: PR #519 merged, main CI green, tag pushed, release + tap
  jobs success, marketplace synced, 0.26.0 installed + verified live.
  Only deploy-dojo waits (manual approval gate).
- Perf: ~/Work added (60 repos); Playwright tracked 5 rounds + shots;
  metrics captured (durations, failures, coverage, load).
- Findings: ts-driver pile-up (5× no-tsconfig compiles, load 31);
  Rust p50 14s; C++ gap 86k files; stale-folder alert flood; socket
  saturation (app survived, test channel didn't).

## Remaining
- User: approve deploy-dojo in GitHub (release waits on it).
- Perf follow-ups (not started): compile cap/dedup, stale prune,
  C++ adapter, phase timing. Scan still draining (~8h ETA).

## Next command
None — read the report, then pick a follow-up.

## Open questions
- None.

## Known-broken
- None on the release; perf gaps listed above.
