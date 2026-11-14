set search_path to relay, extensions;

-- The queue, crew-global: manual items plus tracker references. A ticketed
-- item snapshots title/kind at + time; the screen reads the live values from
-- the tracker, which is also where its tags and sprint live.
create table if not exists relay.items (
  user_id          uuid             not null,
  id               uuid             not null,
  workspace_id     uuid,
  repo_key         text,
  ticket_provider  text,
  ticket_key       text,
  title            text             not null,
  kind             relay.item_kind  not null,
  tags             text[],
  state            relay.item_state not null,
  position         int,
  added_by         uuid,
  added_at         timestamptz      not null default current_timestamp,
  constraint items_pkey primary key (user_id, id),
  constraint items_id_v7 check (substring(id::text from 15 for 1) = '7'),
  constraint items_ticket_atomic check ((ticket_provider is null) = (ticket_key is null)),
  constraint items_tags_manual check (ticket_provider is null or tags is null),
  constraint items_adder_fk foreign key (user_id, added_by) references relay.lanes (user_id, id)
);

alter table relay.items enable row level security;

comment on table relay.items is
'The triage/backlog queue. Ticket identity is atomic: both halves or neither.';
comment on column relay.items.ticket_provider is 'jira/ado/github, free text so providers extend without a migration.';
comment on column relay.items.tags is 'Manual items only. Null for tickets — and for untagged manual items.';
comment on column relay.items.position is 'Backlog order; the coordinator takes the lowest next. Null until ranked.';
comment on column relay.items.added_by is 'The lane that added it; null when the person did (the + button).';
