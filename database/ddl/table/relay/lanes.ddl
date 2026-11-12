set search_path to relay, extensions;

-- A registered lane: one assistant session in a crew. Codes are unique per
-- crew only (L1 here is not L1 there); at most one registered coordinator per
-- crew, and none required: the broker coordinates.
create table if not exists relay.lanes (
  user_id         uuid            not null,
  id              uuid            not null,
  crew_id         uuid            not null,
  code            text            not null,
  title           text,
  suggested_name  text,
  role            relay.lane_role not null default 'contributor',
  family          text            not null,
  version         text,
  model           text,
  capabilities    text[]          not null default '{}',
  repo            text,
  worktree        text,
  token_hash      text            not null,
  checkpoint      jsonb,
  registered_at   timestamptz     not null default current_timestamp,
  unregistered_at timestamptz,
  limit_paused_at timestamptz,
  limit_scope     relay.limit_scope,
  limit_resumes_at timestamptz,
  limit_note      text,
  constraint lanes_pkey primary key (user_id, id),
  constraint lanes_pause_coherent check (
    (limit_paused_at is null) = (limit_scope is null)),
  constraint lanes_id_v7 check (substring(id::text from 15 for 1) = '7'),
  constraint lanes_crew_fk foreign key (user_id, crew_id) references relay.crews (user_id, id),
  constraint lanes_code_per_crew unique (user_id, crew_id, code)
);

create unique index if not exists lanes_one_coordinator
  on relay.lanes (user_id, crew_id) where role = 'coordinator' and unregistered_at is null;

alter table relay.lanes enable row level security;

comment on table relay.lanes is
'Registered lanes. Liveness is the lane''s lease, not a column.';
comment on column relay.lanes.code is 'L1.., unique per crew. The person reads codes, never uuids.';
comment on column relay.lanes.title is 'Set by the coordinator''s first assign; null until then.';
comment on column relay.lanes.suggested_name is 'The name the assistant proposed at register (worker-alpha). Null when it offered none; the code stays the handle.';
comment on column relay.lanes.role is 'contributor by default: the broker coordinates. A coordinator lane is optional and one per crew while registered.';
comment on column relay.lanes.capabilities is 'What the lane declared at register. Empty, never unknown.';
comment on column relay.lanes.repo is 'The lane''s checkout. Null when it has none yet (a coordinator minted without a repo).';
comment on column relay.lanes.worktree is 'The broker-created worktree. Null until the broker makes it.';
comment on column relay.lanes.token_hash is 'sha256 hex of the lane token. The token itself is never stored.';
comment on column relay.lanes.checkpoint is 'Where the lane was, for resume after interrupt. Null until set.';
comment on column relay.lanes.unregistered_at is 'Null until the lane leaves. Rows persist: history keeps them.';
comment on column relay.lanes.limit_paused_at is 'When the lane reported quota exhaustion. Null: not paused. Distinct from lease expiry: a paused lane may hold a live lease.';
comment on column relay.lanes.limit_scope is 'Which quota ran out. Set exactly when paused (lanes_pause_coherent).';
comment on column relay.lanes.limit_resumes_at is 'Quota-reset time from the warning. Null: unknown — any beat resumes.';
comment on column relay.lanes.limit_note is 'The lane''s own words on the pause. Audit, shown nowhere yet.';
