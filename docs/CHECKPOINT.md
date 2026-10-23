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
- **Next: R5c** (sign in with Microsoft, D45; agent on `r5c-microsoft-signin`), then **#330** (Terms of Service and Privacy Policy pages). Then the release (R5a, R8, R9, R5b, #323), rehearsed on a production copy first: R5b's hooks relabel and backfill membership provenance.
- **The user's step:** one multi-tenant Entra app registration (checklist: spec "R5b · one-time setup"), then the live checks.
- #314 (production push: the user's steps); #312 live on an unattended run; #329 stale session
  after a magic-link switch; dbd#103 (a column comment lands one reconcile late).
- Wave 11 (website from the user's mocks), after the dōjō waves.

**Next command:** `make bump v=minor` on develop → PR to main → `make release` on main → the user approves
deploy-dojo. Or, before releasing, start R5b per docs/spec/dojo/git-integration-import.md.

**Open questions:** none blocking. Carried: #289, #149, #133, #282, #194.

**Known-broken / pending:**
- qlty is out of billing minutes, and the Cloudflare "Workers Builds: dojo" preview check fails
  (the user is removing the git connection).
- Keychain approval is needed after every daemon reinstall (#283 until #282).
