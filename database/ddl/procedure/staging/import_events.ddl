set search_path to staging;

create or replace procedure import_events()
language plpgsql
as $$
begin
  insert into sensei.events (
      id, user_uuid, session_id, repo_id, phase
    , tool, project_path, input, ts, created_at
  )
  select
      coalesce(stg.id, gen_random_uuid())
    , stg.user_uuid, stg.session_id, stg.repo_id, stg.phase
    , stg.tool, stg.project_path, stg.input, stg.ts
    , coalesce(stg.created_at, now())
  from staging.events stg
  where stg.user_uuid is not null
    and stg.phase in ('pre', 'post')
  on conflict (id) do nothing;
end;
$$;
