# Checkpoint — 2026-11-19 (#299 — verify sync-filed dōjō workspaces)

**Slice:** `verify-dojo-sync-files-workspaces` slice 1/1 (#299). Branch `lane/relay-299-s1`
(worktree `lanes/relay-open`). No production code: the change gathers evidence.

**Done (tasks 1.1–1.4, 2.1–2.7):**
- Local dōjō DB converged (`scripts/deploy-dojo-db.sh`, diff clean).
- SQL `make test-db` 101/101 (filing, workspaces read/write ok). dōjō vitest 55/55. senseid
  `offered_workspace_tests` and `a_failed_workspace_filing_is_partial_and_names_the_error` pass.
- Walkthrough: scratch daemon (`--instance`, `DOJO_URL` at wrangler dev) ran two `dojo_sync` cycles.
  One workspace filed, one link, same id after cycle 2. Playwright: member sees it on
  `/org/<tenant>/workspaces`; a `lead` with no repository role does not. A repository whose org has no
  connection was reported `no_connection` and filed nothing.
- Evidence posted on #299 (issuecomment-6995483210). Scratch daemon, DB, credential and wrangler torn
  down; installed daemon on :7744 untouched.

**Remaining:** tasks 3.1–3.5, the production runbook. User only (needs `sensei auth approve` and the
production dōjō).

**Next command:** the user runs `sensei auth approve`, then follows tasks.md section 3.

**Open questions:** none.

**Known broken:** none from this branch. Note: a lane test DB created before a DDL merge goes stale
(`relay_now.sql` failed on it); drop `sensei_test_<lane>` and `make test-db` recreates it.
