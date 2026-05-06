set search_path to sensei, extensions;

grant select on benchmark_reports to authenticated;
grant all    on benchmark_reports to service_role;

alter table benchmark_reports enable row level security;
drop policy if exists benchmark_reports_select on benchmark_reports;
create policy benchmark_reports_select on benchmark_reports
  for select to authenticated
  using (repo_id is null or public.can_access_repo(repo_id));
