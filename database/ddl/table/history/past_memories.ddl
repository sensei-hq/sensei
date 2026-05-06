set search_path to history, sensei, extensions;

create table if not exists past_memories (
  id                       uuid          primary key default gen_random_uuid()
, memory_id                uuid          not null
, project_id               uuid
, scope                    text          not null
, scope_filter             text
, type                     text          not null
, title                    text          not null
, content                  text          not null
, impact                   text
, strength                 real          not null
, status                   text          not null
, session_id               uuid
, operation                dml_operation not null
, effective_from           timestamptz   not null
, effective_to             timestamptz
, changed_at               timestamptz   not null default now()
);

create index if not exists past_memories_memory_id_idx
    on past_memories(memory_id);

comment on table past_memories is
'History table for sensei.memories — auto-populated by historize_memories trigger.
Captures every INSERT, UPDATE, and DELETE so that memory evolution can be audited.

- memory_id: references sensei.memories.id with no FK — survives hard deletes
- effective_from/to: when this revision was the live state
- operation: INSERT, UPDATE, or DELETE';

comment on column past_memories.id
     is 'Surrogate primary key (UUID).';
comment on column past_memories.memory_id
     is 'References sensei.memories.id. No FK — survives hard deletes.';
comment on column past_memories.operation
     is 'DML that produced this entry: INSERT, UPDATE, or DELETE.';
comment on column past_memories.effective_from
     is 'When this revision became the live state.';
comment on column past_memories.effective_to
     is 'When this revision was superseded. Null if still current or if source was deleted.';
comment on column past_memories.changed_at
     is 'When this history entry was written.';
