set search_path to relay, extensions;

-- A crew: the person's unit of delegated work. Minted with the person, or by a
-- coordinator's first register (Q6: goal, repo and workspace all optional at
-- mint time). The repository gate never applies here: work on a workspace-less
-- repository must still appear, so the workspace stays optional down the stack.
create table if not exists relay.crews (
  user_id       uuid        not null,
  id            uuid        not null,
  workspace_id  uuid,
  repo_key      text,
  goal          text,
  status        text        not null,
  heartbeat_at  timestamptz,
  started_at    timestamptz not null default now(),
  completed_at  timestamptz,
  constraint crews_pkey primary key (user_id, id),
  constraint crews_id_v7 check (substring(id::text from 15 for 1) = '7')
);

alter table relay.crews enable row level security;

comment on table relay.crews is
'A crew of lanes. Upsert key (user_id, id); ids are uuid v7 (sync + time order).';
comment on column relay.crews.user_id is 'The owning person''s dōjō principal id: the RLS key.';
comment on column relay.crews.workspace_id is 'The workspace window, when the crew has one. Optional, always.';
comment on column relay.crews.repo_key is 'The founding repository (Q6). Tasks carry their own repo_key; a crew spans repos.';
comment on column relay.crews.goal is 'Why this crew exists, in the person''s words. Null until stated.';
comment on column relay.crews.status is 'Free string, daemon vocabulary (lanes-data-literal).';
comment on column relay.crews.heartbeat_at is 'Last proof any lane is alive. Null until the first heartbeat.';
comment on column relay.crews.completed_at is 'Null until the crew is done.';
