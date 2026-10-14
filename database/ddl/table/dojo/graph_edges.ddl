set search_path to dojo, extensions;

-- An edge of a repository's shared code graph (#225; graph-sync.md §2–3).
--
-- The source is a row here and the edge goes with it. The TARGET is a key —
-- (target_folder, target_fqn) — resolved against graph_nodes when read, so an
-- edge into a file not yet filed is kept and resolves once that file arrives.
-- An edge the indexer could not resolve has no target key, only target_name.
create table if not exists graph_edges (
  id                bigserial   primary key
, tenant_id         uuid        not null references dojo.tenants(id) on delete cascade
, repository_id     uuid        not null references dojo.repositories(id) on delete cascade
, source_id         uuid        not null references dojo.graph_nodes(id) on delete cascade
, kind              text        not null
, target_folder     text
, target_fqn        text
, target_name       text
, confidence        text
, resolved_via      text
, unresolved_reason text
, props             jsonb       not null default '{}'
);

create index if not exists graph_edges_source on graph_edges(source_id);
create index if not exists graph_edges_target on graph_edges(repository_id, target_folder, target_fqn);

comment on table graph_edges is
'Edges of a repository''s shared code graph. Source by row, target by (folder, fqn) key resolved at read time; unresolved edges keep only target_name. props carries occurrences. Written only by dojo.file_graph_slices.';
