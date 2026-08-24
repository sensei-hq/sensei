set search_path to sensei, extensions;
create table if not exists intake_guide (
  id          uuid        primary key default gen_random_uuid()
, kind        text        not null
, axis        text
, prompt      text        not null
, help        text
, enabled     boolean     not null default true
, source      text        not null default 'builtin'
, created_at  timestamptz not null default now()
, constraint intake_guide_kind_chk check (kind in ('frame','axis'))
, constraint intake_guide_axis_chk check ((kind='axis') = (axis is not null))
, constraint intake_guide_source_chk check (source in ('builtin','org','learned'))
  -- Shared vocabulary (see sensei.entity_scope / sensei.entity_origin).
  -- scope answers WHO MAY SEE IT and therefore whether it syncs; origin answers
  -- WHERE IT CAME FROM and therefore what a re-import may safely replace.
, scope         entity_scope  not null default 'local'
, origin        entity_origin not null default 'builtin'
);
