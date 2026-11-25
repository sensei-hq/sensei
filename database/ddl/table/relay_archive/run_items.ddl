set search_path to relay_archive, extensions;

-- Run membership of ended runs (#393): same columns as relay.run_items plus
-- archived_at; same primary key; no foreign keys.
create table if not exists relay_archive.run_items (
  user_id    uuid        not null,
  run_id     uuid        not null,
  item_id    uuid        not null,
  position   int         not null default 0,
  added_at   timestamptz not null default current_timestamp,
  archived_at timestamptz not null default current_timestamp,
  constraint run_items_pkey primary key (user_id, run_id, item_id)
);

create index if not exists run_items_archived_at_idx
  on relay_archive.run_items (archived_at);

alter table relay_archive.run_items enable row level security;

comment on table relay_archive.run_items is
'Backlog items on ended runs, in pick order. Moved here at run completion; purged past 30 days.';
comment on column relay_archive.run_items.archived_at is 'When the row moved here; the purge horizon counts from it.';
