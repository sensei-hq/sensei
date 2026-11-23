# Checkpoint — 2026-11-23 (#444 landed; epics report next)

**Slice:** relay + diagrams open issues.
**Branch:** `develop` (4 #444 commits citing §10.1–10.3).

## Done
- #312 (verified per D28 — ready to close), #433 (not repro —
  ready to close), #513, #512, #393 (all landed).
- #444: janitor (§10.2) → sentinel (§10.1) → serial-build
  guard + `POST /api/relay/resources/execute` (§10.3, waits
  behind the release, records props, frees the slot).
  Relay suites 150 + 186 green; clippy/fmt clean.

## Remaining
- Push the 4 #444 commits.
- Close #312/#433 on user confirmation.
- Report on epics/blocked (#427 #428 #314 #299 #268 #246).

## Next command
`git push origin develop`, then write the epics/blocked report.

## Open questions
- 6 process.rs fmt spots: local toolchains flag what CI stable
  accepts. Left untouched.

## Known-broken
- None.
