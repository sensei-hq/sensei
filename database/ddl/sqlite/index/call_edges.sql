-- call_edges
-- Call graph edges extracted by language adapters.
-- caller_id: FK to symbols; callee resolved to symbol id when possible.
-- confidence: EXTRACTED (direct call found in AST) | INFERRED (call-graph second pass)
-- See graph_edges.sql for all graph edges including cross-file and doc links.

create table if not exists call_edges (
  id          text not null primary key
, caller_id   text not null references symbols(id) on delete cascade
, callee_name text not null
, callee_id   text references symbols(id) on delete set null  -- null if unresolved
, callee_file text
, confidence  text not null default 'EXTRACTED'
                   check (confidence in ('EXTRACTED','INFERRED','AMBIGUOUS'))
, modified_at text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by text not null default 'system'
, unique(caller_id, callee_name, callee_file)
);

create index if not exists call_edges_caller_idx  on call_edges(caller_id);
create index if not exists call_edges_callee_idx  on call_edges(callee_id) where callee_id is not null;
