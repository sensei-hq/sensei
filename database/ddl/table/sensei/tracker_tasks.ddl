set search_path to sensei, extensions;

-- A tracker's tickets, mirrored from the tracker. Spec: tracker-jira-linkage.md R2, R5.
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
, assigned_to_me     boolean
, writeback_hash     text
, writeback_comment_ref text
, writeback_status   tracker_writeback_status
, writeback_at       timestamptz
, writeback_tried_at timestamptz
, unique (tracker_id, external_id)
);

create index if not exists tracker_tasks_tracker_id_key_idx
    on tracker_tasks(tracker_id, key);

comment on table tracker_tasks is
'Tickets (issues) of a tracker board, mirrored from the tracker; usage is written back to the token owner''s (spec tracker-metrics-writeback). Keyed on the provider''s immutable issue id so a renamed key updates in place. A ticket missing from a full sync gets removed_at; its session links survive. No assignee identity: only whether the token owner is the assignee (assigned_to_me).';

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
comment on column tracker_tasks.assigned_to_me
     is 'Whether the ticket''s assignee is the tracker''s owner (trackers.owner_ref), set on every upsert; false when unassigned or someone else''s, NULL while the owner is unknown. The assignee itself is compared, never stored. Spec tracker-metrics-writeback W1.';
comment on column tracker_tasks.writeback_hash
     is 'SHA-256 of what write-back last wrote (the comment lines, or the mapped field values), so an unchanged ticket sends nothing (W4). NULL until a write lands.';
comment on column tracker_tasks.writeback_comment_ref
     is 'The provider id of the comment sensei writes this ticket''s usage into (W5). Kept with comment_removed; cleared by a re-connect or by switching write-back back on.';
comment on column tracker_tasks.writeback_status
     is 'The last write-back outcome (W9); NULL when never attempted.';
comment on column tracker_tasks.writeback_at
     is 'When a write-back last landed on this ticket.';
comment on column tracker_tasks.writeback_tried_at
     is 'When write-back last tried this ticket, landed or not: the pass order (oldest first, never-tried first), so a ticket that keeps failing never starves the others (W2).';
