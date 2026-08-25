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
  -- Shared vocabulary (see sensei.entity_scope / sensei.entity_origin).
  -- scope answers WHO MAY SEE IT and therefore whether it syncs; origin answers
  -- WHERE IT CAME FROM and therefore what a re-import may safely replace.
, scope         entity_scope  not null default 'local'
, origin        entity_origin not null default 'builtin'
);
