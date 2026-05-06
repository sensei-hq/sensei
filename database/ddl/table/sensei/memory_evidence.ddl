set search_path to sensei, activity, extensions;

create table if not exists memory_evidence (
  id                       uuid          primary key default gen_random_uuid()
, memory_id                uuid          not null references sensei.memories(id) on delete cascade
, session_id               uuid          not null
, note                     text
, modified_at              timestamptz   not null default now()
);

create index if not exists memory_evidence_memory_id_idx
    on memory_evidence(memory_id);

comment on table memory_evidence is
'Sessions that prove or reinforce a memory.
Each row is one piece of evidence — a session where the memory was learned,
confirmed, or contradicted. Used for strength scoring.

- session_id: references activity.sessions.id. No FK — sessions may be in a separate schema/DB.
- note: what happened in this session (e.g. "user corrected indentation twice")';

comment on column memory_evidence.id
     is 'Surrogate primary key (UUID).';
comment on column memory_evidence.memory_id
     is 'Foreign key to memories — which memory this evidence supports.';
comment on column memory_evidence.session_id
     is 'References activity.sessions.id. No FK — cross-schema reference.';
comment on column memory_evidence.note
     is 'What happened: correction, reinforcement, or contradiction.';
comment on column memory_evidence.modified_at
     is 'Timestamp of the last modification to this row.';
