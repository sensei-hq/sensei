set search_path to sensei, extensions;

-- Where each shared thing stands with dōjō.
--
-- An UPSERT KEYED TABLE, not a queue. A queue of pending sync jobs grows without
-- bound and needs its own retention — the same trap `task_executions` fell into,
-- reaching 4.8M rows before anyone looked. Here there is exactly one row per
-- (entity, key, direction) and it is overwritten, so the table's size is the size
-- of the shared surface rather than a function of how often sync runs.
--
-- Both directions are tracked separately because they fail independently: a push
-- can be rejected by governance while pulls stay healthy, and one row per pair
-- would hide whichever failed second.
create table if not exists sync_state (
  entity          sync_entity    not null
  -- The entity's DURABLE key, not a local uuid: repo_key for a repository,
  -- metric key for the catalogue. A local uuid differs between installs, so a
  -- sync row keyed on one could never be reconciled against the other side.
, entity_key      text           not null
, direction       sync_direction not null
, local_version   bigint
, remote_version  bigint
, state           text           not null default 'pending'
      check (state in ('pending', 'synced', 'error', 'skipped'))
  -- Kept, not cleared on the next attempt: a sync that fails, retries and fails
  -- differently is a different problem, and overwriting loses the first cause.
, last_error      text
, attempted_at    timestamptz
, synced_at       timestamptz
, updated_at      timestamptz    not null default now()
, primary key (entity, entity_key, direction)
);

create index if not exists sync_state_pending_idx
    on sync_state(entity, direction) where state <> 'synced';

comment on table sync_state is
'One row per (entity, key, direction) describing where that thing stands with
dōjō. Upserted, never appended — the table stays the size of the shared surface
instead of growing with every sync run.

`entity_key` is the durable cross-install key (repo_key, metric key), never a
local uuid: uuids differ per machine, so a row keyed on one could not be
reconciled against the other side.';

comment on column sync_state.state
     is 'pending → not yet reconciled; synced → both sides agree; error → last attempt failed (see last_error); skipped → deliberately not synced (private repo, deactivated metric).';
