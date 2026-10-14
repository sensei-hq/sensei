set search_path to dojo, extensions;

-- File a batch of a repository's code-graph slices for the signed-in person's
-- daemon (#225; docs/spec/dojo/graph-sync.md).
--
-- A slice is one file's nodes and the edges whose source is in it, or — with a
-- null path — the repository's unplaced nodes. p_commit is
-- {branch, sha, committed_at}; p_manifest, when given, is the repository's whole
-- file list {path: content_hash}.
--
--   1. The repository must be one the caller may sync — their own verdict row of
--      dojo.all_my_repositories says sync_enabled — else insufficient_privilege.
--      The tenant comes from that row, never from the input.
--   2. The branch must be one the repository tracks
--      (dojo.repository_graph_branches): none tracked → branch_unknown; another →
--      branch_not_tracked (invalid_parameter_value). Each tracked branch is its
--      own graph; this call touches only the one it names.
--   3. A slice REPLACES its file's rows on that branch. One older than the commit
--      stored for its path (or, unplaced, than the branch's head) is refused and
--      counted — a member a few commits behind does not roll the branch back.
--   4. A manifest prunes the branch's files absent from it, only when its commit
--      is at least the head's. The unplaced nodes are not a file and are never
--      pruned.
--
-- Returns {files, refused_older, pruned}. SECURITY DEFINER: the graph tables take
-- no session write.
create or replace function dojo.file_graph_slices(
  p_repository_id uuid,
  p_commit        jsonb,
  p_slices        jsonb,
  p_manifest      jsonb
) returns jsonb
language plpgsql
volatile
security definer
set search_path = dojo, pg_temp
as $$
declare
    v_principal uuid := dojo.current_principal_id();
    v_tenant    uuid;
    v_branch    text        := p_commit ->> 'branch';
    v_sha       text        := p_commit ->> 'sha';
    v_at        timestamptz := (p_commit ->> 'committed_at')::timestamptz;
    v_head      dojo.graph_heads;
    v_slice     jsonb;
    v_path      text;
    v_stored_at timestamptz;
    v_files     integer := 0;
    v_older     integer := 0;
    v_pruned    integer := 0;
begin
    select v.tenant_id into v_tenant
      from dojo.all_my_repositories v
     where v.principal_id = v_principal
       and v.sync_enabled
       and v.repository_id = p_repository_id;
    if v_tenant is null then
        raise exception 'the caller may not sync repository %', p_repository_id
            using errcode = 'insufficient_privilege';
    end if;

    if not exists (select 1 from dojo.repository_graph_branches t where t.repository_id = p_repository_id) then
        raise exception 'branch_unknown: repository % tracks no branch (no default captured, none added)', p_repository_id
            using errcode = 'invalid_parameter_value';
    end if;
    if not exists (select 1 from dojo.repository_graph_branches t
                    where t.repository_id = p_repository_id and t.branch = v_branch) then
        raise exception 'branch_not_tracked: repository % does not track branch %', p_repository_id, v_branch
            using errcode = 'invalid_parameter_value';
    end if;
    if v_sha is null or v_at is null then
        raise exception 'a graph commit needs a sha and committed_at'
            using errcode = 'invalid_parameter_value';
    end if;

    select * into v_head from dojo.graph_heads h
     where h.repository_id = p_repository_id and h.branch = v_branch;

    for v_slice in select e from jsonb_array_elements(coalesce(p_slices, '[]'::jsonb)) e loop
        v_path := v_slice ->> 'path';

        if v_path is null then
            if v_head.committed_at is not null and v_head.committed_at > v_at then
                v_older := v_older + 1;
                continue;
            end if;
            delete from dojo.graph_nodes
             where repository_id = p_repository_id and branch = v_branch and file_path is null;
        else
            select f.committed_at into v_stored_at
              from dojo.graph_files f
             where f.repository_id = p_repository_id and f.branch = v_branch and f.path = v_path;
            if v_stored_at is not null and v_stored_at > v_at then
                v_older := v_older + 1;
                continue;
            end if;
            -- The file's nodes, and their edges, go with it.
            delete from dojo.graph_files
             where repository_id = p_repository_id and branch = v_branch and path = v_path;
            insert into dojo.graph_files (tenant_id, repository_id, branch, path, content_hash, commit_sha, committed_at)
            values (v_tenant, p_repository_id, v_branch, v_path, v_slice ->> 'content_hash', v_sha, v_at);
            v_files := v_files + 1;
        end if;

        -- A symbol that moved here from another file: the stale copy goes, with
        -- its edges, before this slice claims the key.
        delete from dojo.graph_nodes g
         using jsonb_array_elements(coalesce(v_slice -> 'nodes', '[]'::jsonb)) n
         where g.repository_id = p_repository_id
           and g.branch = v_branch
           and g.folder_path = n ->> 'folder_path'
           and g.fqn = n ->> 'fqn';

        insert into dojo.graph_nodes
               (tenant_id, repository_id, branch, file_path, folder_path, fqn, kind, name, parent_fqn,
                line_start, line_end, is_exported, is_test, signature, description, docstring,
                language, community_id, tags, props)
        select v_tenant, p_repository_id, v_branch, v_path, n ->> 'folder_path', n ->> 'fqn', n ->> 'kind',
               n ->> 'name', n ->> 'parent_fqn', (n ->> 'line_start')::int, (n ->> 'line_end')::int,
               coalesce((n ->> 'is_exported')::boolean, false), coalesce((n ->> 'is_test')::boolean, false),
               n ->> 'signature', n ->> 'description', n ->> 'docstring', n ->> 'language',
               (n ->> 'community_id')::int,
               coalesce(array(select jsonb_array_elements_text(n -> 'tags')), '{}'),
               coalesce(n -> 'props', '{}'::jsonb)
          from jsonb_array_elements(coalesce(v_slice -> 'nodes', '[]'::jsonb)) n;

        insert into dojo.graph_edges
               (tenant_id, repository_id, branch, source_id, kind, target_folder, target_fqn, target_name,
                confidence, resolved_via, unresolved_reason, props)
        select v_tenant, p_repository_id, v_branch, g.id, e ->> 'kind', e ->> 'target_folder', e ->> 'target_fqn',
               e ->> 'target_name', e ->> 'confidence', e ->> 'resolved_via', e ->> 'unresolved_reason',
               coalesce(e -> 'props', '{}'::jsonb)
          from jsonb_array_elements(coalesce(v_slice -> 'edges', '[]'::jsonb)) e
          join dojo.graph_nodes g
            on g.repository_id = p_repository_id
           and g.branch = v_branch
           and g.folder_path = e ->> 'source_folder'
           and g.fqn = e ->> 'source_fqn';
    end loop;

    if p_manifest is not null and (v_head.committed_at is null or v_at >= v_head.committed_at) then
        delete from dojo.graph_files f
         where f.repository_id = p_repository_id
           and f.branch = v_branch
           and not (p_manifest ? f.path);
        get diagnostics v_pruned = row_count;
    end if;

    if v_head.committed_at is null or v_at >= v_head.committed_at then
        insert into dojo.graph_heads (repository_id, tenant_id, branch, commit_sha, committed_at)
        values (p_repository_id, v_tenant, v_branch, v_sha, v_at)
        on conflict (repository_id, branch) do update
           set commit_sha = excluded.commit_sha,
               committed_at = excluded.committed_at, pushed_at = now();
    end if;

    return jsonb_build_object('files', v_files, 'refused_older', v_older, 'pruned', v_pruned);
end;
$$;

revoke all on function dojo.file_graph_slices(uuid, jsonb, jsonb, jsonb) from public;
grant execute on function dojo.file_graph_slices(uuid, jsonb, jsonb, jsonb) to authenticated;

comment on function dojo.file_graph_slices(uuid, jsonb, jsonb, jsonb) is
'File a batch of a repository''s code-graph slices for the caller (#225): the repository must be one they may sync (tenant from their verdict row); the branch must be one the repository tracks (branch_unknown / branch_not_tracked) and only that branch''s graph is touched; a slice replaces its file, one older than the stored commit is refused and counted; a manifest at least as new as the head prunes absent files. Returns {files, refused_older, pruned}.';
