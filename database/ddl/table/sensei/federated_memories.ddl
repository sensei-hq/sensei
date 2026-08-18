set search_path to sensei, extensions;

create table if not exists federated_memories (
  knowledge_source_id uuid        not null references sensei.knowledge_sources(id) on delete cascade
, remote_rule_id      uuid        not null
, content_hash        text        not null
, memory_id           uuid        references sensei.memories(id) on delete set null
, remote_seq          bigint      not null
, synced_at           timestamptz not null default now()
, primary key (knowledge_source_id, remote_rule_id)
);

-- Covering index for the memory_id FK (nullable) — avoids a seq-scan when the
-- linked memory is hard-deleted (on delete set null). knowledge_source_id is the
-- PK's leading column, so its FK is already covered.
create index if not exists federated_memories_memory_id_idx
    on federated_memories(memory_id) where memory_id is not null;

comment on table federated_memories is
'Local↔remote rule mapping + per-rule cursor (federation sync bookkeeping — NOT
a parallel rules table). Pull upserts by (knowledge_source_id, remote_rule_id),
making ingestion idempotent; it is also the echo-guard — a rule this daemon
pushed is recorded here, so pulling it back links to the existing memory instead
of creating a federated duplicate. memory_id is the local memory; null after the
linked memory is hard-deleted.';
