set search_path to dojo, extensions;

-- Per-repository latest activity for the ids passed in that the caller may
-- read: the newest of any reading landed (`repository_metrics.pushed_at`) or
-- graph sync filed (`graph_heads.pushed_at`). Backs the repository sort (most
-- recently active first, quiet last, never invented) and the timestamp the
-- lists show, so the two can never disagree (#531).
--
-- SECURITY DEFINER, gated per id by `dojo.can_read_repository` — called,
-- never copied: the links admit no direct reads, so an invoker body fails
-- every caller with `permission denied`. The caller learns activity only for
-- repositories they may read. A timestamp exposes no metric value.
--
-- Each max is taken in its own subquery: readings and graph heads never share
-- a FROM, so no join multiplies readings by branch count (MAX would still
-- answer right; the rows would still cost).
--
-- The reader is `listMyRepositories`, calling it as the signed-in person via
-- rpc with the listed ids. Unknown ids read no row; quiet repositories read
-- one NULL row.
create or replace function dojo.repository_activity(p_ids uuid[])
returns table (
    repository_id uuid
  , tenant_id uuid
  , last_activity_at timestamptz
)
language sql
stable
security definer
set search_path = dojo, pg_temp
as $$
  select r.id
       , r.tenant_id
       , greatest(
             (select max(rm.pushed_at)
                from dojo.repository_metrics rm
               where rm.repository_id = r.id),
             (select max(gh.pushed_at)
                from dojo.graph_heads gh
               where gh.repository_id = r.id))
    from unnest(p_ids) u(id)
    join dojo.repositories r on r.id = u.id
   where dojo.can_read_repository(r.id, r.tenant_id);
$$;

revoke all on function dojo.repository_activity(uuid[]) from public;
grant execute on function dojo.repository_activity(uuid[]) to authenticated;

comment on function dojo.repository_activity(uuid[]) is
'Latest activity per repository the caller may read — newest reading landed or graph sync filed (#531). Backs the repository sort and the timestamp the lists show.';
