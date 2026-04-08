-- graph_edges
-- Confidence-tagged edges for the full project knowledge graph.
-- A superset of call_edges — includes all relationship types between any two nodes,
-- whether code→code, code→doc, or doc→doc.
--
-- confidence:
--   EXTRACTED  — relationship explicitly stated in source (import, direct call, explicit ref)
--   INFERRED   — reasonable deduction (call-graph second pass, co-occurrence, semantic sim)
--   AMBIGUOUS  — uncertain; flagged for review in graph report
--
-- confidence_score: 0.0–1.0 for INFERRED edges; 1.0 for EXTRACTED; null for AMBIGUOUS
--
-- from_type / to_type:
--   symbol     — a code symbol (symbols.id)
--   file       — a source file path
--   doc        — a doc_section (doc_sections.id)
--   rationale  — a rationale node (rationale.id)
--   community  — a community label
--
-- relation: see card_links for the human-facing version; this table uses code-level relations
--   calls, imports, exports, implements, extends, uses,
--   documents, references, semantically_similar_to

create table if not exists graph_edges (
  id                text    not null primary key
, from_type         text    not null check (from_type in ('symbol','file','doc','rationale'))
, from_ref          text    not null   -- id or path depending on from_type
, relation          text    not null
, to_type           text    not null check (to_type in ('symbol','file','doc','rationale'))
, to_ref            text    not null
, confidence        text    not null default 'EXTRACTED'
                            check (confidence in ('EXTRACTED','INFERRED','AMBIGUOUS'))
, confidence_score  real    check (confidence_score between 0.0 and 1.0)
, modified_at       text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by       text    not null default 'system'
, unique(from_type, from_ref, relation, to_type, to_ref)
);

create index if not exists graph_edges_from_idx       on graph_edges(from_type, from_ref);
create index if not exists graph_edges_to_idx         on graph_edges(to_type, to_ref);
create index if not exists graph_edges_confidence_idx on graph_edges(confidence);
create index if not exists graph_edges_relation_idx   on graph_edges(relation);
