set search_path to inference, sensei, extensions;
create table if not exists drift_items (
  id                       uuid         primary key default gen_random_uuid()
, folder_id                uuid         not null references sensei.folders(id) on delete cascade
, doc_node_id              uuid         not null references sensei.nodes(id) on delete cascade
, code_node_id             uuid         references sensei.nodes(id) on delete set null
, status                   drift_status not null default 'current'
, expected_signature       text
, actual_signature         text
, detail                   text
, detected_at              timestamptz  not null default now()
, resolved_at              timestamptz
);

create index if not exists drift_items_folder_id_idx
    on drift_items(folder_id);

create index if not exists drift_items_status_idx
    on drift_items(status)
 where status != 'current';

comment on table drift_items is
'Doc-to-code drift tracking. One row per reference from a doc node to a code node.
- current: reference is valid, signatures match
- drifted: target exists but signature changed
- broken: target was deleted or renamed
- expected_signature: what the doc says the symbol looks like
- actual_signature: what the code actually has (null if broken)';

comment on column drift_items.id
     is 'Surrogate primary key (UUID).';
comment on column drift_items.folder_id
     is 'Foreign key to folders — which folder this drift was detected in.';
comment on column drift_items.doc_node_id
     is 'Foreign key to nodes — the doc section that references the code.';
comment on column drift_items.code_node_id
     is 'Foreign key to nodes — the code symbol being referenced. Null if broken (deleted).';
comment on column drift_items.status
     is 'Drift status: current (valid), drifted (signature changed), broken (target deleted).';
comment on column drift_items.expected_signature
     is 'What the documentation says the symbol looks like.';
comment on column drift_items.actual_signature
     is 'What the code actually has. Null if the target no longer exists.';
comment on column drift_items.detail
     is 'Human-readable description of the drift.';
comment on column drift_items.detected_at
     is 'Timestamp when this drift was first detected.';
comment on column drift_items.resolved_at
     is 'Timestamp when the drift was resolved (doc updated or code reverted). Null if unresolved.';
