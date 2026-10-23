# Checkpoint — 2026-10-22

**Slice:** dōjō expansion (docs/roadmap/09-dojo-expansion.md) inside the issue burndown
(docs/roadmap/08-issue-burndown.md). Released: v0.19.0 (Worker deployed from the tag).

**Done since v0.19.0 (on develop, ships in the next release):**
- **R8 (D38, D41)** merged as 21a5d1a3. The lead sees per-person rows for the teams they are on; the admin sees
  every team; contributors and maintainers see team totals only when 3 others contributed; a viewer gets 403.
  Walked through in the browser on the local dōjō for all five roles; the fixture was removed afterwards.
- **R9 (D23, D40)**: several tracker boards per workspace, each at a level (`epic | sprint | task`).
  - Old rows become sprint boards; the upgrade was rehearsed on sensei_test (864 rows, 2 passes).
  - One credential per site; per-board routes; `sensei tracker --level/--board`.
  - The app's Integrations screen groups boards by level; the Sprints screen shows lanes side by side.
  - An independent review found 6 defects (a drain race that could orphan a secret, `--board` ambiguous
    across sites, lookup stopping at a busy board, a re-key read as "gone", stale sibling errors, ticket
    detail flipping between boards). All 6 are fixed, each with a red test first.

- **#328 fixed:** the CI flake was a real heal defect. A nested standalone re-absorbed before its workspace landed
  kept its own repository anchor and stayed in a phantom workspace for good. The heal now moves the anchor.
  Live: 0 of 936 folders were stranded.

**Remaining:**
- R9 live check on a real Jira board (two boards, two levels): the user's. The local daemon has no tracker.
- **R5b merged** (18f9311c): Azure DevOps import through Microsoft Entra (D42–D44), refresh token in Vault, scheduled member read (cron in `dojo/worker-entry.js`, needs Worker secrets `AZURE_DEVOPS_CLIENT_ID`, `AZURE_DEVOPS_CLIENT_SECRET`, `DOJO_CRON_SECRET` of 32+ chars). Two security reviews, all findings fixed. Gates on the merge with #323: check 0, 1911 tests, test-db 58, dbd diff clean.
- **R5c merged:** sign in with Microsoft (D45). It records the Entra oid (lowercased, tenant-pinned) only when `xms_edov` confirms the email, and provisions from member sets no older than 48 h. Gates on the merge: check 0, 1976 tests, test-db 59, dbd diff clean after 3 passes. **Release blocker for Microsoft sign-in in production:** the app sends `xms_edov` (ID and access token), and the negative live test passes (a test-tenant user claiming an existing login's email must not land in it). **#330 merged**; the user approved the wording; still to give: a contact for export and deletion requests.
- **Entra app registration: done by the user (2026-10-23)**, linked to Supabase Auth with the Azure provider enabled. Still to confirm on it: the connect redirect URI `/v1/integrations/azure_devops/callback`, the Azure DevOps `vso.*` permissions, the `email` and `xms_edov` claims, and the Worker secrets `AZURE_DEVOPS_CLIENT_ID`, `AZURE_DEVOPS_CLIENT_SECRET` and `DOJO_CRON_SECRET`. Live checks after R5c and the release: the R5b connect and import, and the R5c object-id match.
- #314 (production push: the user's steps); #312 live on an unattended run; #329 stale session
  after a magic-link switch; dbd#103 (a column comment lands one reconcile late).
- Wave 11 (website from the user's mocks), after the dōjō waves.

**v0.20.0 rehearsal (2026-10-23): passed.** On a copy of production (the 2026-10-21 dump, brought to v0.19.0 by that tag's own reconcile, deploy and diff), develop's dōjō deploy ran twice:
- pass 1 created 4 objects, altered 4 and ran 15 hooks; pass 2 changed nothing; `dbd diff` was clean after each;
- every row count was unchanged (tenants 6, memberships 5, principals 2, identities 2, connections 3);
- provenance: 3 org memberships attributed to GitHub, 2 personal memberships left unattributed;
- `invite_accept_failures` was dropped by its hook.
The copy was dropped afterwards.

**Next command:** `make bump v=minor` on develop → PR to main → `make release` on main → the user approves
deploy-dojo. Or, before releasing, start R5b per docs/spec/dojo/git-integration-import.md.

**Open questions:** none blocking. Carried: #289, #149, #133, #282, #194.

**Known-broken / pending:**
- qlty is out of billing minutes, and the Cloudflare "Workers Builds: dojo" preview check fails
  (the user is removing the git connection).
- Keychain approval is needed after every daemon reinstall (#283 until #282).
