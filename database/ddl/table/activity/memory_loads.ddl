set search_path to activity, sensei, extensions;

create table if not exists memory_loads (
  id                 bigserial   primary key
, session_id         uuid        references activity.sessions(id) on delete set null
, client_session_id  text
, project_id         uuid        references sensei.projects(id) on delete set null
, memory_id          uuid        not null references sensei.memories(id) on delete cascade
, source             text        not null default 'get_layered_context'
, loaded_at          timestamptz not null default now()
);

create index if not exists memory_loads_memory_id_idx
    on memory_loads(memory_id, loaded_at desc);

create index if not exists memory_loads_session_idx
    on memory_loads(session_id, loaded_at desc);

-- Covering index for the project_id FK (nullable) — avoids a seq-scan of this
-- high-volume table when a project is deleted (on delete set null).
create index if not exists memory_loads_project_id_idx
    on memory_loads(project_id) where project_id is not null;

comment on table memory_loads is
'Per-memory load log — one row per (memory, delivery) event. Written when a
memory is injected into an assistant''s context (e.g. get_layered_context /
assemble_context). The read counterpart of sensei.memory_outcomes: loads here
plus applied/ignored outcomes there answer "did injected memory help?" over a
rolling window (loaded vs followed vs skipped). High write volume — bigserial
key. Insertion is non-fatal on the context-delivery hot path: a failure to log
must never block context delivery.
v1 logs with session_id NULL (per-memory COUNT needs no session join);
per-session correlation is a deferred follow-up.';

comment on column memory_loads.id
     is 'Surrogate primary key (bigserial — high write volume).';
comment on column memory_loads.session_id
     is 'Optional FK to activity.sessions. NULL in v1 (per-session correlation deferred); set null when the session is deleted.';
comment on column memory_loads.client_session_id
     is 'Optional assistant-side session id string (not a DB UUID). Reserved for future per-session correlation; NULL in v1.';
comment on column memory_loads.project_id
     is 'The project the context was assembled for (not necessarily the memory''s own scope — global/stack memories load into many projects). Null when unknown or after the project is deleted.';
comment on column memory_loads.memory_id
     is 'Foreign key to memories — which memory was loaded/injected.';
comment on column memory_loads.source
     is 'Which delivery path logged this load (e.g. get_layered_context).';
comment on column memory_loads.loaded_at
     is 'When the memory was delivered into context (server clock).';
