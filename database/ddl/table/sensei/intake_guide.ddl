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
-- Bumped whenever the row changes. The staging import guards on it: a re-import
-- only overwrites a row when the DATAFILE's stamp is at least as new, so an edit
-- made in place survives redeployment. Without this column the import had nothing
-- to compare and fell back to clobbering (playbooks) or to never updating at all
-- (intake_guide, playbook_rules) — see docs/backlog.md.
, modified_at  timestamptz not null default now()
, constraint intake_guide_kind_chk check (kind in ('frame','axis'))
, constraint intake_guide_axis_chk check ((kind='axis') = (axis is not null))
, constraint intake_guide_source_chk check (source in ('builtin','org','learned'))
  -- Shared vocabulary (see sensei.entity_scope / sensei.entity_origin).
  -- scope answers WHO MAY SEE IT and therefore whether it syncs; origin answers
  -- WHERE IT CAME FROM and therefore what a re-import may safely replace.
, scope         entity_scope  not null default 'local'
, origin        entity_origin not null default 'builtin'
);

-- The natural key, so the staging import can upsert on it.
--
-- NULLS NOT DISTINCT because `axis` is optional and the default treats every
-- NULL as unique — which would let the same (kind, NULL) row be inserted over
-- and over, one per deploy, with nothing flagging it.
create unique index if not exists intake_guide_kind_axis_unique
    on sensei.intake_guide (kind, axis) nulls not distinct;
