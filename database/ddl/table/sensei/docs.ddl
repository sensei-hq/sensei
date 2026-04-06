set search_path to sensei, extensions;

create table if not exists docs (
  id            uuid primary key default gen_random_uuid()
, repo_id       uuid not null references sensei.repos(id) on delete cascade
, doc_path      text not null
, covers        text[]
, auto_detected boolean not null default false
, modified_at   timestamptz not null default now()
, modified_by   text        not null default current_user
, unique(repo_id, doc_path)
);

create index if not exists docs_repo_id_idx on docs(repo_id);

comment on table docs is
'Traceability: which source files each doc covers. Replaces .sensei/traceability.json.
- covers: array of source file paths this doc documents
- auto_detected: true when coverage was inferred from doc content, not declared in llmspec';

comment on column docs.id is 'Surrogate primary key (UUID).';
comment on column docs.repo_id is 'Foreign key to sensei.repos — scopes this row to a specific repository.';
comment on column docs.doc_path is 'Repository-relative path of the documentation file.';
comment on column docs.covers is 'Array of source file paths that this doc file documents.';
comment on column docs.auto_detected is 'True when doc coverage was inferred from content rather than declared in llmspec.';
comment on column docs.modified_at is 'Timestamp of the last modification to this row.';
comment on column docs.modified_by is 'Identity (user, role, or service) that last modified this row.';
