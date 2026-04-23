set search_path to sensei, extensions;

create type if not exists confidence_level
    as enum ('extracted', 'inferred', 'ambiguous');

create table if not exists call_edges (
  id                       uuid             primary key default gen_random_uuid()
, folder_id                uuid             not null references sensei.folders(id) on delete cascade
, caller_id                uuid             not null references sensei.symbols(id) on delete cascade
, callee_id                uuid             references sensei.symbols(id) on delete set null
, callee_name              text             not null
, callee_file              text
, confidence               confidence_level not null default 'extracted'
, modified_at              timestamptz      not null default now()
);

create index if not exists call_edges_folder_id_idx
    on call_edges(folder_id);

create index if not exists call_edges_caller_idx
    on call_edges(caller_id);

create index if not exists call_edges_callee_idx
    on call_edges(callee_id)
 where callee_id is not null;

create index if not exists call_edges_confidence_idx
    on call_edges(confidence);

comment on table call_edges is
'Call graph edges extracted by language adapters.
- caller_id: FK to the calling symbol
- callee_id: FK to the called symbol (nullable — null if unresolved)
- callee_name: name of the called function/symbol (always present, for unresolved lookups)
- callee_file: resolved file path of callee (null if unresolved)
- confidence: extracted (AST-certain), inferred (embedding similarity), ambiguous (drift/gap analysis)';

comment on column call_edges.id
     is 'Surrogate primary key (UUID).';
comment on column call_edges.folder_id
     is 'Foreign key to folders — which folder this edge belongs to.';
comment on column call_edges.caller_id
     is 'Foreign key to symbols — the symbol that performs this call.';
comment on column call_edges.callee_id
     is 'Foreign key to symbols — the resolved target symbol. Null if unresolved.';
comment on column call_edges.callee_name
     is 'Name of the function or symbol being called. Always present even when callee_id is null.';
comment on column call_edges.callee_file
     is 'Resolved repository-relative path of the file where the callee is defined. Null if unresolved.';
comment on column call_edges.confidence
     is 'Edge confidence: extracted (AST-certain), inferred (embedding/BM25), ambiguous (drift/gap).';
comment on column call_edges.modified_at
     is 'Timestamp of the last modification to this row.';
