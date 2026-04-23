set search_path to inference, extensions;

create table if not exists communities (
  id                       uuid        primary key default gen_random_uuid()
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, community_id             integer     not null
, label                    text
, description              text
, node_count               integer     not null default 0
, god_node_ids             uuid[]      not null default '{}'
, computed_at              timestamptz not null default now()
, modified_at              timestamptz not null default now()
, unique(folder_id, community_id)
);

create index if not exists communities_folder_id_idx
    on communities(folder_id);

comment on table communities is
'Leiden algorithm community clusters. Computed offline as a batch job.
- community_id: integer cluster ID (referenced by nodes.community_id)
- god_node_ids: top-5 highest-degree nodes in this community
- node_count: denormalized count of nodes in this cluster';

comment on column communities.id
     is 'Surrogate primary key (UUID).';
comment on column communities.folder_id
     is 'Foreign key to folders — which repo this community was detected in.';
comment on column communities.community_id
     is 'Leiden-assigned cluster integer. Matches nodes.community_id.';
comment on column communities.label
     is 'Human-readable label for this community (e.g. "auth module", "CRDT sync").';
comment on column communities.description
     is 'One-paragraph summary of what this community does.';
comment on column communities.node_count
     is 'Number of nodes assigned to this community.';
comment on column communities.god_node_ids
     is 'UUIDs of top-5 highest-degree nodes in this community.';
comment on column communities.computed_at
     is 'Timestamp when Leiden algorithm last ran for this cluster.';
comment on column communities.modified_at
     is 'Timestamp of the last modification to this row.';
