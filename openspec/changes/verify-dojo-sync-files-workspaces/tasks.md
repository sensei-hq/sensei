# Tasks: verify-dojo-sync-files-workspaces

No production code changes. The fix is on `develop` (see proposal.md). These tasks gather the evidence.

## 1. Re-run the existing tests on develop

- [ ] 1.1 Converge the local dōjō database: `scripts/deploy-dojo-db.sh`.
- [ ] 1.2 SQL: run `database/tests/dojo/repository_workspace_filing.sql`, `workspaces_read.sql` and
      `workspaces_write.sql` (via `make test-db`; `make test-db-if-reachable` prints SKIPPED and exits 0
      when the database is unreachable, which is not evidence). Expect all to pass: converging filings, a
      move keeps one link, personal-dōjō filing stays personal, a non-member is refused, the link table
      refuses direct writes.
- [ ] 1.3 dōjō unit: `cd dojo && bun run test -- src/lib/server/repositories.spec.ts
      src/routes/v1/you/workspaces`. Expect the filing cases (files, admin mapping wins, no workspace, did not
      map, failure reported) to pass.
- [ ] 1.4 Daemon: `cargo test -p senseid offered_workspace_tests -- --test-threads=1` and
      `cargo test -p senseid a_failed_workspace_filing_is_partial_and_names_the_error -- --test-threads=1`.

## 2. Local dōjō walkthrough (Playwright, local Supabase only)

- [ ] 2.1 Start the local dōjō (build + wrangler dev) against the converged local Supabase; sign in by magic
      link as the local test user who is a member of an org dōjō connected to a GitHub org.
- [ ] 2.2 Point a scratch daemon (`--instance`) at the local dōjō (`DOJO_URL`) and let one `dojo_sync` cycle
      run, with a repository in exactly one sensei workspace that has a `workspace` namespace. The daemon does
      not parse or log the returned `workspace_id`; it logs only a failure (warn "the dōjō could not file this
      repository under its workspace"). So check the log has no such warning for the repository, and query the
      local dōjō DB: the repository has one `dojo.repositories_in_workspaces` row, under the expected slug.
- [ ] 2.3 Playwright: open `/org/<slug>/workspaces`. Assert the workspace for that repository is listed
      (the `ScrWorkspaces` list, `dojo/src/routes/(app)/org/[slug]/[section]/+page.svelte:138-146`).
- [ ] 2.4 Run a second sync cycle. Assert the list is unchanged (same row count, same workspace id).
- [ ] 2.5 Sign in as a second local user with no role on that repository. Assert the workspace is not listed.
- [ ] 2.6 Register a repository whose org has no connection. Assert it is reported `unmapped` and no workspace
      appears for it.
- [ ] 2.7 Post the results (counts and workspace names only, no transcript text, no emails) as a comment on
      #299.

## 3. Production runbook (user only; not run by the workflow)

- [ ] 3.1 Approve the installed daemon's keychain access: `sensei auth approve` (#283).
- [ ] 3.2 Confirm the daemon is enrolled with `https://dojo.sensei-hq.com` and the persona is signed in.
- [ ] 3.3 Wait for one `dojo_sync` cycle. In Settings → dōjō, the repository rows show "Syncing" with no
      inline notice. (Both `synced` and `partial` render "Syncing"; a `partial` row carries the workspace
      error as an inline notice, `app/src/routes/(preferences)/settings/dojo/dojo-connection.ts:104-123`.)
- [ ] 3.4 Sign in to the production dōjō. Open `organization/sensei-hq` → Workspaces. `sensei` is listed.
      Open the personal dōjō; personal repositories' workspaces are listed there and not under the org.
- [ ] 3.5 Comment the evidence on #299 and close it with "Closes #299" wording in the comment.
