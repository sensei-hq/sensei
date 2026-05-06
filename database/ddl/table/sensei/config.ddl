set search_path to sensei, extensions;

create table if not exists config (
  key          text        primary key
, value        text        not null
);

comment on table config is
'Key-value user preferences.
setup_complete, active_project, sidebar_max_items, etc.';

comment on column config.key is 'Preference key.';
comment on column config.value is 'Preference value stored as text (JSON-encoded for complex values).';