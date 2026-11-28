# Checkpoint — 2026-11-27 (v0.28.2 released; public downloads verified)

**Slice:** #538 public install + daemon serve-while-booting + #543 janitor.
**Branch:** `develop` == `main` @ v0.28.2 (`bed44f5c6`).

## Done
- #539 merged (develop brought in; CI red was `relay_host_disk_exhausted`
  on the runner, fixed by develop's relay-health seam).
- v0.28.2 tagged; release run 46097239465: builds, `release` (to public
  `sensei-hq/sensei-releases`) and `update-tap` green.
- Verified logged out: website Download → sensei-releases v0.28.2; DMG and
  macOS tarball download 200; checksums equal the tap's; DMG verifies;
  `sensei --version` = 0.28.2; tokenless `brew fetch` of formula + cask OK.

## Remain
1. `deploy-dojo` awaits the owner's approval in Actions.
2. After it completes: `gh repo edit sensei-hq/sensei --visibility private --accept-visibility-change-consequences`.
3. Not yet verified: first run on a clean user creates the DB from the
   embedded schema with no network.

## Next command
`gh run view 46097239465` (after approval), then the visibility flip.

## Open questions
- Follow-ups #544 (version rescan never commits), #545 (slow folder delete).
