set search_path to sensei, extensions;

-- A third-party spec-driven-development framework a session worked in
-- (blueprint docs/blueprints/2026-11-10-ticket-sdd-framework.md).
create type sdd_framework as enum (
  'openspec'   -- OpenSpec: openspec/, /opsx:*, openspec-* skills
, 'spec_kit'   -- GitHub Spec Kit: .specify/, specs/<n>-*/, /speckit.*
, 'kiro'       -- Kiro: .kiro/specs/
, 'bmad'       -- BMAD Method: _bmad/, _bmad-output/, .bmad-core/, bmad-* skills
);

comment on type sdd_framework is
'Third-party spec-driven-development framework (sdlc::framework). sensei''s own plan graph is not one: a session that used only sensei has no framework (NULL).';
