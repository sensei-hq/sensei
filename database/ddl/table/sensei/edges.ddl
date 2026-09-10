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

-- Stage 0 S7 (R10.1). The ATTRIBUTION index: answers "which edges did file F
-- contribute to" via `props->'occurrences' ? F`, which is the ONLY unit of
-- edge attribution reconcile may use. Without it that question needs an
-- fqn-based approximation, and every such approximation measured so far has
-- been a narrowing that silently strands stale occurrences.
--
-- THE OPERATOR CLASS IS LOAD-BEARING. props.occurrences is an OBJECT keyed by
-- file, so the test is `?` (key existence), which only the DEFAULT jsonb_ops
-- supports. jsonb_path_ops is smaller and looks like the better choice; it
-- CANNOT serve `?` and degrades silently to a seq scan. Proven both ways on
-- this table before this index was written. Do not "optimise" it.
create index if not exists edges_occurrences_gin
    on edges using gin ((props -> 'occurrences'));

comment on index sensei.edges_occurrences_gin is
'Attribution index for reconcile (R10.1): "which edges did file F contribute
to", via props->''occurrences'' ? F. Opclass MUST be the default jsonb_ops —
jsonb_path_ops does not support `?` and falls back to a seq scan without
error. Measured at creation: 128ms build, 888kB, against a column that is
100% NULL today because the v2 writer has no caller yet — that is a FLOOR,
not a steady-state size.';

-- Edge identity (D1): two partial unique indexes give an edge an identity so
-- insert_edge can upsert instead of duplicating. The nullable target_id forces
-- the split — a resolved edge is unique by its target node; an unresolved edge
-- is unique by (target_name, target_file). `nulls not distinct` (PG15+) makes a
-- NULL target_file/target_name dedup like a value, so repeated unresolved
-- inserts collapse to one row instead of accumulating.
create unique index if not exists edges_unique_resolved
    on edges (folder_id, source_id, target_id, kind)
 where target_id is not null;

create unique index if not exists edges_unique_unresolved
    on edges (folder_id, source_id, target_name, target_file, kind) nulls not distinct
 where target_id is null;

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
     is 'Extensible metadata, MERGED on re-insert (props || EXCLUDED.props) so a later writer cannot erase an earlier one. For imports: {names:["a","b"]}. For duplicates: {similarity:0.86}. For extends/implements: {relation:"extends"|"implements"|"trait_impl"} — the discriminant that separates a Rust trait impl from Java-style interface implementation, which share the implements kind.';
comment on column edges.modified_at
     is 'Timestamp of the last modification to this row.';
