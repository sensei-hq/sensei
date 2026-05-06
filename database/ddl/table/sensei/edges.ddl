set search_path to sensei, extensions;

create table if not exists edges (
  id                       uuid            primary key default gen_random_uuid()
, folder_id                uuid            not null references sensei.folders(id) on delete cascade
, source_id                uuid            not null references sensei.nodes(id) on delete cascade
, target_id                uuid            references sensei.nodes(id) on delete cascade
, target_name              text
, target_file              text
, kind                     edge_kind       not null
, confidence               edge_confidence not null default 'extracted'
, confidence_score         numeric(3,2)
, props                    jsonb           not null default '{}'
, modified_at              timestamptz     not null default now()
);

create index if not exists edges_folder_id_idx
    on edges(folder_id);

create index if not exists edges_source_id_idx
    on edges(source_id);

create index if not exists edges_target_id_idx
    on edges(target_id)
 where target_id is not null;

create index if not exists edges_kind_idx
    on edges(kind);

create index if not exists edges_confidence_idx
    on edges(confidence);

comment on table edges is
'Typed relationships between nodes in the code graph.

Edge kinds:
  calls — function/method calls another function/method
  implements — class implements interface
  extends — class/interface extends another
  imports — file imports from file/module (props: {names:["useState","useEffect"]})
  depends_on — module-level dependency
  traces_to — documentation traces to code symbol
  references — doc section references a symbol or file
  covers — doc file covers source file
  rationale_for — comment explains a symbol
  duplicates — structurally identical code (confidence_score = similarity)
  similar_to — semantically similar (inferred from embeddings)

target_id is nullable — null when the target node is unresolved (external symbol, unindexed file).
target_name and target_file provide lookup info for unresolved edges.';

comment on column edges.id
     is 'Surrogate primary key (UUID).';
comment on column edges.folder_id
     is 'Foreign key to folders — which repo this edge belongs to.';
comment on column edges.source_id
     is 'Foreign key to nodes — the source node of this relationship.';
comment on column edges.target_id
     is 'Foreign key to nodes — the target node. Null if unresolved.';
comment on column edges.target_name
     is 'Name of the target for unresolved edges. Allows lookup without a node.';
comment on column edges.target_file
     is 'File path of the target for unresolved edges.';
comment on column edges.kind
     is 'Relationship type. See table comment for full list.';
comment on column edges.confidence
     is 'Edge confidence: extracted (AST-certain), inferred (embedding/BM25), ambiguous (drift/gap).';
comment on column edges.confidence_score
     is 'Numeric confidence 0.00-1.00. Used for similarity/duplicate edges.';
comment on column edges.props
     is 'Extensible metadata. For imports: {names:["a","b"]}. For duplicates: {similarity:0.86}.';
comment on column edges.modified_at
     is 'Timestamp of the last modification to this row.';
