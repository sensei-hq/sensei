set search_path to sensei, extensions;
create table if not exists playbook_run (
  id          uuid            primary key default gen_random_uuid()
, session_id  uuid            references activity.sessions(id) on delete set null
, feature     text
, lifecycle   chunk_lifecycle not null
, intent      chunk_intent    not null
, risk        chunk_risk      not null
, rule_id     uuid            references sensei.playbook_rules(id) on delete set null
, playbook    text            not null references sensei.playbooks(name)
, rationale   text            not null
, confirmed   boolean         not null default false
, outcome     text
, classified_by   text
, model_fallback  boolean     not null default false
, outcome_ftr boolean
, project_id  uuid            not null references projects(id) on delete cascade
, created_at  timestamptz     not null default now()
);
create index if not exists playbook_run_session_idx on playbook_run(session_id);
create index if not exists playbook_run_project_idx on playbook_run(project_id);
