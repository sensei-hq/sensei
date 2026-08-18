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
, source           text            not null default 'builtin'
, created_at       timestamptz     not null default now()
, constraint playbook_rules_source_chk check (source in ('builtin','org','learned'))
);
create index if not exists playbook_rules_match_idx on playbook_rules(enabled, priority desc);
-- Covering index for the playbook FK (→ playbooks.name) — the learned unique index
-- has `playbook` as its 4th column, so it does not cover this FK on its own.
create index if not exists playbook_rules_playbook_idx on playbook_rules(playbook);
create unique index if not exists playbook_rules_learned_uq
    on playbook_rules(match_lifecycle, match_intent, match_risk, playbook)
    where source = 'learned';
