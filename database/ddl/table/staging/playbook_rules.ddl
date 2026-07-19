set search_path to staging, extensions;
drop table if exists playbook_rules cascade;
create table playbook_rules (
  name text, match_lifecycle text, match_intent text, match_risk text,
  playbook text, rationale text, priority integer, enabled boolean default true,
  source text default 'builtin', modified_at timestamptz not null default now()
);
