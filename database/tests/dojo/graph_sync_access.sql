-- Filing a repository's code graph as the signed-in person (#225;
-- docs/spec/dojo/graph-sync.md).
--
-- dojo.file_graph_slices files a batch of file slices — a file's nodes and the
-- edges whose source is in it — for one repository at one commit. The repository
-- must be one the caller may sync (their own verdict row says sync_enabled); the
-- tenant comes from that row, never from the input. A slice REPLACES its file's
-- rows; a slice older than the stored commit for its path is refused and
-- counted; the slice with a null path is the repository's unplaced nodes (no
-- file), replaced whole. A manifest prunes the files absent from it — only when
-- its commit is at least the repository's newest accepted commit.
--
-- Branches: a repository's graph is kept per TRACKED branch — the rows of
-- dojo.repository_graph_branches (the forge default and any the team adds). A
-- slice from an untracked branch is refused (branch_not_tracked); with no
-- tracked branch, nothing is filed (branch_unknown) — never a guessed main. Two
-- tracked branches are two graphs side by side; removing a branch removes its
-- graph and nothing else.
--
-- Reads: dojo.can_read_repository — the repo-scoped rule repository metrics use.
--
-- WHAT BREAKS THIS TEST: a slice that appends instead of replacing; a replace
-- that touches another file; an older commit rolling a file back; a stale
-- manifest pruning; a tenant taken from the input; a repository the caller may
-- not sync accepted; any session write grant; a reader outside the repository's
-- teams seeing rows.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000141', 'a1000000-0000-4000-8000-000000000141', 'ada'),
  ('a0000000-0000-4000-8000-000000000142', 'a1000000-0000-4000-8000-000000000142', 'ola'),
  ('a0000000-0000-4000-8000-000000000143', 'a1000000-0000-4000-8000-000000000143', 'eve');

-- T: organisation, subscribed, shares its private repos. U: not ola's.
insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-00000000014a', 'organization/ztest-gs-t', 'organization', 'ztest-gs-t', 'T', 'x', now()),
  ('b0000000-0000-4000-8000-00000000014c', 'organization/ztest-gs-u', 'organization', 'ztest-gs-u', 'U', 'x', now());
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000014a', 'a0000000-0000-4000-8000-000000000141', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000014a', 'a0000000-0000-4000-8000-000000000142', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000014a', 'a0000000-0000-4000-8000-000000000143', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000014c', 'a0000000-0000-4000-8000-000000000141', 'admin',       'employer', 'github_oauth');
insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b0000000-0000-4000-8000-00000000014a', 'team', 'active', current_date - 1, current_date + 30);
insert into dojo.tenant_share_policy (tenant_id, private_repos_shared) values ('b0000000-0000-4000-8000-00000000014a', true);

-- 1 T private (syncs for ola via the org policy) · 2 T public (nobody elected it) ·
-- 3 T private, no branch tracked · 4 U public (not ola's)
insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider, default_branch) values
  ('92000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-00000000014a', 'github.com/ztest-gs/priv', 'priv', 'private', now(), 'github', 'main'),
  ('92000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-00000000014a', 'github.com/ztest-gs/pub',  'pub',  'public',  now(), 'github', 'main'),
  ('92000000-0000-4000-8000-000000000003', 'b0000000-0000-4000-8000-00000000014a', 'github.com/ztest-gs/nob',  'nob',  'private', now(), 'github', null),
  ('92000000-0000-4000-8000-000000000004', 'b0000000-0000-4000-8000-00000000014c', 'github.com/ztest-gu/x',    'x',    'public',  now(), 'github', 'main');
-- Repository 1 tracks its default (main) and production.
insert into dojo.repository_graph_branches (repository_id, tenant_id, branch, is_default, label) values
  ('92000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-00000000014a', 'main',       true,  null),
  ('92000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-00000000014a', 'production', false, 'production'),
  ('92000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-00000000014a', 'main',       true,  null),
  ('92000000-0000-4000-8000-000000000004', 'b0000000-0000-4000-8000-00000000014c', 'main',       true,  null);

