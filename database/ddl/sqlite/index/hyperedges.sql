-- hyperedges + hyperedge_members
-- Group relationships connecting 3 or more nodes simultaneously.
-- Pairwise edges (call_edges, graph_edges) cannot express these.
--
-- Examples:
--   All functions in an auth flow
--   All classes implementing a shared interface
--   All files changed together in a migration
--   All requirements addressed by a single session
--
-- kind:
--   flow         — ordered sequence of nodes forming a processing flow
--   group        — unordered set sharing a property (e.g. all implementors of IAuth)
--   co_change    — files that historically change together (from git history)
--   addressed_by — nodes addressed together by a session or card

create table if not exists hyperedges (
  id          text not null primary key
, kind        text not null check (kind in ('flow','group','co_change','addressed_by'))
, label       text not null
, description text
, confidence  text not null default 'EXTRACTED'
                   check (confidence in ('EXTRACTED','INFERRED','AMBIGUOUS'))
, member_count integer not null default 0  -- denormalized from hyperedge_members count
, created_at  text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_at text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by text not null default 'system'
);

-- hyperedge_members
-- Each member of a hyperedge. position is used for ordered flows (kind='flow').
-- node_type and node_ref mirror graph_edges from_type / from_ref conventions.

create table if not exists hyperedge_members (
  id           text    not null primary key
, hyperedge_id text    not null references hyperedges(id) on delete cascade
, node_type    text    not null check (node_type in ('symbol','file','doc','card','session'))
, node_ref     text    not null
, position     integer not null default 0  -- order within flow; 0 for unordered groups
, unique(hyperedge_id, node_type, node_ref)
);

create index if not exists hyperedge_members_edge_idx on hyperedge_members(hyperedge_id, position);
create index if not exists hyperedge_members_node_idx on hyperedge_members(node_type, node_ref);
