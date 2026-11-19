# Checkpoint — 2026-11-19 (#299 — verify sync-filed dōjō workspaces)

**Slice:** `verify-dojo-sync-files-workspaces` slice 1/1 (#299). Branch `lane/relay-299-s1`
(worktree `lanes/relay-open`), PR #503.

**Done (tasks 1.1–1.4, 2.1–2.9) + #503 review fixes:**
- SQL `make test-db` 101/101; dōjō vitest 237 files / 2411 tests; senseid filing tests pass.
- Member floor on `GET /v1/t/{origin}/{org}/workspaces` (red `110729d0e`, fix `87d9a1693`).
- Committed browser spec `dojo/e2e/workspace-filing.e2e.ts` (`cd dojo && bun run test:e2e`, local Supabase
  only): two sync cycles, contributor sees the filed workspace once, lead with no repository role sees none,
  unconnected org files nothing. 5/5 green; red with the floor put back to `lead`.
- Live re-run after the fix: scratch `senseid --instance` against the local dōjō, 7 cycles `mapped=1
  unmapped=1`, no "could not file" warning, 1 workspace (same id) and 1 link throughout; browser: contributor
  1 row, lead 0 rows.

**Remaining:** tasks 3.1–3.5, the production runbook (user only). The member floor reaches production with
the next release.

**Next command:** the user runs `sensei auth approve`, then follows tasks.md section 3.

**Open questions:** none.

**Known broken:** none from this branch. `bun run test:e2e` is not in CI (needs a local Supabase). A scratch
daemon reads a `security`-written keychain item only with `SENSEI_CREDENTIAL_BACKEND=security`.
