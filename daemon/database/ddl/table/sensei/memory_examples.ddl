set search_path to sensei, extensions;

create table if not exists memory_examples (
  id                       uuid          primary key default gen_random_uuid()
, memory_id                uuid          not null references sensei.memories(id) on delete cascade
, node_id                  text          not null
, is_good                  boolean       not null
, note                     text
, modified_at              timestamptz   not null default now()
);

create index if not exists memory_examples_memory_id_idx
    on memory_examples(memory_id);

comment on table memory_examples is
'Code examples grounding a memory in real code.
- node_id: references a hierarchy node (file, function, class, etc.)
- is_good: true = canonical implementation to follow, false = divergence to avoid
- note: why this example matters for this memory

No FK on node_id — nodes may be deleted when code changes, but the example record
stays as historical evidence. The note should describe the example independently.';

comment on column memory_examples.id
     is 'Surrogate primary key (UUID).';
comment on column memory_examples.memory_id
     is 'Foreign key to memories — which memory this example supports.';
comment on column memory_examples.node_id
     is 'References hierarchy_nodes.id. No FK — survives node deletion.';
comment on column memory_examples.is_good
     is 'True = good example (follow this), false = bad example (avoid this).';
comment on column memory_examples.note
     is 'Why this code is a good or bad example for this memory.';
comment on column memory_examples.modified_at
     is 'Timestamp of the last modification to this row.';
