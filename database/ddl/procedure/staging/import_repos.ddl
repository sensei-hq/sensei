set search_path to staging, extensions;

create or replace procedure import_repos()
language plpgsql
as $$
begin
  insert into sensei.repos (
      id, name, remote_url, default_branch, description
    , stack, entry_points, last_indexed_commit, last_indexed_at
    , is_public, modified_at, modified_by
  )
  select
      coalesce(stg.id, gen_random_uuid())
    , stg.name, stg.remote_url, stg.default_branch, stg.description
    , stg.stack, stg.entry_points, stg.last_indexed_commit, stg.last_indexed_at
    , coalesce(stg.is_public, false)
    , coalesce(stg.modified_at, now())
    , coalesce(stg.modified_by, current_user)
  from staging.repos stg
  where stg.name is not null
  on conflict (id)
  do update set
      name = excluded.name
    , remote_url = excluded.remote_url
    , default_branch = excluded.default_branch
    , description = excluded.description
    , stack = excluded.stack
    , entry_points = excluded.entry_points
    , last_indexed_commit = excluded.last_indexed_commit
    , last_indexed_at = excluded.last_indexed_at
    , is_public = excluded.is_public
    , modified_at = excluded.modified_at
    , modified_by = excluded.modified_by
  where excluded.modified_at >= sensei.repos.modified_at;
end;
$$;

comment on procedure import_repos is
'Import staging.repos into sensei.repos.
Upserts on id, updates only if source is newer (freshness gate).';