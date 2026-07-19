set search_path to sensei, extensions;
create table if not exists playbooks (
  name          text        primary key
, title         text        not null
, when_to_use   text        not null
, opening_tone  text        not null
, method_ref    text
, enabled       boolean     not null default true
, source        text        not null default 'builtin'
, created_at    timestamptz not null default now()
, constraint playbooks_source_chk check (source in ('builtin','org','learned'))
);
