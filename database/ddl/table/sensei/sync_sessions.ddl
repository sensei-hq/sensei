set search_path to sensei, extensions;

create table if not exists sync_sessions (
  id               uuid        primary key default gen_random_uuid()
, account_id       uuid        not null references core.accounts(id) on delete cascade
, session_id       text        not null
, repo_slug        text        not null
, stack            text[]
, ftr_score        numeric(5,3)
, token_cost       numeric(12,6)
, duration_ms      int
, tool_call_count  int         not null default 0
, error_count      int         not null default 0
, snapshot_count   int         not null default 0
, completed_cleanly boolean    not null default false
, recorded_at      timestamptz not null
, received_at      timestamptz not null default now()
, modified_at      timestamptz not null default now()
, modified_by      text        not null default current_user
, unique (account_id, session_id)
);

create index if not exists sync_sessions_account_idx     on sync_sessions(account_id);
create index if not exists sync_sessions_recorded_at_idx on sync_sessions(recorded_at);

comment on table sync_sessions is
'PII-scrubbed session summaries synced from the collector to the platform.
- session_id: opaque client-generated ID (not the local UUID)
- repo_slug: SHA-256 hash of account_id + repo name — no plaintext repo info stored
- received_at: server-side receipt timestamp; recorded_at is the client-side session end time';

comment on column sync_sessions.id is 'Surrogate primary key (UUID).';
comment on column sync_sessions.account_id is 'Foreign key to core.accounts — identifies the account that submitted this session.';
comment on column sync_sessions.session_id is 'Opaque client-generated session identifier (not a local UUID); unique per account.';
comment on column sync_sessions.repo_slug is 'SHA-256 hash of account_id + repo name, used to group sessions without storing plaintext repo info.';
comment on column sync_sessions.stack is 'Technology stack detected for the repo during this session (e.g. [''typescript'', ''react'']).';
comment on column sync_sessions.ftr_score is 'First-Try-Right score for this session (0.000–1.000).';
comment on column sync_sessions.token_cost is 'Estimated total token cost in USD for this session.';
comment on column sync_sessions.duration_ms is 'Wall-clock duration of the session in milliseconds.';
comment on column sync_sessions.tool_call_count is 'Total number of tool calls made during this session.';
comment on column sync_sessions.error_count is 'Number of errors encountered during this session.';
comment on column sync_sessions.snapshot_count is 'Number of progress snapshots recorded during this session.';
comment on column sync_sessions.completed_cleanly is 'Whether the session ended with an explicit checkpoint call (true) or was abandoned (false).';
comment on column sync_sessions.recorded_at is 'Client-side timestamp of when the session ended.';
comment on column sync_sessions.received_at is 'Server-side timestamp of when the platform received this session record.';
comment on column sync_sessions.modified_at is 'Timestamp of the last modification to this row.';
comment on column sync_sessions.modified_by is 'Identity (user, role, or service) that last modified this row.';
