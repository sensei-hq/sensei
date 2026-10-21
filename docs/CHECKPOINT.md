# Checkpoint — 2026-10-21

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md) — the security and production gaps (#309–#321, roadmap/06-gaps.md).

**Done:**
- **v0.16.0 released.** Tag 16869d1f; PR #322 merged as e62764fb. The production dōjō reconcile was approved by the user
  and went exactly as rehearsed, and `dbd diff` is in sync. The Worker serves 0.16.0 (gitSha e62764f). #315 closed.
  The scratch prod copy is dropped; the user's dump file is kept.
- #312 built (4fcea9de, D28): an unanswered hard-block in an unattended run is denied. Live proof comes with the next release.
- Placement floor (nightly, red since 10-19): @sensei/glyphs (5981faab, tsconfig: ES5 had no Set) and
  @sensei/marketplace (356ce9a0, vendored engine `claude-code` types — user decision; `make plugin-types src=…`
  refreshes them). CI re-run on develop green (run 39676172084).
- #307 R2b done (f080e38a, e6c58885): the dōjō's pages say workspace; fixed the Viewers tab's R2a drift (shipped in
  v0.16.0, no prod viewers yet — needs the next release). Verified live on the local dōjō.
- #313 closed (6dad1e33): grant changes audited in-transaction; a viewer reads no one else (SQL-pinned). Verified live
  on the local dōjō through the API (2 audit rows for 4 calls).

**Remaining:**
- #314 (user steps: enrol the daemon with production, prod URL env, VAPID + Worker secrets, phone test).
- #307 R2b, R3; #318 rest (D25/D29 stacks applied at a level, D23 trackers).
- Wave 11 (after the dōjō waves): the website from the user's new mocks; coming soon → available as each ships.

**Next command:** R3a in progress — an agent works on a branch in a worktree (DDL + in-place rename hook + SQL tests +
the daemon's SQL; HTTP API unchanged until R3b). The rehearsal copy is `ztest_r3a` (pg_dump of the live 25 GB DB:
`~/sensei-r3a-dump`; log /tmp/r3a-copy.log). On its report: review the diff, re-run the gates and the rehearsal (twice,
counts only), then R3b (daemon API + app) on the same branch; merge to develop when R3 is whole.
**R3a inventory (daemon DB catalogue, 2026-10-21):** tables sensei.projects, repositories_in_projects, service_projects,
extension_projects; 19 `project_id`-style columns (activity, history, inference, sensei) + staging.memories.project_name;
17 views (project_metric_*, project_*); fn sole_project_of; enums project_maturity + 'project' in entity_scope,
memory_scope, sync_entity; task kinds analyze_project / compute_project_metrics / compile_projects. Keep: dbd.* (dbd's
word); extension_scope 'project' if it is Claude Code's scope (check).

**Open questions:**
- Client scope as a governance level: retire it (recommended)?
- Carried: #289, #149, #133, #282, #194.

**Known-broken / pending:**
- qlty is out of billing minutes, and the
  Cloudflare "Workers Builds: dojo" preview check fails; the production deploy works.
- Keychain approval needed after every daemon reinstall (#283 until #282).
