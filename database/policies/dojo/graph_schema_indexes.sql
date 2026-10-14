-- RLS for dojo.graph_schema_indexes — a repository's shared code graph (#225; graph-sync.md).
--
-- Here rather than with the table because the policy calls a function, which
-- `dbd apply` creates after every table (see repository_metrics.sql).

alter table dojo.graph_schema_indexes enable row level security;

-- Read: the rule every repository-scoped row shares (dojo.can_read_repository).
drop policy if exists graph_schema_indexes_read on dojo.graph_schema_indexes;
create policy graph_schema_indexes_read on dojo.graph_schema_indexes
  for select to authenticated
  using (dojo.can_read_repository(repository_id, tenant_id));

-- Writes go through dojo.file_graph_slices only.
drop policy if exists graph_schema_indexes_no_client_write on dojo.graph_schema_indexes;
create policy graph_schema_indexes_no_client_write on dojo.graph_schema_indexes
    for all to anon using (false) with check (false);

-- RLS filters WHICH rows; the grant is what lets the role read the table at all.
grant select on dojo.graph_schema_indexes to authenticated;
