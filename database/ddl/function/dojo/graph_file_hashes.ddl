set search_path to dojo, extensions;

-- What the dōjō holds of one branch of a repository's shared graph: each file's
-- repository-relative path and accepted content hash (#225; graph-sync.md §3).
--
-- The daemon diffs its own files against this to push only what changed. Gated
-- like the WRITE, not the team read: whoever may sync the repository (their own
-- verdict row says sync_enabled) may read it — a member on no team owning the
-- project would otherwise see nothing, and re-push every file every cycle.
-- Paths and hashes only; no node, edge or content leaves through here.
--
-- SECURITY DEFINER: graph_files' row policy is the team read.
create or replace function dojo.graph_file_hashes(p_repository_id uuid, p_branch text)
    returns table (path text, content_hash text)
    language plpgsql
    stable
    security definer
    set search_path = dojo, pg_temp
as $$
#variable_conflict use_column
begin
    if not exists (
        select 1 from dojo.all_my_repositories v
         where v.principal_id = dojo.current_principal_id()
           and v.sync_enabled
           and v.repository_id = p_repository_id) then
        raise exception 'the caller may not sync repository %', p_repository_id
            using errcode = 'insufficient_privilege';
    end if;
    return query
    select f.path, f.content_hash
      from dojo.graph_files f
     where f.repository_id = p_repository_id
       and f.branch = p_branch
     order by f.path;
end;
$$;

revoke all on function dojo.graph_file_hashes(uuid, text) from public;
grant execute on function dojo.graph_file_hashes(uuid, text) to authenticated;

comment on function dojo.graph_file_hashes(uuid, text) is
'Path and accepted content hash of each file the dōjō holds for one branch of a repository''s shared graph, for a caller who may sync it — so a daemon pushes only what changed. Paths and hashes only (#225).';
