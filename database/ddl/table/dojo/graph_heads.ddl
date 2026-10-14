set search_path to dojo, extensions;

-- Where each tracked branch of a repository's shared code graph stands: the
-- newest commit filed for it (#225; docs/spec/dojo/graph-sync.md).
--
-- The order rule for what has no file of its own — the unplaced slice — and for
-- manifests: neither is accepted from a commit older than this one, so a member
-- a few commits behind cannot roll the repository back.
create table if not exists graph_heads (
  repository_id uuid        not null references dojo.repositories(id) on delete cascade
, branch        text        not null
, tenant_id     uuid        not null references dojo.tenants(id) on delete cascade
, commit_sha    text        not null
, committed_at  timestamptz not null
, pushed_at     timestamptz not null default now()
, primary key (repository_id, branch)
, foreign key (repository_id, branch) references dojo.repository_graph_branches(repository_id, branch) on delete cascade
);

comment on table graph_heads is
'One row per tracked branch with a shared code graph: the newest commit filed. Written only by dojo.file_graph_slices.';
