set search_path to dojo, extensions;

-- Per-workspace latest activity: the newest of any reading landed
-- (`repository_metrics.pushed_at`) or graph sync filed (`graph_heads.pushed_at`)
-- across the workspace's repositories. Backs the workspaces sort (most recently
-- active first, quiet last, never invented).
--
-- ONE ROW PER WORKSPACE, from `workspaces` leftwards: a workspace whose repos
-- pushed nothing — or which holds no repos at all — reads one NULL row, never
-- zero rows (an inner join would drop quiet workspaces from the sort) and never
-- a fabricated date.
--
-- `pushed_at`, not `computed_on`: activity is when the landing happened, and a
-- backfilled reading carries an old computed date with a new push. MAX over the
-- group: first-wins and per-repo min die in the test. The two joined row sets
-- fan out against each other (readings × syncs per repo); MAX is fan-out-proof,
-- so no dedup is needed.
--
-- SECURITY: `security_invoker = on` — runs as the calling role, not the owner.
-- Both list reads run as the caller, so the caller sees activity only through
-- the base tables' RLS as it stands (repository_metrics, graph_heads). A
-- service_role reader's whole boundary is its own `.in('workspace_id', …)`.
create or replace view dojo.workspace_activity
with (security_invoker = on)
as
select w.id                as workspace_id
     , w.tenant_id         as tenant_id
     , greatest(max(rm.pushed_at), max(gh.pushed_at)) as last_activity_at
  from dojo.workspaces w
  left join dojo.repositories_in_workspaces rip on rip.workspace_id = w.id
  left join dojo.repository_metrics rm        on rm.repository_id = rip.repository_id
  left join dojo.graph_heads gh               on gh.repository_id = rip.repository_id
 group by w.id, w.tenant_id;

comment on view dojo.workspace_activity is
'One row per workspace: the newest reading landing or graph sync across its repositories (pushed_at, never computed_on). Quiet workspaces — no readings, or no repositories — read one NULL row. The workspaces sort reads this narrowed to the listed ids and orders most-recently-active first, quiet last.';

-- The grants are stated here rather than assumed to survive a recreate. Both
-- list reads run as the caller, so `authenticated` reads this; under
-- security_invoker the base tables' RLS decides which activity rows each caller
-- sees. service_role reads the same view for the console's tenant lists.
grant select on dojo.workspace_activity to authenticated;
grant select on dojo.workspace_activity to service_role;
