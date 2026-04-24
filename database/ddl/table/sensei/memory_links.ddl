set search_path to sensei, extensions;

create table if not exists memory_links (
  parent_id                uuid          not null references sensei.memories(id) on delete cascade
, child_id                 uuid          not null references sensei.memories(id) on delete cascade
, modified_at              timestamptz   not null default now()
, primary key (parent_id, child_id)
);

create index if not exists memory_links_child_id_idx
    on memory_links(child_id);

comment on table memory_links is
'Memory-to-memory relationships for consolidation.
When multiple memories are combined into one, the combined memory becomes the parent
and the originals become children. Children are then historized into past_memories.

- parent_id: the consolidated/surviving memory
- child_id: the original memory that was merged in';

comment on column memory_links.parent_id
     is 'The consolidated memory that replaced the children.';
comment on column memory_links.child_id
     is 'An original memory that was merged into the parent.';
comment on column memory_links.modified_at
     is 'Timestamp of the last modification to this row.';
