# Checkpoint — 2026-10-21

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md) — the security and production gaps (#309–#321, roadmap/06-gaps.md).

**Done:**
- #309 (ede5da58), #311 (f607bf0a), #318 part 1 (5da293d5), #307 R2a (65eede4d), D28/D29 (0dc6d66e).
- **#315 release rehearsal** on a restored prod copy (scratch DB `ztest_prodcopy`, from the user's dump). It found a real
  blocker: the rename hook assumed `project_viewers` exists, but prod never had it. Fixed in 55b5753d. Two passes of
  reconcile --allow-destructive → deploy → diff --exit-code are green. Counts held: 6 tenants, 5 memberships, 0 invites.

**Remaining:**
- **Release develop → main** (runs the Release Checklist in ~/.claude/CLAUDE.md) — waits on the user's go.
- After the release: #314 (enrol the daemon with production, read-only prod URL via env, VAPID + Worker secrets, phone
  test); drop `ztest_prodcopy` (keep the user's dump file).
- #312 option C (D28: fail closed for hard-block categories in unattended runs only); #313 viewer aggregate-only reads +
  audit; #307 R2b (page URLs and words), R3 (sensei says workspace, D16); #318 rest (team/stack grouping D25/D29,
  multiple trackers D23).

**Next command:** on the user's go → `/release` (minor bump). Otherwise #312 option C.

**Open questions:**
- Go for the release?
- Client scope as a governance level: retire it (recommended)?
- Carried: #289 duplicate-event delete, #149 community algorithm, #133 glyph defaults, #282 Apple account, #194 row drop.

**Known-broken / pending:**
- Keychain approval needed after every daemon reinstall (#283 until #282).
- Production dōjō still on 0.15.0's schema until the release.
