set search_path to sensei, extensions;

-- A tracker's tickets, mirrored read-only. Spec: tracker-jira-linkage.md R2, R5.
create table if not exists tracker_tasks (
  id                 uuid         primary key default gen_random_uuid()
, tracker_id         uuid         not null references sensei.trackers(id) on delete cascade
, external_id        text         not null
, key                text         not null
, scope              text
, summary            text
, issue_type         text
, status             text
, current_sprint_id  uuid         references sensei.sprints(id) on delete set null
, removed_at         timestamptz
, last_seen_at       timestamptz  not null default now()
, unique (tracker_id, external_id)
);

create index if not exists tracker_tasks_tracker_id_key_idx
    on tracker_tasks(tracker_id, key);

comment on table tracker_tasks is
'Tickets (issues) of a tracker board, mirrored read-only. Keyed on the provider''s immutable issue id so a renamed key updates in place. A ticket missing from a full sync gets removed_at; its session links survive. No assignee or other person data, by design.';

comment on column tracker_tasks.tracker_id
     is 'The board this ticket belongs to.';
comment on column tracker_tasks.external_id
     is 'The provider''s immutable issue id.';
comment on column tracker_tasks.key
     is 'The provider''s display key (Jira: AUTH-420, stored upper-case). Overwritten when the issue is renamed or moved.';
comment on column tracker_tasks.scope
     is 'What the key is relative to, when it is not globally unique (GitLab: normalize_repo_key of the project URL). NULL for Jira.';
comment on column tracker_tasks.summary
     is 'Issue title. Client-confidential: never leaves the machine.';
comment on column tracker_tasks.issue_type
     is 'Provider issue type, raw (Story, Bug, Task, ...). Not mapped in v1.';
comment on column tracker_tasks.status
     is 'Provider workflow status, raw.';
comment on column tracker_tasks.current_sprint_id
     is 'The sprint the ticket is in now: among non-removed sprints listing it, active > future > closed (crate::tracker::sprint::pick_current_sprint). NULL = backlog.';
comment on column tracker_tasks.removed_at
     is 'When a full sync stopped seeing it. Set once.';
comment on column tracker_tasks.last_seen_at
     is 'When a sync last saw it.';
