# verify-dojo-sync-files-workspaces

Issue: #299. dōjō Projects stayed empty unless a relay run filed the project; the metrics sync never created one.

## Why

The fix is already on `develop`. All three layers landed and shipped (S1 + S2 in v0.15.0, S3 in v0.16.0). Later work renamed
dōjō projects to workspaces (#307 D12), so the code now says "workspace" where the issue says "project".
The issue is still open because its acceptance needs evidence. A local proof was recorded on 2026-10-21. The
production proof waits on the daemon being enrolled with the production dōjō.

This change specs the behaviour as it stands, adds no new code, and lists the steps that close the issue.

Already done (each commit is an ancestor of this branch's `HEAD`):

| Layer | Commit | Where it lives now |
|---|---|---|
| Filing function | 20260dd9 | `database/ddl/function/dojo/file_repository_workspace.ddl:28-77` |
| Shared workspace table `(tenant_id, slug)` | 1e9d76b9 | `database/ddl/table/dojo/workspaces.ddl:3-16` |
| Read access follows repository access | 1e9d76b9 | `database/ddl/function/dojo/can_read_workspace.ddl:12-25` |
| Register endpoint files the workspace | 13cf12f2, 3d5c481f | `dojo/src/lib/server/repositories.ts:193-209` |
| Daemon offers `{slug, name}` per repository | 0d00ee9c | `crates/senseid/src/db/pg_store/sync.rs:932-970`, `crates/senseid/src/tasks/dojo_sync.rs:432-450` |
| One slug derivation shared with the relay | 0d00ee9c | `crates/senseid/src/db/pg_store/mod.rs:9-10` (`WORKSPACE_NAMESPACE_JOIN`) |
| Relay attaches, never creates | 3d5c481f | `database/ddl/function/dojo/attach_run_to_workspace.ddl` |
| Relay payload trimmed to slug + constitution | 99cddc18 | daemon relay publish |

## What Changes

- One production fix, found by the walkthrough (review of #503): the org Workspaces read
  (`GET /v1/t/{origin}/{org}/workspaces`) gated at `lead`, so a contributor holding a role on a workspace's
  repository got 403 on the Workspaces screen. The floor is now `member`; the workspaces RLS policy decides the
  rows (`dojo/src/routes/v1/t/[origin]/[org]/workspaces/+server.ts`). Nothing else changes in production code.
- Adds a `dojo-workspace-filing` spec that records the decided behaviour (one shared dōjō workspace per sensei
  workspace, filed by the sync, read by repository access).
- Adds verification tasks: re-run the existing tests, re-run the local dōjō Playwright walkthrough, and a
  production runbook the user runs before the issue closes.

## Impact

- Affected spec: `dojo-workspace-filing` (new).
- Affected code: none.
- Closing #299 needs production evidence (the prod daemon's sync filing a workspace in the prod dōjō). That
  step is the user's; see tasks.md section 3.
