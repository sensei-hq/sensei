set search_path to sensei, extensions;

create table if not exists docs (
  id            uuid primary key default gen_random_uuid()
, repo_id       uuid not null references sensei.repos(id) on delete cascade
, doc_path      text not null
, covers        text[]
, auto_detected boolean not null default false
, modified_at   timestamptz not null default now()
, unique(repo_id, doc_path)
);

create index if not exists docs_repo_id_idx on docs(repo_id);

comment on table docs is
'Traceability: which source files each doc covers. Replaces .sensei/traceability.json.
- covers: array of source file paths this doc documents
- auto_detected: true when coverage was inferred from doc content, not declared in llmspec';
