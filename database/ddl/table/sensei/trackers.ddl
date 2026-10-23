set search_path to sensei, extensions;

-- Issue-tracker connections (boards) of a sensei workspace, each at a level, and
-- one workspace per board. Spec: docs/spec/daemon/tracker-jira-linkage.md R1-R3,
-- docs/spec/tracker-levels.md (D40).
create table if not exists trackers (
  id               uuid          primary key default gen_random_uuid()
, workspace_id     uuid          not null references sensei.workspaces(id) on delete cascade
, kind             tracker_kind  not null
, level            tracker_level not null default 'sprint'
, base_url         text          not null
, account          text          not null
, external_scope   text          not null
, board_ref        text          not null
, params           jsonb         not null default '{}'
, secret_service   text          not null
, last_attempt_at  timestamptz
, last_success_at  timestamptz
, last_error       tracker_error
, discovered_through timestamptz not null default now()
, created_at       timestamptz   not null default now()
, unique (kind, base_url, board_ref)
);

comment on table trackers is
'An issue-tracker board connected to a sensei workspace at a level (epic, sprint, task): a workspace may hold several, and a board belongs to one workspace. Trackers of one workspace on one site share its credential (D14). Read-only mirror source for sensei.sprints / sensei.tracker_tasks. The API token is never stored here; secret_service names its keyring entry. Deleting a row (or its workspace) tombstones that entry via the tracker_secret_tombstone trigger.';

comment on column trackers.workspace_id
     is 'The sensei workspace this board belongs to. ON DELETE CASCADE, and the delete trigger still tombstones the secret.';
comment on column trackers.level
     is 'What the board serves in the workspace (D40): epic (portfolio), sprint (a team board, one Sprints lane), task (linked from changes). Rows from before levels are sprint trackers.';
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
     is 'Keyring service of the API token, com.sensei.tracker.<kind>.<credential uuid>; a fresh uuid on every connect, so an entry is never overwritten in place. Shared by every tracker of the workspace on the same (kind, base_url): a connect re-keys them all. Changing it tombstones the old value; the drain keeps a secret another tracker still references.';
comment on column trackers.last_attempt_at
     is 'When a sync last started (written before the fetch, outside any transaction).';
comment on column trackers.last_success_at
     is 'When a sync last committed in full. NULL = never (status pending), also reset by a re-connect.';
comment on column trackers.last_error
     is 'Failure code of the last sync or connect; NULL after a success. Never a raw message.';
comment on column trackers.discovered_through
     is 'R11 watermark: transcript turns ingested up to here have been checked for ticket keys the tracker does not have yet. Starts at connect time; the first full sync covers what came before.';
comment on column trackers.created_at
     is 'When the connection was first made.';
