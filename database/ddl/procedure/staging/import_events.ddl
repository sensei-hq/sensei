set search_path to staging, extensions;

create or replace procedure import_events()
language plpgsql
as $$
begin
  insert into sensei.events (
      id, user_uuid, session_id, folder_id, phase
    , tool, project_path, input, ts, created_at
    , modified_at
  )
  select
      coalesce(stg.id, gen_random_uuid())
    , stg.user_uuid, stg.session_id, stg.folder_id, stg.phase
    , stg.tool, stg.project_path, stg.input, stg.ts
    , coalesce(stg.created_at, now())
    , coalesce(stg.modified_at, stg.created_at, now())
  from staging.events stg
  join sensei.folders f on f.id = stg.folder_id
  where stg.user_uuid is not null
    and stg.phase in ('pre', 'post')
  on conflict (id) do nothing;
end;
$$;

comment on procedure import_events is
'Import staging.events into sensei.events.
Joins to sensei.folders to skip rows whose folder has not been imported yet.
Only imports events with valid phase (pre or post).';
