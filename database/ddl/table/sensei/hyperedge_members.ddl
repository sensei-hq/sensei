set search_path to sensei, extensions;

create table if not exists hyperedge_members (
  id                       uuid        primary key default gen_random_uuid()
, hyperedge_id             uuid        not null references sensei.hyperedges(id) on delete cascade
, node_id                  uuid        not null references sensei.nodes(id) on delete cascade
, position                 integer
, unique(hyperedge_id, node_id)
);

create index if not exists hyperedge_members_hyperedge_id_idx
    on hyperedge_members(hyperedge_id);

create index if not exists hyperedge_members_node_id_idx
    on hyperedge_members(node_id);

comment on table hyperedge_members is
'Members of a hyperedge. Links nodes to their hyperedge with optional ordering.
- position: sequence order for flow-type hyperedges (null for unordered groups)';

comment on column hyperedge_members.id
     is 'Surrogate primary key (UUID).';
comment on column hyperedge_members.hyperedge_id
     is 'Foreign key to hyperedges — which hyperedge this member belongs to.';
comment on column hyperedge_members.node_id
     is 'Foreign key to nodes — which node is a member of this hyperedge.';
comment on column hyperedge_members.position
     is 'Order position for flow-type hyperedges. Null for unordered groups.';
