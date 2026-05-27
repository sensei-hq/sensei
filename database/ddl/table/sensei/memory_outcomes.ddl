set search_path to sensei, extensions;

create table if not exists memory_outcomes (
  id            uuid              primary key default gen_random_uuid()
, memory_id     uuid              not null references sensei.memories(id) on delete cascade
, session_id    uuid              references activity.sessions(id) on delete set null
, outcome       memory_outcome    not null
, context       text
, recorded_at   timestamptz       not null default now()
);

create index if not exists memory_outcomes_memory_id_idx
    on memory_outcomes(memory_id, recorded_at desc);

comment on table memory_outcomes is
'Per-memory event log: applied/consulted/violated/ignored.
Insert triggers update memories.reinforced_count / violated_count / strength / status.';

comment on column memory_outcomes.memory_id
     is 'Foreign key to memories — which memory this event is about.';
comment on column memory_outcomes.session_id
     is 'Foreign key to activity.sessions. Null when unknown or after the session is deleted.';
comment on column memory_outcomes.outcome
     is 'What happened: applied (used in output), consulted (loaded but not used), violated (user overruled), ignored (loaded but discarded).';
comment on column memory_outcomes.context
     is 'Optional free-form note (e.g. file path, brief reason).';
comment on column memory_outcomes.recorded_at
     is 'When the event was recorded (server clock).';
