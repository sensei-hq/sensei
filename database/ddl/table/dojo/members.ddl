set search_path to hive, sensei, extensions;

create table if not exists hive.members (
  id           uuid        primary key default gen_random_uuid()
, name         text        not null
, email        text
, role         text        not null default 'member'
, disabled_at  timestamptz
, created_at   timestamptz not null default now()
);

comment on table hive.members is
'A federation participant. role gates the REST API: member=pull, publisher=pull+publish,
admin=publisher+manage members/keys/namespaces+audit. Instance-global roles (one hive
instance == one org); per-namespace ACLs are a deferred extension.';
