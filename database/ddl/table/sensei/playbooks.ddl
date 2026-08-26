set search_path to sensei, extensions;
create table if not exists playbooks (
  name          text        primary key
, title         text        not null
, when_to_use   text        not null
, opening_tone  text        not null
, method_ref    text
, enabled       boolean     not null default true
-- `source_kind`, not text+CHECK: the same three values were duplicated as a
-- separate constraint on sensei.playbooks. See ddl/enum/sensei/source_kind.ddl.
, source        source_kind        not null default 'builtin'
, created_at    timestamptz not null default now()
-- Bumped whenever the row changes. The staging import guards on it: a re-import
-- only overwrites a row when the DATAFILE's stamp is at least as new, so an edit
-- made in place survives redeployment. Without this column the import had nothing
-- to compare and fell back to clobbering (playbooks) or to never updating at all
-- (intake_guide, playbook_rules) — see docs/backlog.md.
, modified_at   timestamptz not null default now()
  -- Shared vocabulary (see sensei.entity_scope / sensei.entity_origin).
  -- scope answers WHO MAY SEE IT and therefore whether it syncs; origin answers
  -- WHERE IT CAME FROM and therefore what a re-import may safely replace.
, scope         entity_scope  not null default 'local'
, origin        entity_origin not null default 'builtin'
);
