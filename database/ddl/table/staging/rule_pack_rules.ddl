set search_path to staging, extensions;

-- Staging landing for the rule-pack library RULES (import/staging/rule_pack_rules.jsonl).
-- Rows reference their pack by `pack_slug` (the datafile can't know the generated
-- pack uuid); import_rule_pack_rules() resolves slug → sensei.rule_packs.id (global
-- packs only) and upserts on (pack_id, ordinal) with a strict `modified_at` guard.
drop table if exists rule_pack_rules cascade;
create table rule_pack_rules (
  pack_slug     text
, ordinal       integer
, statement     text
, body          text
, rationale     text
, enforcement   text        -- cast to sensei.enforcement on import (per-rule tier)
, verification  text        -- cast to sensei.rule_check on import
, checker_ref   text
, remediation   text
, skill_ref     text
, applies_to    jsonb       default '{}'::jsonb
, evidence      jsonb       default '{}'::jsonb
, modified_at   timestamptz not null default now()
);
