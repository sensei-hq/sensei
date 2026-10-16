set search_path to sensei, extensions;

-- One issue-tracker connection (a board) per sensei project, and one project
-- per board. Spec: docs/spec/daemon/tracker-jira-linkage.md R1-R3.
create table if not exists trackers (
  id               uuid          primary key default gen_random_uuid()
, project_id       uuid          not null references sensei.projects(id) on delete cascade
, kind             tracker_kind  not null
, base_url         text          not null
, account          text          not null
, external_scope   text          not null
, board_ref        text          not null
, params           jsonb         not null default '{}'
, secret_service   text          not null
, last_attempt_at  timestamptz
, last_success_at  timestamptz
, last_error       tracker_error
, created_at       timestamptz   not null default now()
, unique (project_id)
, unique (kind, base_url, board_ref)
);

comment on table trackers is
'An issue-tracker board connected to a sensei project: one tracker per project (one sprint calendar) and one project per board. Read-only mirror source for sensei.sprints / sensei.tracker_tasks. The API token is never stored here; secret_service names its keyring entry. Deleting a row (or its project) tombstones that entry via the tracker_secret_tombstone trigger.';

comment on column trackers.project_id
     is 'The sensei project this board belongs to. Unique: a project has exactly one tracker. ON DELETE CASCADE, and the delete trigger still tombstones the secret.';
comment on column trackers.kind
     is 'Which adapter speaks to this tracker (crate::tracker::adapter_for_kind).';
comment on column trackers.base_url
     is 'Adapter-normalised origin, exactly "https://" + lowercase host, no port/path/slash (crate::tracker::http::normalize_base_url). Part of the board identity.';
comment on column trackers.account
     is 'The account the token belongs to, as the adapter defines it (Jira: the Atlassian email). Keyring account of secret_service.';
comment on column trackers.external_scope
     is 'The provider scope shown to users (Jira: the board''s project key, or the board name for a multi-project board).';
comment on column trackers.board_ref
     is 'The board, in the adapter''s canonical form (Jira: decimal board id, no leading zeros) so one board cannot be connected twice under two spellings.';
comment on column trackers.params
     is 'Adapter-specific extras (e.g. board_name). Never secrets.';
comment on column trackers.secret_service
     is 'Keyring service of the API token, com.sensei.tracker.<kind>.<credential uuid>; a fresh uuid on every connect, so an entry is never overwritten in place. Changing it tombstones the old value.';
comment on column trackers.last_attempt_at
     is 'When a sync last started (written before the fetch, outside any transaction).';
comment on column trackers.last_success_at
     is 'When a sync last committed in full. NULL = never (status pending), also reset by a re-connect.';
comment on column trackers.last_error
     is 'Failure code of the last sync or connect; NULL after a success. Never a raw message.';
comment on column trackers.created_at
     is 'When the connection was first made.';
