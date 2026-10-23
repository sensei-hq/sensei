# Checkpoint — 2026-10-23

**Slice:** dōjō expansion (docs/roadmap/09-dojo-expansion.md) inside the issue burndown
(docs/roadmap/08-issue-burndown.md). Released: v0.19.0. Next: **v0.20.0**.

**On develop, ships in v0.20.0:**
- R8 lead team metrics (D38, D41); R9 trackers by level (D23, D40); #323 Jira linkage F9–F11 + API cost (user).
- R5b Azure DevOps through Entra (D42–D44) and R5c sign in with Microsoft (D45). Each had two security reviews,
  and every finding was fixed with a failing test first.
- #330 Terms of Service and Privacy Policy pages. The user approved the wording.
- #328 heal defect fixed (0 of 936 live folders were stranded).
- Sign-in fixes, verified live on the local dōjō: #329 (a magic-link switch never shows the previous
  account), #331 (one personal dōjō per person, however many passes race), #297 (/signin shows Supabase's error).

**v0.20.0 rehearsal (2026-10-23): passed.**
- The copy: the 2026-10-21 production dump, brought to v0.19.0 by that tag's reconcile, deploy and diff.
- Develop's dōjō deploy ran twice. Pass 2 changed nothing, and `dbd diff` was clean after each pass.
- Row counts were unchanged: tenants 6, memberships 5, connections 3.
- Provenance: 3 org memberships attributed to GitHub; 2 personal ones left unattributed.

**Next command:** once the user says go: `make bump v=minor` on develop → PR to main → `make release` on
main → the user approves deploy-dojo.

**Open questions (the user's):**
- When to release.
- A contact for export or deletion requests on the privacy page (#330), now or in a follow-up.

**Live checks after v0.20.0:**
- R5b connect and import; R5c oid/tid match.
- The negative `xms_edov` test, which blocks enabling Microsoft sign-in in production.
- These need the Worker secrets `AZURE_DEVOPS_CLIENT_ID`, `AZURE_DEVOPS_CLIENT_SECRET` and `DOJO_CRON_SECRET`,
  the connect redirect URI, the `vso.*` permissions, and the `email` and `xms_edov` claims.
- R9: two Jira boards at two levels.

**Remaining:**
- #314 (production push: the user's steps); #312 live on an unattended run; dbd#103 (a column comment lands
  one reconcile late). #331: after v0.20.0, count production's `personal/*-2` tenants (counts only); merging
  them is the user's call.
- Wave 11: the website from the user's mocks.
- Known: qlty is out of billing minutes; a daemon reinstall needs keychain approval (#283 until #282).
