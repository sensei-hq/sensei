set search_path to inference, sensei, extensions;

create type if not exists hyperedge_kind
    as enum ('flow', 'group', 'co_change', 'addressed_by');

create table if not exists hyperedges (
  id                       uuid            primary key default gen_random_uuid()
, folder_id                uuid            not null references sensei.folders(id) on delete cascade
, kind                     hyperedge_kind  not null
, label                    text
, description              text
, confidence               edge_confidence not null default 'extracted'
, member_count             integer         not null default 0
, modified_at              timestamptz     not null default now()
);

create index if not exists hyperedges_folder_id_idx
    on hyperedges(folder_id);

create index if not exists hyperedges_kind_idx
    on hyperedges(kind);

comment on table hyperedges is
'Multi-node relationships (3+ nodes simultaneously).
- flow: all functions in an auth flow or request pipeline
- group: all implementations of an interface, all files in a module
- co_change: files that always change together (from git history)
- addressed_by: all requirements addressed by a single session
Members stored in hyperedge_members.';

comment on column hyperedges.id
     is 'Surrogate primary key (UUID).';
comment on column hyperedges.folder_id
     is 'Foreign key to folders — which repo this hyperedge belongs to.';
comment on column hyperedges.kind
     is 'Hyperedge category: flow, group, co_change, addressed_by.';
comment on column hyperedges.label
     is 'Human-readable label (e.g. "auth refresh flow", "CRDTOp implementations").';
comment on column hyperedges.description
     is 'Description of what this group of nodes represents.';
comment on column hyperedges.confidence
     is 'How this hyperedge was detected: extracted, inferred, ambiguous.';
comment on column hyperedges.member_count
     is 'Denormalized count of members.';
comment on column hyperedges.modified_at
     is 'Timestamp of the last modification to this row.';
