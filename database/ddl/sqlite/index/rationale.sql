-- rationale
-- "Why" nodes extracted from annotated inline comments during parsing.
-- Captures the reasoning behind code decisions — what symbols.docstring misses.
--
-- Trigger patterns (case-insensitive, matched by language adapters):
--   // NOTE: // WHY: // REASON: // IMPORTANT: // HACK: // FIXME: // TODO: // DECISION:
--   # NOTE: # WHY: etc. (Python/shell)
--
-- These become first-class nodes in the graph, linked to their symbol via
-- graph_edges (rationale → symbol, relation='rationale_for').
-- Surfaced in the graph narrative report and in get_session_context orientation.

create table if not exists rationale (
  id          text not null primary key
, file_path   text not null
, line_number integer not null
, tag         text not null   -- NOTE | WHY | REASON | IMPORTANT | HACK | FIXME | TODO | DECISION
, content     text not null   -- the full comment text after the tag
, symbol_id   text references symbols(id) on delete set null  -- nearest enclosing symbol
, modified_at text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by text not null default 'system'
, unique(file_path, line_number)
);

create index if not exists rationale_file_path_idx on rationale(file_path);
create index if not exists rationale_symbol_idx    on rationale(symbol_id) where symbol_id is not null;
create index if not exists rationale_tag_idx       on rationale(tag);

create virtual table if not exists rationale_fts using fts5(
  id unindexed
, content
, content='rationale'
, content_rowid='rowid'
);
