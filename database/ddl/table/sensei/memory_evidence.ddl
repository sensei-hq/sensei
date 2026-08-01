set search_path to sensei, activity, extensions;

create table if not exists memory_evidence (
  id                       uuid          primary key default gen_random_uuid()
, memory_id                uuid          not null references sensei.memories(id) on delete cascade
, session_id               uuid
, note                     text
, modified_at              timestamptz   not null default now()
);

create index if not exists memory_evidence_memory_id_idx
    on memory_evidence(memory_id);

comment on table memory_evidence is
'Evidence that proves or reinforces a memory. Each row is one piece — a session
where the memory was learned/confirmed/contradicted, OR a source reference
supplied at save time (file:line, test name, run id). Used for strength scoring
and provenance.

- session_id: references activity.sessions.id (nullable). No FK — sessions may be in
  a separate schema/DB. Null when the evidence is a save-time source note, not a session.
- note: what happened / the source (e.g. "user corrected indentation twice", "crates/x.rs:42").';

comment on column memory_evidence.id
     is 'Surrogate primary key (UUID).';
comment on column memory_evidence.memory_id
     is 'Foreign key to memories — which memory this evidence supports.';
comment on column memory_evidence.session_id
     is 'References activity.sessions.id (nullable). No FK — cross-schema reference. Null for a save-time source note.';
comment on column memory_evidence.note
     is 'What happened: correction, reinforcement, or contradiction.';
comment on column memory_evidence.modified_at
     is 'Timestamp of the last modification to this row.';
