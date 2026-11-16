set search_path to relay, extensions;

-- A sprint run: backlog items grouped under a goal by the person. A run is
-- drafted with its items and no crew (#419); starting it binds the crew and
-- dispatches Task 0 (phase planning) into the queue; a peer-approved plan
-- unpacks into ready tasks and the run turns active; it completes itself when
-- every task is done. The crew is where the lanes are; the run is what they
-- are working through.
create table if not exists relay.runs (
  user_id      uuid            not null,
  id           uuid            not null,
  crew_id      uuid,
  title        text            not null,
  goal         text            not null,
  state        relay.run_state not null default 'draft',
  repo_key     text,
  workspace_id uuid,
  config       jsonb           not null default '{}',
  created_at   timestamptz     not null default current_timestamp,
  started_at   timestamptz,
  completed_at timestamptz,
  constraint runs_pkey primary key (user_id, id),
  constraint runs_id_v7 check (substring(id::text from 15 for 1) = '7'),
  constraint runs_crew_fk foreign key (user_id, crew_id) references relay.crews (user_id, id)
);

create index if not exists runs_crew_open_idx
  on relay.runs (user_id, crew_id) where state in ('planning', 'active');

alter table relay.runs enable row level security;

comment on table relay.runs is
'Sprint runs. Minted by the person from backlog items; Task 0 plans, the approved plan unpacks, the run completes when every task is done.';
comment on column relay.runs.goal is 'The sprint goal in the person''s words: what Task 0 decomposes.';
comment on column relay.runs.config is 'Gate inputs and knobs: base_checkout, base_branch, verify_cmd, auto_merge. Empty until the person sets them.';
comment on column relay.runs.started_at is 'When the plan unpacked (planning → active). Null while planning.';
comment on column relay.runs.completed_at is 'When the last task reached done. Null until then.';
comment on column relay.runs.crew_id is 'The crew it runs in: null while a draft, bound at start (#419).';
comment on column relay.runs.repo_key is 'The repository Task 0 plans in: the person''s, else the first item''s; else the crew''s at start.';
comment on column relay.runs.workspace_id is 'The workspace, when known: the person''s, else the first item''s; else the crew''s at start.';
