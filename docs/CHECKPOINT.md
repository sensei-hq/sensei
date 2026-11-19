# Checkpoint — 2026-11-19 (#299 — verify sync-filed dōjō workspaces)

**Slice:** `verify-dojo-sync-files-workspaces` slice 1/1 (#299). Branch `lane/relay-299-s1`
(worktree `lanes/relay-open`), PR #503.

**Done (tasks 1.1–1.4, 2.1–2.7) + #503 review fix:**
- SQL `make test-db` 101/101; dōjō vitest 237 files / 2411 tests; senseid filing tests pass.
- Walkthrough: one workspace filed by the sync, one link, same id after cycle 2; unconnected org files nothing.
- Review fix: step 2.3 re-run as a NON-admin contributor with a role on the filed repository. It got 403
  "lead role required": `GET /v1/t/{origin}/{org}/workspaces` gated at `lead`. Red `110729d0e`, fix
  `87d9a1693` (floor = member; the RLS policy decides rows). Re-walked: the contributor sees 1 row, the filed
  workspace; the lead with no role on it still does not.
- `repository_workspace_filing.sql` now asserts a non-admin with a repository role reads the filed workspace
  and a lead without one does not (`b34ec28a3`; fails under a mutant `can_read_workspace` with no repository
  branch, where the old file passed).

**Remaining:** tasks 3.1–3.5, the production runbook (user only). The member floor reaches production with
the next release.

**Next command:** the user runs `sensei auth approve`, then follows tasks.md section 3.

**Open questions:** none.

**Known broken:** none from this branch. A lane test DB created before a DDL merge goes stale; drop
`sensei_test_<lane>` and `make test-db` recreates it.
