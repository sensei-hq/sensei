set search_path to inference, sensei, extensions;

create table if not exists corrections (
  id            uuid         primary key default gen_random_uuid()
, signature     text         not null unique
, text          text         not null
, suggestion    text
, count         integer      not null default 0
, project_ids   uuid[]       not null default '{}'
, last_seen     timestamptz
, memory_id     uuid         references sensei.memories(id) on delete set null
, instances     jsonb        not null default '[]'
, detected_at   timestamptz  not null default now()
, modified_at   timestamptz  not null default now()
);

create index if not exists corrections_project_ids_idx
    on corrections using gin(project_ids);

create index if not exists corrections_count_idx
    on corrections(count desc);

-- Covering index for the memory_id FK (nullable) — avoids a seq-scan when a
-- referenced memory is deleted (on delete set null).
create index if not exists corrections_memory_id_idx
    on corrections(memory_id) where memory_id is not null;

comment on table corrections is
'Recurring developer corrections, clustered globally across projects (analyzer #65 step 5).
One row per recurring correction cluster: similar corrective prompts grouped by
embedding (or lexical) similarity. Re-derived idempotently by the AggregateCorrections
task; `signature` is the stable natural key.';

comment on column corrections.id           is 'Surrogate primary key (UUID). Stable across runs via upsert-on-signature.';
comment on column corrections.signature    is 'Deterministic cluster identity: hash(seed_session + normalized seed prompt). Stable as the cluster grows.';
comment on column corrections.text         is 'Canonical correction statement (LLM). Falls back to the seed member''s normalized snippet when no chat model is available.';
comment on column corrections.suggestion   is 'LLM advisory on what to do (reinforce a memory / add a rule / write a skill). Null when no chat model.';
comment on column corrections.count        is 'Number of corrective prompts in the cluster.';
comment on column corrections.project_ids  is 'Distinct projects the correction appeared in. Names resolved by the API.';
comment on column corrections.last_seen    is 'Most recent corrective prompt in the cluster.';
comment on column corrections.memory_id    is 'Related learned memory (LLM-matched from a shortlist), or null.';
comment on column corrections.instances    is 'Provenance: [{project_id, session_id, ts, prompt}] — the member corrective prompts (snippet).';
comment on column corrections.detected_at  is 'When this cluster was first derived.';
comment on column corrections.modified_at  is 'When this row was last upserted.';
