set search_path to sensei, extensions;
create table if not exists playbook_rules (
  id               uuid            primary key default gen_random_uuid()
, name             text            not null
, match_lifecycle  chunk_lifecycle
, match_intent     chunk_intent
, match_risk       chunk_risk
, playbook         text            not null references sensei.playbooks(name)
, rationale        text            not null
, priority         integer         not null
, base_priority   integer
, enabled          boolean         not null default true
-- `source_kind`, not text+CHECK: the same three values were duplicated as a
-- separate constraint on sensei.playbooks. See ddl/enum/sensei/source_kind.ddl.
, source           source_kind            not null default 'builtin'
, created_at       timestamptz     not null default now()
-- Bumped whenever the row changes. The staging import guards on it: a re-import
-- only overwrites a row when the DATAFILE's stamp is at least as new, so an edit
-- made in place survives redeployment. Without this column the import had nothing
-- to compare and fell back to clobbering (playbooks) or to never updating at all
-- (intake_guide, playbook_rules) — see docs/backlog.md.
, modified_at      timestamptz     not null default now()
);
create index if not exists playbook_rules_match_idx on playbook_rules(enabled, priority desc);
-- Covering index for the playbook FK (→ playbooks.name) — the learned unique index
-- has `playbook` as its 4th column, so it does not cover this FK on its own.
create index if not exists playbook_rules_playbook_idx on playbook_rules(playbook);
create unique index if not exists playbook_rules_learned_uq
    on playbook_rules(match_lifecycle, match_intent, match_risk, playbook)
    where source = 'learned';

-- The natural key, so the staging import can upsert on it. A rule is addressed
-- by name everywhere else; without this the import could only ever insert, and
-- an edited rule in the datafile silently never reached the table.
create unique index if not exists playbook_rules_name_unique
    on sensei.playbook_rules (name);
