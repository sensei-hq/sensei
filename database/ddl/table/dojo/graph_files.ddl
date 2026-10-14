set search_path to dojo, extensions;

-- A file of a repository's shared code graph, at the commit its slice came from
-- (#225; docs/spec/dojo/graph-sync.md §3).
--
-- The unit of sync. `path` is repository-relative — the daemon rewrites it
-- before it leaves the machine, so every member's copy of a file is one row.
-- `content_hash` is the change marker the plan hands back: a daemon pushes a
-- file whose hash differs. Its nodes hang off this row and go with it.
create table if not exists graph_files (
  tenant_id     uuid        not null references dojo.tenants(id) on delete cascade
, repository_id uuid        not null references dojo.repositories(id) on delete cascade
, path          text        not null
, content_hash  text        not null
, commit_sha    text        not null
, committed_at  timestamptz not null
, pushed_at     timestamptz not null default now()
, primary key (repository_id, path)
);

comment on table graph_files is
'Files of a repository''s shared code graph: repository-relative path, content hash, and the commit the slice was filed at. Replaced per file; pruned by a manifest. Written only by dojo.file_graph_slices.';
