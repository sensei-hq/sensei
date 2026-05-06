set search_path to sensei, extensions;

grant select on call_edges to authenticated;
grant all    on call_edges to service_role;

alter table call_edges enable row level security;
drop policy if exists call_edges_select on call_edges;
create policy call_edges_select on call_edges
  for select to authenticated using (public.can_access_repo(repo_id));
