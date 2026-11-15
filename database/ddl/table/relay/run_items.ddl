set search_path to relay, extensions;

-- The backlog items a sprint run groups, in the order the person picked them.
-- An item may be on one run at a time only by convention (the daemon checks);
-- the key is (run, item) so the same item never doubles up on one run.
create table if not exists relay.run_items (
  user_id  uuid        not null,
  run_id   uuid        not null,
  item_id  uuid        not null,
  position int         not null default 0,
  added_at timestamptz not null default current_timestamp,
  constraint run_items_pkey primary key (user_id, run_id, item_id),
  constraint run_items_run_fk foreign key (user_id, run_id) references relay.runs (user_id, id) on delete cascade,
  constraint run_items_item_fk foreign key (user_id, item_id) references relay.items (user_id, id) on delete cascade
);

alter table relay.run_items enable row level security;

comment on table relay.run_items is
'Backlog items on a sprint run, in pick order. Cascades with the run and with the item.';
comment on column relay.run_items.position is 'Pick order within the run, 0-based.';
