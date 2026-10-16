set search_path to sensei, extensions;

-- Keyring entries that no tracker row references and that must be deleted. A
-- keyring cannot be listed, so this table is the only record of an entry to
-- clean up. Spec: docs/spec/daemon/tracker-jira-linkage.md R3.
create table if not exists tracker_secret_tombstones (
  secret_service  text         primary key
, account         text         not null
, not_before      timestamptz  not null
, created_at      timestamptz  not null default now()
);

create index if not exists tracker_secret_tombstones_not_before_idx
    on tracker_secret_tombstones(not_before);

comment on table tracker_secret_tombstones is
'Keyring entries pending deletion. Written by the tracker_secret_tombstone trigger for the OLD secret of a deleted or re-keyed tracker (due now), and by connect for the NEW secret before it is written (due after a grace period, deleted when the connect commits). Drained by the tracker_sync tick.';

comment on column tracker_secret_tombstones.secret_service
     is 'The keyring service to delete.';
comment on column tracker_secret_tombstones.account
     is 'The keyring account of that entry.';
comment on column tracker_secret_tombstones.not_before
     is 'Do not drain before this time. now() for an orphaned secret; now() + grace for a connect still in progress.';
comment on column tracker_secret_tombstones.created_at
     is 'When the tombstone was written.';
