set search_path to sensei, extensions;

-- Which memories are attached to which share batch. Normalised join table
-- rather than a UUID[] column on memory_share_batches so cascading a
-- memory deletion drops it from any batches it appears in and the
-- verdict-writer can iterate members with a plain JOIN.
create table if not exists memory_share_batch_members (
  batch_id   uuid  not null references sensei.memory_share_batches(id) on delete cascade
, memory_id  uuid  not null references sensei.memories(id)             on delete cascade
, added_at   timestamptz  not null default now()
, primary key (batch_id, memory_id)
);

create index if not exists memory_share_batch_members_memory_idx
    on memory_share_batch_members(memory_id);

comment on table memory_share_batch_members is
'Join between memory_share_batches and memories. One row per (batch,
memory) pair. Both foreign keys cascade on delete so removing a memory
or a batch cleanly clears the association.';

comment on column memory_share_batch_members.batch_id
     is 'FK to memory_share_batches — the batch this membership belongs to.';
comment on column memory_share_batch_members.memory_id
     is 'FK to memories — the memory being proposed for share via this batch.';
comment on column memory_share_batch_members.added_at
     is 'When the memory was attached to the batch (may differ from batch created_at if members are added later).';
