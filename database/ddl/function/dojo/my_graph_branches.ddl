set search_path to dojo, extensions;

-- The tracked graph branches of every repository the caller may sync — what the
-- sync plan hands the daemon, so it pushes the checkouts on those branches and
-- no others (#225; graph-sync.md §9).
--
-- Gated like the write (the caller's verdict says sync_enabled), not like the
-- team read: a member on no team owning the project may still sync, and under
-- the table's read policy their daemon would learn of no branch and push nothing.
--
-- SECURITY DEFINER: repository_graph_branches' row policy is the team read.
create or replace function dojo.my_graph_branches()
    returns table (repository_id uuid, branch text, is_default boolean)
    language sql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
  select b.repository_id, b.branch, b.is_default
    from dojo.repository_graph_branches b
    join dojo.all_my_repositories v
      on v.repository_id = b.repository_id
     and v.principal_id = dojo.current_principal_id()
     and v.sync_enabled
   order by b.repository_id, b.branch;
$$;

revoke all on function dojo.my_graph_branches() from public;
grant execute on function dojo.my_graph_branches() to authenticated;

comment on function dojo.my_graph_branches() is
'Tracked graph branches of every repository the caller may sync (their verdict says sync_enabled) — the sync plan''s branches, so a daemon pushes only checkouts on them (#225).';