-- ola is on team X, which owns project P holding repository 1; eve is
-- on no team, so she reads nothing.
insert into dojo.projects (id, user_id, tenant_id, slug, name) values
  ('c0000000-0000-4000-8000-0000000000b1', 'a0000000-0000-4000-8000-000000000141',
   'b0000000-0000-4000-8000-00000000014a', 'ztest-gs-p', 'project P');
insert into dojo.teams (id, tenant_id, name) values
  ('d0000000-0000-4000-8000-0000000000b1', 'b0000000-0000-4000-8000-00000000014a', 'ztest gs team X');
insert into dojo.team_members (team_id, principal_id, role) values
  ('d0000000-0000-4000-8000-0000000000b1', 'a0000000-0000-4000-8000-000000000142', 'contributor');
insert into dojo.team_projects (team_id, project_id) values
  ('d0000000-0000-4000-8000-0000000000b1', 'c0000000-0000-4000-8000-0000000000b1');
insert into dojo.repositories_in_projects (project_id, repository_id, tenant_id) values
  ('c0000000-0000-4000-8000-0000000000b1', '92000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-00000000014a');

set local role authenticated;

do $$
declare
    repo   uuid := '92000000-0000-4000-8000-000000000001';
    c1     jsonb := '{"branch": "main", "sha": "c1", "committed_at": "2026-10-01T10:00:00Z"}';
    c2     jsonb := '{"branch": "main", "sha": "c2", "committed_at": "2026-10-02T10:00:00Z"}';
    c0     jsonb := '{"branch": "main", "sha": "c0", "committed_at": "2026-09-30T10:00:00Z"}';
    a      jsonb;
    b      jsonb;
    unplaced jsonb;
    r      jsonb;
    n      int;
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000142"}', true);

    -- a carries a forged tenant (U's): the verdict's tenant must win.
    a := jsonb_build_object('path', 'src/a.ts', 'content_hash', 'ha1', 'tenant_id', 'b0000000-0000-4000-8000-00000000014c',
        'nodes', jsonb_build_array(
            jsonb_build_object('folder_path', '', 'fqn', 'a.f', 'kind', 'function', 'name', 'f', 'line_start', 1, 'line_end', 3),
            jsonb_build_object('folder_path', '', 'fqn', 'a.g', 'kind', 'function', 'name', 'g', 'line_start', 5, 'line_end', 9)),
        'edges', jsonb_build_array(
            jsonb_build_object('source_folder', '', 'source_fqn', 'a.f', 'kind', 'calls',
                               'target_folder', '', 'target_fqn', 'b.h', 'target_name', 'h',
                               'props', '{"occurrences": {"src/a.ts": [[2, 4]]}}'::jsonb)));
    b := jsonb_build_object('path', 'src/b.ts', 'content_hash', 'hb1',
        'nodes', jsonb_build_array(
            jsonb_build_object('folder_path', '', 'fqn', 'b.h', 'kind', 'function', 'name', 'h', 'line_start', 1, 'line_end', 2)),
        'edges', '[]'::jsonb);
    unplaced := jsonb_build_object('path', null,
        'nodes', jsonb_build_array(jsonb_build_object('folder_path', '', 'fqn', 'pkg', 'kind', 'package', 'name', 'pkg')),
        'edges', '[]'::jsonb);

    -- ── file: the slices land, under the verdict's tenant ────────────────────
    select to_jsonb(g) into r from dojo.file_graph_slices(repo, c1, jsonb_build_array(a, b, unplaced), null) g;
    if (r ->> 'files')::int <> 2 then raise exception 'file: two file slices should be filed, got %', r; end if;
    select count(*) into n from dojo.graph_nodes where repository_id = repo;
    if n <> 4 then raise exception 'file: 4 nodes expected (2 + 1 + 1 unplaced), got %', n; end if;
    select count(*) into n from dojo.graph_edges where repository_id = repo;
    if n <> 1 then raise exception 'file: 1 edge expected, got %', n; end if;
    if exists (select 1 from dojo.graph_nodes where repository_id = repo and tenant_id <> 'b0000000-0000-4000-8000-00000000014a')
       or exists (select 1 from dojo.graph_files where repository_id = repo and tenant_id <> 'b0000000-0000-4000-8000-00000000014a') then
        raise exception 'file: a row was filed under a tenant other than the verdict''s';
    end if;

    -- ── replace: a newer slice of a.ts replaces a.ts only ────────────────────
    a := jsonb_set(jsonb_set(a, '{content_hash}', '"ha2"'), '{nodes}',
        jsonb_build_array(jsonb_build_object('folder_path', '', 'fqn', 'a.f', 'kind', 'function', 'name', 'f', 'line_start', 1, 'line_end', 4)));
    a := jsonb_set(a, '{edges}', '[]'::jsonb);
    select to_jsonb(g) into r from dojo.file_graph_slices(repo, c2, jsonb_build_array(a), null) g;
    select count(*) into n from dojo.graph_nodes where repository_id = repo and file_path = 'src/a.ts';
    if n <> 1 then raise exception 'replace: a.ts should hold exactly its new slice''s 1 node, holds %', n; end if;
    select count(*) into n from dojo.graph_edges where repository_id = repo;
    if n <> 0 then raise exception 'replace: a.ts''s old edge should be gone, % remain', n; end if;
    select count(*) into n from dojo.graph_nodes where repository_id = repo and file_path = 'src/b.ts';
    if n <> 1 then raise exception 'replace: b.ts must be untouched by a.ts''s slice, holds %', n; end if;

    -- ── move: a symbol that moved files leaves no stale copy, and no edge ────
    select to_jsonb(g) into r from dojo.file_graph_slices(repo, c2, jsonb_build_array(jsonb_build_object(
        'path', 'src/c.ts', 'content_hash', 'hc1',
        'nodes', jsonb_build_array(jsonb_build_object('folder_path', '', 'fqn', 'a.f', 'kind', 'function', 'name', 'f')),
        'edges', '[]'::jsonb)), null) g;
    select count(*) into n from dojo.graph_nodes where repository_id = repo and fqn = 'a.f';
    if n <> 1 then raise exception 'move: a.f should exist once, in c.ts, exists % time(s)', n; end if;
    if (select file_path from dojo.graph_nodes where repository_id = repo and fqn = 'a.f') <> 'src/c.ts' then
        raise exception 'move: a.f should now belong to src/c.ts';
    end if;

    -- ── schema: a table's columns, indexes and keys ride on its node ─────────
    select to_jsonb(g) into r from dojo.file_graph_slices(repo, c2, jsonb_build_array(jsonb_build_object(
        'path', 'db/t.sql', 'content_hash', 'ht1',
        'nodes', jsonb_build_array(
            jsonb_build_object('folder_path', '', 'fqn', 'app.orders', 'kind', 'table', 'name', 'orders',
                'schema', jsonb_build_object(
                    'entity', jsonb_build_object('kind', 'table', 'db_schema', 'app', 'source', 'dbd', 'naming', 'stated'),
                    'columns', jsonb_build_array(
                        jsonb_build_object('ordinal', 1, 'name', 'id', 'sql_type', 'uuid', 'primary_key', true, 'not_null', true),
                        jsonb_build_object('ordinal', 2, 'name', 'customer_id', 'sql_type', 'uuid', 'foreign_key', true)),
                    'indexes', jsonb_build_array(
                        jsonb_build_object('name', 'orders_customer', 'definition', 'create index …', 'unique_index', false),
                        -- An inline UNIQUE (…) has no name; dbd reports it unnamed, and so does the slice.
                        jsonb_build_object('name', null, 'definition', '(customer_id, placed_at)', 'unique_index', true)),
                    'refs', jsonb_build_array(
                        jsonb_build_object('from_column', 'customer_id', 'to_folder', '', 'to_fqn', 'app.customers',
                                           'to_column', 'id', 'on_delete', 'cascade')))),
            jsonb_build_object('folder_path', '', 'fqn', 'app.customers', 'kind', 'table', 'name', 'customers')),
        'edges', '[]'::jsonb)), null) g;
    select count(*) into n from dojo.graph_schema_columns c
      join dojo.graph_nodes g on g.id = c.node_id where g.repository_id = repo and g.fqn = 'app.orders';
    if n <> 2 then raise exception 'schema: orders should carry 2 columns, carries %', n; end if;
    if (select db_schema from dojo.graph_schema_entities e join dojo.graph_nodes g on g.id = e.node_id
         where g.repository_id = repo and g.fqn = 'app.orders') is distinct from 'app' then
        raise exception 'schema: the entity row did not land';
    end if;
    if (select to_fqn from dojo.graph_schema_refs x join dojo.graph_nodes g on g.id = x.from_node_id
         where g.repository_id = repo and g.fqn = 'app.orders') is distinct from 'app.customers' then
        raise exception 'schema: the foreign key should name its target by key';
    end if;
    select count(*) into n from dojo.graph_schema_indexes i join dojo.graph_nodes g on g.id = i.node_id
     where g.repository_id = repo and g.fqn = 'app.orders';
    if n <> 2 then raise exception 'schema: orders should carry 2 indexes, carries %', n; end if;
    if not exists (select 1 from dojo.graph_schema_indexes i join dojo.graph_nodes g on g.id = i.node_id
                    where g.repository_id = repo and g.fqn = 'app.orders' and i.name is null
                      and i.definition = '(customer_id, placed_at)') then
        raise exception 'schema: an unnamed index should land with no name, not be refused or renamed';
    end if;
    -- Re-filed without a schema: the old rows go with the old nodes.
    select to_jsonb(g) into r from dojo.file_graph_slices(repo, c2, jsonb_build_array(jsonb_build_object(
        'path', 'db/t.sql', 'content_hash', 'ht2',
        'nodes', jsonb_build_array(jsonb_build_object('folder_path', '', 'fqn', 'app.orders', 'kind', 'table', 'name', 'orders')),
        'edges', '[]'::jsonb)), null) g;
    if exists (select 1 from dojo.graph_schema_columns c join dojo.graph_nodes g on g.id = c.node_id
                where g.repository_id = repo and g.fqn = 'app.orders') then
        raise exception 'schema: a re-filed slice left its old columns behind';
    end if;

    -- ── order: an older commit does not roll a file back ─────────────────────
    select to_jsonb(g) into r from dojo.file_graph_slices(repo, c0, jsonb_build_array(jsonb_set(b, '{content_hash}', '"hb0"')), null) g;
    if (r ->> 'refused_older')::int <> 1 then raise exception 'order: an older slice should be refused and counted, got %', r; end if;
    if (select content_hash from dojo.graph_files where repository_id = repo and branch = 'main' and path = 'src/b.ts') <> 'hb1' then
        raise exception 'order: an older commit rolled b.ts back';
    end if;

    -- ── prune: a stale manifest prunes nothing; a current one prunes ─────────
    perform dojo.file_graph_slices(repo, c0, '[]'::jsonb, '{"src/a.ts": "ha2", "src/c.ts": "hc1"}'::jsonb);
    if not exists (select 1 from dojo.graph_files where repository_id = repo and branch = 'main' and path = 'src/b.ts') then
        raise exception 'prune: a manifest older than the newest accepted commit pruned b.ts';
    end if;
    select to_jsonb(g) into r from dojo.file_graph_slices(repo, c2, '[]'::jsonb, '{"src/a.ts": "ha2", "src/c.ts": "hc1"}'::jsonb) g;
    if exists (select 1 from dojo.graph_files where repository_id = repo and branch = 'main' and path = 'src/b.ts')
       or exists (select 1 from dojo.graph_nodes where repository_id = repo and branch = 'main' and file_path = 'src/b.ts') then
        raise exception 'prune: b.ts is absent from a current manifest and should be gone';
    end if;
    if not exists (select 1 from dojo.graph_nodes where repository_id = repo and branch = 'main' and file_path is null) then
        raise exception 'prune: the unplaced nodes are not a file and must survive a manifest';
    end if;

    -- ── branch: tracked branches only, never a guess ─────────────────────────
    begin
        perform dojo.file_graph_slices(repo, c1 || '{"branch": "feature-x"}', jsonb_build_array(b), null);
        raise exception 'branch: a slice from an untracked branch was filed';
    exception when invalid_parameter_value then
        if sqlerrm not like '%branch_not_tracked%' then raise exception 'branch: wrong refusal: %', sqlerrm; end if;
    end;
    begin
        perform dojo.file_graph_slices('92000000-0000-4000-8000-000000000003', c1, jsonb_build_array(b), null);
        raise exception 'branch: a repository tracking no branch was filed under a guess';
    exception when invalid_parameter_value then
        if sqlerrm not like '%branch_unknown%' then raise exception 'branch: wrong refusal: %', sqlerrm; end if;
    end;

    -- ── two branches: production is a second graph, beside main ─────────────
    -- Dated before main's head: branches do not order each other.
    select to_jsonb(g) into r from dojo.file_graph_slices(repo, c0 || '{"branch": "production"}', jsonb_build_array(
        jsonb_build_object('path', 'src/a.ts', 'content_hash', 'hp1',
            'nodes', jsonb_build_array(jsonb_build_object('folder_path', '', 'fqn', 'a.f', 'kind', 'function', 'name', 'f')),
            'edges', '[]'::jsonb),
        jsonb_build_object('path', 'src/p.ts', 'content_hash', 'hpp',
            'nodes', jsonb_build_array(jsonb_build_object('folder_path', '', 'fqn', 'p.q', 'kind', 'function', 'name', 'q')),
            'edges', '[]'::jsonb)), null) g;
    if (r ->> 'files')::int <> 2 then raise exception 'two branches: production''s slices should land, got %', r; end if;
    if (select content_hash from dojo.graph_files where repository_id = repo and branch = 'main' and path = 'src/a.ts') <> 'ha2' then
        raise exception 'two branches: production''s a.ts overwrote main''s';
    end if;
    select count(*) into n from dojo.graph_nodes where repository_id = repo and branch = 'production';
    if n <> 2 then raise exception 'two branches: production should hold its 2 nodes, holds %', n; end if;

    -- main works on: re-files a.ts, re-claims a.f in c.ts, sends a manifest
    -- without p.ts. None of it may reach production.
    perform dojo.file_graph_slices(repo, c2, jsonb_build_array(a), null);
    perform dojo.file_graph_slices(repo, c2, jsonb_build_array(jsonb_build_object(
        'path', 'src/c.ts', 'content_hash', 'hc1',
        'nodes', jsonb_build_array(jsonb_build_object('folder_path', '', 'fqn', 'a.f', 'kind', 'function', 'name', 'f')),
        'edges', '[]'::jsonb)), '{"src/a.ts": "ha2", "src/c.ts": "hc1"}'::jsonb);
    select count(*) into n from dojo.graph_nodes where repository_id = repo and branch = 'production';
    if n <> 2 then raise exception 'two branches: main''s work changed production''s graph (% node(s) left of 2)', n; end if;
    select count(*) into n from dojo.graph_files where repository_id = repo and branch = 'production';
    if n <> 2 then raise exception 'two branches: main''s manifest pruned production''s files (% left of 2)', n; end if;

    -- ── hashes: what the dōjō holds, readable by whoever may sync it ─────────
    -- ola can sync repository 1 (she is also on its team, but the read must not
    -- depend on that — see the eve case below).
    select count(*) into n from dojo.graph_file_hashes(repo, 'main');
    if n <> (select count(*) from dojo.graph_files where repository_id = repo and branch = 'main') then
        raise exception 'hashes: should list every main file, listed %', n;
    end if;
    if (select content_hash from dojo.graph_file_hashes(repo, 'main') h where h.path = 'src/a.ts') <> 'ha2' then
        raise exception 'hashes: a.ts should read back its accepted hash';
    end if;
    if exists (select 1 from dojo.graph_file_hashes(repo, 'main') h where h.path = 'src/p.ts') then
        raise exception 'hashes: production''s file p.ts was listed under main';
    end if;
    begin
        perform dojo.graph_file_hashes('92000000-0000-4000-8000-000000000004', 'main');
        raise exception 'hashes: another tenant''s repository was read';
    exception when insufficient_privilege then null;
    end;

    -- ── access: only a repository the caller may sync ────────────────────────
    begin
        perform dojo.file_graph_slices('92000000-0000-4000-8000-000000000002', c1, jsonb_build_array(b), null);
        raise exception 'access: a repository nobody elected was filed';
    exception when insufficient_privilege then null;
    end;
    begin
        perform dojo.file_graph_slices('92000000-0000-4000-8000-000000000004', c1, jsonb_build_array(b), null);
        raise exception 'access: another tenant''s repository was filed';
    exception when insufficient_privilege then null;
    end;
    begin
        insert into dojo.graph_files (tenant_id, repository_id, path, content_hash, commit_sha, committed_at)
        values ('b0000000-0000-4000-8000-00000000014a', repo, 'x', 'h', 'c', now());
        raise exception 'graph_files: a session wrote the table directly';
    exception when insufficient_privilege then null;
    end;

    -- ── read: the tenant admin reads; a contributor on no team does not ──────
    -- (ola reads as a member of the team owning the project, which every check
    -- above relied on.)
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000141"}', true);
    select count(*) into n from dojo.graph_nodes where repository_id = repo;
    if n = 0 then raise exception 'read: the tenant admin should read the repository''s graph'; end if;
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000143"}', true);
    select count(*) into n from dojo.graph_nodes where repository_id = repo;
    if n <> 0 then raise exception 'read: a contributor on no team owning the repository read % node(s)', n; end if;
    -- …yet she may SYNC it (the organisation shares its private repositories), so
    -- her daemon must see what the dōjō holds, or it re-pushes every file forever.
    select count(*) into n from dojo.graph_file_hashes(repo, 'main');
    if n = 0 then raise exception 'hashes: a member who may sync the repository read no hashes'; end if;
end $$;

-- ── untrack: removing production removes its graph, and only its graph ──────
reset role;
delete from dojo.repository_graph_branches
 where repository_id = '92000000-0000-4000-8000-000000000001' and branch = 'production';
do $$
begin
    if exists (select 1 from dojo.graph_nodes where repository_id = '92000000-0000-4000-8000-000000000001' and branch = 'production')
       or exists (select 1 from dojo.graph_files where repository_id = '92000000-0000-4000-8000-000000000001' and branch = 'production')
       or exists (select 1 from dojo.graph_heads where repository_id = '92000000-0000-4000-8000-000000000001' and branch = 'production') then
        raise exception 'untrack: production''s graph outlived its branch';
    end if;
    if not exists (select 1 from dojo.graph_nodes where repository_id = '92000000-0000-4000-8000-000000000001' and branch = 'main') then
        raise exception 'untrack: removing production took main''s graph with it';
    end if;
end $$;

rollback;
