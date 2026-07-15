set search_path to sensei, extensions;

create table if not exists symbol_names (
  name        text        primary key
, first_seen  timestamptz not null default now()
, last_seen   timestamptz not null default now()
);

comment on table symbol_names is
'Global registry of every code-symbol name ever indexed. The doc-drift scan
(scan_project_doc_drift) uses it as the "was ever a real symbol" gate: a doc
mention is flagged broken only when the name IS in this registry (it was a real
symbol at some point) but no longer resolves to a current sensei.nodes row
(removed / renamed). Identifiers that were NEVER symbols — Rust enum variants,
serde-renamed camelCase API fields, string-dispatched MCP tool names — are prose
or config, not drift, so their absence from the graph is not flagged (this
killed the ~408 false-positive drift rows). Populated each drift scan by
record_symbol_names (a monotonic upsert of the current symbol names); never
pruned, so a removed symbol stays known and its stale doc references surface as
real drift.';

comment on column symbol_names.name
     is 'The symbol name (matches sensei.nodes.name). Primary key — global, deduped.';
comment on column symbol_names.first_seen
     is 'When this name was first recorded as an indexed symbol.';
comment on column symbol_names.last_seen
     is 'When this name was last seen as a current indexed symbol (updated each scan).';
