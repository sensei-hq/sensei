set search_path to sensei, extensions;

create table if not exists call_edges (
  id          uuid primary key default gen_random_uuid()
, repo_id     uuid not null references sensei.repos(id) on delete cascade
, caller_id   uuid not null references sensei.symbols(id) on delete cascade
, callee_name text not null
, callee_file text
);

create index if not exists call_edges_repo_id_idx  on call_edges(repo_id);
create index if not exists call_edges_caller_idx   on call_edges(caller_id);

comment on table call_edges is
'Call graph edges extracted by the TypeScript adapter.
- caller_id: FK to the calling symbol in symbols table
- callee_name: name of the called function/symbol
- callee_file: resolved file path of callee (null if unresolved)';
