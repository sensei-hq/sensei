# Checkpoint — 2026-10-21

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md) — the security and production gaps (#309–#321, roadmap/06-gaps.md).

**Done:**
- **v0.16.0 released.** Tag 16869d1f; PR #322 merged as e62764fb. The production dōjō reconcile was approved by the user
  and went exactly as rehearsed, and `dbd diff` is in sync. The Worker serves 0.16.0 (gitSha e62764f). #315 closed.
  The scratch prod copy is dropped; the user's dump file is kept.
- #312 built (4fcea9de, D28): an unanswered hard-block in an unattended run is denied. Live proof comes with the next release.
- Placement floor (nightly, red since 10-19): @sensei/glyphs fixed (5981faab, tsconfig — ES5 had no Set).

**Remaining:**
- **Placement floor: @sensei/marketplace** (plugin hooks at 37% / 26%). Its `$` comes from the engine's `claude-code` types,
  which the repo does not carry. Waits on the user's decision (see Open questions).
- #313: audit rows for viewer grant/revoke/remove/team-grant (inside the SECURITY DEFINER functions, into
  dojo.audit_events) + SQL tests pinning that a viewer reads no person-keyed row. Export / first open come with the viewer shell.
- #314 (user steps: enrol the daemon with production, prod URL env, VAPID + Worker secrets, phone test).
- #307 R2b, R3; #318 rest (D25/D29 stacks applied at a level, D23 trackers).

**Next command:** #313 — write database/tests/dojo/viewer_audit.sql (red), then the audit inserts.

**Open questions:**
- Marketplace plugin types: vendor the engine's `claude-code` declarations into the plugin (recommended), hand-write the
  subset it uses, or exclude the package from the floor (documented in the backlog)?
- Client scope as a governance level: retire it (recommended)?
- Carried: #289, #149, #133, #282, #194.

**Known-broken / pending:**
- Nightly placement floor red until the marketplace question is settled. qlty is out of billing minutes, and the
  Cloudflare "Workers Builds: dojo" preview check fails; the production deploy works.
- Keychain approval needed after every daemon reinstall (#283 until #282).
