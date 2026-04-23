set search_path to sensei, extensions;

create table if not exists memory_items (
  id                       uuid        primary key default gen_random_uuid()
, folder_id                uuid        not null references sensei.folders(id) on delete cascade
, session_id               uuid        references sensei.sessions(id) on delete set null
, type                     text        not null
                                       check (type in ('decision', 'pattern', 'question'))
, title                    text        not null
, content                  text        not null
, status                   text        not null default 'open'
                                       check (status in ('open', 'closed'))
, resolution               text
, closed_at                timestamptz
, created_at               timestamptz not null default now()
, modified_at              timestamptz not null default now()
);

create index if not exists memory_items_folder_id_idx
    on memory_items(folder_id, type, status);

create index if not exists memory_items_session_id_idx
    on memory_items(session_id)
 where session_id is not null;

comment on table memory_items is
'Project-scoped persistent knowledge that survives across sessions.
- type=decision: architectural choice (always surfaced in orientation)
- type=pattern: coding convention (always surfaced)
- type=question: open question needing resolution (surfaced until closed)';

comment on column memory_items.id
     is 'Surrogate primary key (UUID).';
comment on column memory_items.folder_id
     is 'Foreign key to folders — which folder this memory item belongs to.';
comment on column memory_items.session_id
     is 'Foreign key to sessions — the session that created this item.';
comment on column memory_items.type
     is 'Knowledge category: decision, pattern, or question.';
comment on column memory_items.title
     is 'Short label used as heading in orientation output.';
comment on column memory_items.content
     is 'Full body text surfaced to the agent during orientation.';
comment on column memory_items.status
     is 'Lifecycle: open (active) or closed (resolved, retained for history).';
comment on column memory_items.resolution
     is 'How the item was resolved. Required when closing a question.';
comment on column memory_items.closed_at
     is 'Timestamp when the item was closed. Null while open.';
comment on column memory_items.created_at
     is 'Timestamp when the row was first created.';
comment on column memory_items.modified_at
     is 'Timestamp of the last modification to this row.';
