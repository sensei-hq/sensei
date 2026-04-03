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
, unique (account_id, session_id)
);

create index if not exists sync_sessions_account_idx     on sync_sessions(account_id);
create index if not exists sync_sessions_recorded_at_idx on sync_sessions(recorded_at);

comment on table sync_sessions is
'PII-scrubbed session summaries synced from the collector to the platform.
- session_id: opaque client-generated ID (not the local UUID)
- repo_slug: SHA-256 hash of account_id + repo name — no plaintext repo info stored
- received_at: server-side receipt timestamp; recorded_at is the client-side session end time';
