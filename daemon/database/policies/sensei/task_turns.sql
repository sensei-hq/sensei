set search_path to sensei, extensions;

grant select, insert, update, delete on task_turns to authenticated, service_role;

alter table task_turns enable row level security;
drop policy if exists task_turns_select on task_turns;
drop policy if exists task_turns_write  on task_turns;
create policy task_turns_select on task_turns
  for select to authenticated using (public.can_access_repo(repo_id));
create policy task_turns_write on task_turns
  for all to authenticated
  using (public.can_access_repo(repo_id)) with check (public.can_access_repo(repo_id));
