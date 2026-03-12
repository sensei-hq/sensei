set search_path to staging;

create table if not exists repos (
  id                  uuid
, name                text
, remote_url          text
, default_branch      text
, description         text
, stack               text[]
, entry_points        jsonb
, last_indexed_commit text
, last_indexed_at     timestamptz
, is_public           boolean
, modified_at         timestamptz default now()
);

create unique index if not exists repos_id_ukey on repos(id);
