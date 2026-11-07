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
, owner_ref        text
, owner_state      tracker_owner_state
, owner_error      tracker_error
, writeback_enabled boolean      not null default false
, writeback_error  tracker_error
, estimation_field text
, points_field     text
, sprint_field     text
, history_error    tracker_error
, history_at       timestamptz
, terms_at         timestamptz
, created_at       timestamptz   not null default now()
, unique (kind, base_url, board_ref)
);

comment on table trackers is
'An issue-tracker board connected to a sensei workspace at a level (epic, sprint, task): a workspace may hold several, and a board belongs to one workspace. Trackers of one workspace on one site share its credential (D14). Mirror source for sensei.sprints / sensei.tracker_tasks; writes back usage and SDLC metrics only, to tickets assigned to the token owner (spec tracker-metrics-writeback). The API token is never stored here; secret_service names its keyring entry. Deleting a row (or its workspace) tombstones that entry via the tracker_secret_tombstone trigger.';

comment on column trackers.workspace_id
     is 'The sensei workspace this board belongs to. ON DELETE CASCADE, and the delete trigger still tombstones the secret.';
comment on column trackers.level
     is 'What the board serves in the workspace (D40): epic (portfolio), sprint (a team board, one Sprints lane), task (linked from changes). Rows from before levels are sprint trackers.';
comment on column trackers.kind
     is 'Which adapter speaks to this tracker (crate::tracker::adapter_for_kind).';
comment on column trackers.base_url
     is 'Adapter-normalised origin, exactly "https://" + lowercase host, no port/path/slash (crate::tracker::http::normalize_base_url). Part of the board identity.';
comment on column trackers.account
     is 'The account the token belongs to, as the adapter defines it (Jira: the Atlassian email; GitLab: the token''s username). Keyring account of secret_service.';
comment on column trackers.external_scope
     is 'The provider scope shown to users (Jira: the board''s project key, or the board name for a multi-project board; GitLab: the board''s full path at connect).';
comment on column trackers.board_ref
     is 'The board, in the adapter''s canonical form (Jira: decimal board id, no leading zeros; GitLab: project:<id> or group:<id>) so one board cannot be connected twice under two spellings.';
comment on column trackers.params
     is 'Adapter-specific extras (e.g. board_name; GitLab: full_path, version, sprint_source, sprint_source_changed_at). Never secrets.';
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
comment on column trackers.owner_ref
     is 'The token owner''s id at the provider (Jira: accountId of /myself, a person only). Compared with each ticket''s assignee to set tracker_tasks.assigned_to_me; never an assignee''s id. NULL until known (owner_state says why). Spec tracker-metrics-writeback W1.';
comment on column trackers.owner_state
     is 'Why owner_ref is NULL: unknown (asked again each sync) or not_a_person (an app token; never asked again until a re-connect). NULL when owner_ref is set, or for a tracker not yet asked.';
comment on column trackers.owner_error
     is 'Failure code of the last owner lookup (W1 backfill); NULL after a successful one. Kept apart from last_error: the read sync never fails because of the lookup.';
comment on column trackers.writeback_enabled
     is 'Whether sensei writes usage and SDLC metrics back to tickets assigned to the token owner (spec tracker-metrics-writeback W8). Off by default: a workspace opts in (D50, 2026-11-03, amending D3); existing boards keep their setting.';
comment on column trackers.writeback_error
     is 'Why the last write-back pass could not run or ended early (W2): token_missing / credential_store, db, rate_limited, transport. NULL after a pass that ran. Written only by the pass; never last_error.';
comment on column trackers.created_at
     is 'When the connection was first made.';

comment on column trackers.estimation_field
     is 'The board''s story-point field id (customfield_N), learned by the history task from the board configuration; NULL when the board estimates by count or not at all (spec tracker-sdlc-metrics S4, A6).';
comment on column trackers.points_field
     is 'The estimation field the last sync read points under (its id, or none when it cleared them). Points are trusted only when it matches estimation_field (spec tracker-sdlc-metrics S4, A6); NULL until a sync asked.';
comment on column trackers.sprint_field
     is 'The id of the site''s Sprint custom field (schema.custom gh-sprint), learned by the history task; changelog sprint items are matched by it (spec tracker-sdlc-metrics S4, A6). NULL when the site has none.';
comment on column trackers.history_error
     is 'Why the last history run stopped early (S4); NULL when it completed or ran out of time cleanly. Never touches last_error.';
comment on column trackers.terms_at
     is 'When the history task last read the board''s terms (estimation field, Sprint field, statuses). NULL until a run read them: until then estimation_field means nothing and synced points are kept (spec tracker-sdlc-metrics S4, A6).';
comment on column trackers.history_at
     is 'When the history task last ran.';
