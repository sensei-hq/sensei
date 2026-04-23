set search_path to staging, extensions;

create or replace procedure import_teams()
language plpgsql
as $$
begin
  insert into core.teams (account_id, slug, display_name, modified_at, modified_by)
  select
      a.id
    , stg.slug
    , stg.display_name
    , coalesce(stg.modified_at, now())
    , coalesce(stg.modified_by, current_user)
  from staging.teams stg
  inner join core.accounts a on a.slug = stg.account_slug
  where stg.slug is not null
    and stg.account_slug is not null
    and not exists (
      select 1
        from core.teams t
        where t.account_id = a.id
          and t.slug = stg.slug
          and t.modified_at > coalesce(stg.modified_at, now())
    )
  on conflict (account_id, slug)
  do update set
      display_name = excluded.display_name
    , modified_at = excluded.modified_at
    , modified_by = excluded.modified_by;
end;
$$;

comment on procedure import_teams is
'Import staging.teams into core.teams.
Joins to core.accounts via account_slug, upserts on conflict.';