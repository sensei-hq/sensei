set search_path to sensei, extensions;

grant select on benchmark_runs to authenticated;
grant all    on benchmark_runs to service_role;

alter table benchmark_runs enable row level security;
drop policy if exists benchmark_runs_select on benchmark_runs;
create policy benchmark_runs_select on benchmark_runs
  for select to authenticated using (public.can_access_repo(repo_id));
