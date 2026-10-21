# Checkpoint — 2026-10-21

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md) — the security and production gaps (#309–#321, roadmap/06-gaps.md).

**Done:**
- **v0.16.0 released.** Tag 16869d1f; PR #322 merged as e62764fb. The production dōjō reconcile was approved by the user
  and went exactly as rehearsed, and `dbd diff` is in sync. The Worker serves 0.16.0 (gitSha e62764f). #315 closed.
  The scratch prod copy is dropped; the user's dump file is kept.
- #312 built (4fcea9de, D28): an unanswered hard-block in an unattended run is denied. Live proof comes with the next release.
- Placement floor (nightly, red since 10-19): @sensei/glyphs (5981faab, tsconfig: ES5 had no Set) and
  @sensei/marketplace (356ce9a0, vendored engine `claude-code` types — user decision; `make plugin-types src=…`
  refreshes them). Local floor passes; CI re-run dispatched on develop (run 39676172084).
- #313 closed (6dad1e33): grant changes audited in-transaction; a viewer reads no one else (SQL-pinned). Verified live
  on the local dōjō through the API (2 audit rows for 4 calls).

**Remaining:**
- Confirm the placement floor's CI re-run is green.
- #314 (user steps: enrol the daemon with production, prod URL env, VAPID + Worker secrets, phone test).
- #307 R2b, R3; #318 rest (D25/D29 stacks applied at a level, D23 trackers).
- Wave 11 (after the dōjō waves): the website from the user's new mocks; coming soon → available as each ships.

**Next command:** check run 39676172084; then #307 R2b (page URLs and words) — read roadmap/09 R2b first.

**Open questions:**
- Client scope as a governance level: retire it (recommended)?
- Carried: #289, #149, #133, #282, #194.

**Known-broken / pending:**
- qlty is out of billing minutes, and the
  Cloudflare "Workers Builds: dojo" preview check fails; the production deploy works.
- Keychain approval needed after every daemon reinstall (#283 until #282).
