set search_path to sensei, extensions;

create table if not exists call_edges (
  id          uuid primary key default gen_random_uuid()
, repo_id     uuid not null references sensei.repos(id) on delete cascade
, caller_id   uuid not null references sensei.symbols(id) on delete cascade
, callee_name text not null
, callee_file text
, modified_at timestamptz not null default now()
, modified_by text        not null default current_user
);

create index if not exists call_edges_repo_id_idx  on call_edges(repo_id);
create index if not exists call_edges_caller_idx   on call_edges(caller_id);

comment on table call_edges is
'Call graph edges extracted by the TypeScript adapter.
- caller_id: FK to the calling symbol in symbols table
- callee_name: name of the called function/symbol
- callee_file: resolved file path of callee (null if unresolved)';

comment on column call_edges.id is 'Surrogate primary key (UUID).';
comment on column call_edges.repo_id is 'Foreign key to sensei.repos — scopes this row to a specific repository.';
comment on column call_edges.caller_id is 'Foreign key to sensei.symbols — identifies the symbol that performs this call.';
comment on column call_edges.callee_name is 'Name of the function or symbol being called.';
comment on column call_edges.callee_file is 'Resolved repository-relative path of the file where the callee is defined; null if unresolved.';
comment on column call_edges.modified_at is 'Timestamp of the last modification to this row.';
comment on column call_edges.modified_by is 'Identity (user, role, or service) that last modified this row.';
