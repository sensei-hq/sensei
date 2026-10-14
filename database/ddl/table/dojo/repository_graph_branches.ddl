set search_path to dojo, extensions;

-- The branches whose code graph a repository shares — the forge default and any
-- the team adds, e.g. production or develop (#225; graph-sync.md §9).
--
-- One graph per row: every dojo.graph_* row names its branch and cascades from
-- here, so removing a branch removes exactly its graph, and nothing is ever
-- dropped as a side effect of another change. Capturing the forge default adds
-- its row (is_default) when missing and never removes one — a repository whose
-- default moved keeps the old branch until someone untracks it.
create table if not exists repository_graph_branches (
  repository_id uuid        not null references dojo.repositories(id) on delete cascade
, tenant_id     uuid        not null references dojo.tenants(id) on delete cascade
, branch        text        not null
, is_default    boolean     not null default false
, label         text
, added_by      uuid        references dojo.principals(id) on delete set null
, added_at      timestamptz not null default now()
, primary key (repository_id, branch)
);

comment on table repository_graph_branches is
'The tracked branches of a repository''s shared code graph: one graph each. A repository tracking none does not sync its graph (branch_unknown). Written by the forge capture (the default) and by whoever holds authority over the repository''s settings.';
comment on column repository_graph_branches.added_by
     is 'Who added the branch; NULL for the forge default, which the capture adds.';
