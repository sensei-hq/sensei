# Checkpoint — 2026-10-23

**Slice:** dōjō expansion (docs/roadmap/09-dojo-expansion.md) inside the issue burndown
(docs/roadmap/08-issue-burndown.md). Released: **v0.20.0** and **v0.20.1** (2026-10-23).

**In v0.20.0:**
- R8 lead team metrics (D38, D41); R9 trackers by level (D23, D40); #323 Jira linkage F9–F11 + API cost (user).
- R5b Azure DevOps through Entra (D42–D44) and R5c sign in with Microsoft (D45). Each had two security reviews,
  and every finding was fixed with a failing test first.
- #330 Terms of Service and Privacy Policy pages. The user approved the wording.
- #328 heal defect fixed (0 of 936 live folders were stranded).
- Sign-in fixes, verified live on the local dōjō: #329 (a magic-link switch never shows the previous
  account), #331 (one personal dōjō per person, however many passes race), #297 (/signin shows Supabase's error).

**v0.20.0 rehearsal (2026-10-23): passed** on the 2026-10-21 production dump: two dōjō deploys; pass 2 changed nothing, `dbd diff` clean, row counts unchanged.

**v0.20.1** fixes v0.20.0's daemon upgrade. The schema deploy ran `apply` before `reconcile`, so `comment on column trackers.level` failed. Verified live: the daemon logged "schema deployed for this version 0.20.1", and `dbd diff` reports the live DB in sync.

**Kavach #54 and #55 fixed** on jerrythomas/kavach `develop` (1644470, 3654e64): sync once per session change, and the landing guard built into kavach (`pendingLanding`, on by default). Verified live on the dōjō with the dōjō's own guard removed: one session POST, and the page hidden until the new session's reload. The dōjō upgrade waits in worktree `kavach-landing` until kavach is published.

**On develop since v0.20.1:**
- #294: each daemon sign-in in flight keeps its own verifier, and the dōjō forwards a flow id. Live check after the release: two back-to-back `sensei auth renew` runs.
- #304: app writes return `ApiResult`, and post/put/del are gone. Stays open until a click-through in the Tauri app.
- **#334 closed** (91e2eb52): `caller_guard` covers the whole router (Host must be loopback; a write needs the app's Origin or none), and CORS answers only the app's origins. Verified live on a throwaway daemon (`--instance probe334`, since deleted).

**#298 fixed on branch `fix/298-stored-pairing-keys`** (live: 6–8.4s → 0.58s, output identical). The live DB is converged with the fixed dbd. It is blocked on **dbd 0.27.6**: dbd#104 (a new generated column was added plain), fixed on dbd develop c506dfb0. Then bump the 4 dbd pins, merge the branch, and run `deploy_upgrade --ignored`. dbd#105 is filed (promoting a column a view reads).

**In progress: #106.** The ratchet guard is in; your constitution and Plan & billing are done and live-checked. Next: the org constitution, scope owners and triage placeholder (`org/[slug]/[section]`), then the org workspace preview, to `…/rules`, `…/rules/resolved` and `…/workspaces`. Each removes its line from `STILL_ON_FIXTURES`.

**Next command:** the user says go on dbd 0.27.6; approves `deploy-dojo` (both v0.20.0 and v0.20.1 runs are waiting) and says go on a kavach release (minor: 1.3.0). Then: check the Worker's `/version`; count #331 production duplicates; bump the dōjō to the new kavach and drop `app.html`'s guard.

**Live checks after v0.20.0:** R5b connect and import; R5c oid/tid match; the negative `xms_edov` test, which blocks enabling Microsoft sign-in in production. These need the Worker secrets (`AZURE_DEVOPS_CLIENT_ID`, `AZURE_DEVOPS_CLIENT_SECRET`, `DOJO_CRON_SECRET`), the redirect URI, the `vso.*` permissions and the `email` and `xms_edov` claims. R9: two Jira boards at two levels.

**Remaining:**
- #314 (production push: the user's steps); #312 live on an unattended run; dbd#103 (a column comment lands
  one reconcile late). #331: after v0.20.0, count production's `personal/*-2` tenants (counts only); merging
  them is the user's call.
- Wave 11: the website from the user's mocks. Known: qlty is out of billing minutes; a daemon reinstall needs keychain approval (#283 until #282).
