set search_path to staging;

create or replace procedure import_accounts()
language plpgsql
as $$
begin
  insert into core.accounts (slug, display_name, account_type, is_platform, modified_at, modified_by)
  select
      stg.slug
    , stg.display_name
    , coalesce(stg.account_type, 'individual')
    , coalesce(stg.is_platform, false)
    , coalesce(stg.modified_at, now())
    , coalesce(stg.modified_by, current_user)
  from staging.accounts stg
  where stg.slug is not null
    and not exists (
      select 1
        from core.accounts a
       where a.slug        = stg.slug
         and a.modified_at > coalesce(stg.modified_at, now())
    )
  on conflict (slug)
  do update set
      display_name = excluded.display_name
    , account_type = excluded.account_type
    , is_platform  = excluded.is_platform
    , modified_at  = excluded.modified_at
    , modified_by  = excluded.modified_by;
end;
$$;
