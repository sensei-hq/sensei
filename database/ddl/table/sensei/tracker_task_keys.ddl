set search_path to sensei, extensions;

-- Every key a ticket has ever had. A renamed or moved issue keeps its old keys
-- here, so sessions that wrote the old key still link to it.
-- Spec: docs/spec/daemon/tracker-jira-linkage.md R2, R5, R6.
create table if not exists tracker_task_keys (
  task_id        uuid         not null references sensei.tracker_tasks(id) on delete cascade
, key            text         not null
, first_seen_at  timestamptz  not null default now()
, primary key (task_id, key)
);

create index if not exists tracker_task_keys_key_idx
    on tracker_task_keys(key);

comment on table tracker_task_keys is
'Key aliases of tracker tickets: the current key plus every former one (rename, move). Written by each sync''s apply (on conflict do nothing), read when matching session text, so history survives a rename.';

comment on column tracker_task_keys.task_id
     is 'The ticket.';
comment on column tracker_task_keys.key
     is 'One key the ticket has had, in the adapter''s canonical form.';
comment on column tracker_task_keys.first_seen_at
     is 'When a sync first saw the ticket under this key.';
