set search_path to sensei, extensions;

grant select on events to authenticated;
grant all    on events to service_role;

alter table events enable row level security;
drop policy if exists events_select on events;
create policy events_select on events
  for select to authenticated using (user_uuid = auth.uid()::text);
