-- RLS for dojo.repository_graph_branches — a repository's shared code graph (#225; graph-sync.md).
--
-- Here rather than with the table because the policy calls a function, which
-- `dbd apply` creates after every table (see repository_metrics.sql).

alter table dojo.repository_graph_branches enable row level security;

-- Read: the rule every repository-scoped row shares (dojo.can_read_repository).
drop policy if exists repository_graph_branches_read on dojo.repository_graph_branches;
create policy repository_graph_branches_read on dojo.repository_graph_branches
  for select to authenticated
  using (dojo.can_read_repository(repository_id, tenant_id));

-- Writes go through the forge capture and the branch-setting function only.
drop policy if exists repository_graph_branches_no_client_write on dojo.repository_graph_branches;
create policy repository_graph_branches_no_client_write on dojo.repository_graph_branches
    for all to anon using (false) with check (false);

-- RLS filters WHICH rows; the grant is what lets the role read the table at all.
grant select on dojo.repository_graph_branches to authenticated;
