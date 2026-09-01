set search_path to sensei, extensions;
create table if not exists nodes (
  id                       uuid        primary key default gen_random_uuid()
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, parent_id                uuid        references sensei.nodes(id) on delete cascade
, kind                     node_kind   not null
, name                     text        not null
, file_path                text
, fqn                      text
, resolved                 boolean     not null default false
, language                 text
, signature                text
, description              text
, content                  text
, docstring                text
, line_start               integer
, line_end                 integer
, is_exported              boolean     not null default false
, is_test                  boolean     not null default false
, community_id             integer
, embedding                vector(384)
, tags                     text[]      not null default '{}'
, props                    jsonb       not null default '{}'
, modified_at              timestamptz not null default now()
);

-- Node identity (line-based). A PARTIAL unique index (`where file_path is not
-- null`) so it governs only rows that HAVE a local file — every legacy node and
-- every migrated definition — and NOT the FQN reference stubs / `lib_symbol` nodes
-- (file_path NULL), which are governed solely by `nodes_unique_fqn`. That split is
-- what lets two same-simple-name, different-FQN stubs stay distinct: they would
-- otherwise collide on `(folder_id, NULL, kind, name, NULL, NULL)` and false-merge.
-- Behaviour-preserving for existing data — every current row has a non-null
-- file_path, so the partial index covers exactly the same rows as the old table
-- constraint. Keyed on `line_start` (not `signature`): same-name methods across
-- impl blocks share a decl-line text and would collapse under a signature key
-- (D5c nesting would fix that). NULLS NOT DISTINCT treats NULLs as equal so a
-- file/module node (parent_id/line_start NULL) conflicts rather than duplicates.
create unique index if not exists nodes_unique_identity
    on nodes (folder_id, file_path, kind, name, parent_id, line_start)
    nulls not distinct
 where file_path is not null;

-- FQN identity (SCIP/LSIF moniker model). A definition AND every reference to it
-- get-or-create the same node keyed on the fully-qualified name, so a call edge
-- resolves to a real target at emit instead of a bare-name guess. Partial (only
-- fqn-bearing rows) so it coexists with the line-based `nodes_unique_identity`:
--   • legacy nodes (file/section/rationale + not-yet-migrated languages): fqn NULL
--     → governed by the (partial) identity index only.
--   • reference stubs + external `lib_symbol` nodes: file_path NULL, fqn set
--     → governed by this index only (the partial identity index excludes them, so
--       same-name different-fqn stubs never false-merge).
--   • migrated defs: both file_path and fqn set → in both indexes (consistent).
create unique index if not exists nodes_unique_fqn
    on nodes (folder_id, fqn)
 where fqn is not null;

create index if not exists nodes_folder_id_idx
    on nodes(folder_id);

create index if not exists nodes_parent_id_idx
    on nodes(parent_id)
 where parent_id is not null;

create index if not exists nodes_kind_idx
    on nodes(kind);

create index if not exists nodes_file_path_idx
    on nodes(folder_id, file_path);

create index if not exists nodes_name_idx
    on nodes(folder_id, name);

create index if not exists nodes_exported_idx
    on nodes(folder_id, is_exported)
 where is_exported;

create index if not exists nodes_community_id_idx
    on nodes(community_id)
 where community_id is not null;

create index if not exists nodes_embedding_hnsw
    on nodes using hnsw (embedding vector_cosine_ops)
  with (m = 16, ef_construction = 64)
 where embedding is not null;

create index if not exists nodes_tags_idx
    on nodes using gin(tags);

comment on table nodes is
'Unified node table for the code graph. Every structural element is a node:
files, code symbols, doc sections, rationale comments.

Hierarchy via parent_id:
  file → class → method → parameter
  file → section (H1) → section (H2) → section (H3)
  file → function
  function → rationale (from comments)

Node kinds:
  file — source or doc file within a repo
  module, package — structural containers
  class, interface — type definitions
  function, method — callable code
  property, field, parameter — members
  type, const, enum, enum_variant — value definitions
  section — documentation heading (level stored in props)
  rationale — extracted from NOTE/WHY/HACK/TODO/IMPORTANT comments

file_path is denormalized on every node for fast file-scoped queries.
Relationships between nodes are stored in the edges table.';

comment on column nodes.id
     is 'Surrogate primary key (UUID).';
comment on column nodes.folder_id
     is 'Foreign key to folders — which repo this node belongs to.';
comment on column nodes.parent_id
     is 'Self-referencing FK for containment hierarchy. Null = top-level in file.';
comment on column nodes.kind
     is 'Node classification. See table comment for full list.';
comment on column nodes.name
     is 'Identifier name or heading text.';
comment on column nodes.file_path
     is 'Folder-relative path of the source file. Denormalized for query performance.
NULL for reference stubs and external lib_symbol nodes, which have no local file
(see resolved/fqn). Non-NULL for every definition and legacy node.';
comment on column nodes.fqn
     is 'Fully-qualified name (SCIP/LSIF moniker). Definitions and references
get-or-create the same node by (folder_id, fqn) so call edges resolve at emit.
NULL for legacy / not-yet-migrated nodes, which fall back to bare-name matching.';
comment on column nodes.resolved
     is 'True once a definition has been seen for this fqn (file_path/signature/line
filled). False for a reference-first stub awaiting its definition. lib_symbol nodes
are resolved=true (the external symbol IS the definition; nothing to enrich).';
comment on column nodes.language
     is 'Language slug (rust/python/typescript/…) derived from the file extension at
write time. Scopes the bare-name fallback to same-language candidates during the
per-language FQN rollout so a co-resident un-migrated language never cross-matches.';
comment on column nodes.signature
     is 'Abbreviated type signature (L0 level). For code symbols.';
comment on column nodes.description
     is 'Human-readable description (L1 level). For code symbols or doc summary.';
comment on column nodes.content
     is 'Full content text (L2/L3). For doc sections: markdown body. For rationale: comment text.';
comment on column nodes.docstring
     is 'Extracted JSDoc/TSDoc/rustdoc comment. For code symbols.';
comment on column nodes.line_start
     is 'One-based line number where this node begins in the source file.';
comment on column nodes.line_end
     is 'One-based line number where this node ends.';
comment on column nodes.is_exported
     is 'True when the symbol is exported from its file. Only meaningful for code symbols.';
comment on column nodes.is_test
     is 'True when this node belongs to a test file (path-convention: tests/ dirs,
*.test.*/*.spec.*, *_test.*, test_*.py, *Test.java, …; see languages::is_test_path).
Lets the UI filter tests out when focusing on production code. lib_symbol/lib_package
(external deps) are never test. Reference stubs inherit it from their definition.';
comment on column nodes.community_id
     is 'Leiden community cluster ID. Batch-computed. Null until computed.';
comment on column nodes.embedding
     is '384-dim vector embedding for semantic search. Computed by local model during indexing. HNSW indexed.';
comment on column nodes.tags
     is 'Array of tag strings for filtering.';
comment on column nodes.props
     is 'Extensible metadata. For sections: {level:2}. For rationale: {tag:"WHY"}. For files: {language:"rust"}.';
comment on column nodes.modified_at
     is 'Timestamp of the last modification to this row.';
