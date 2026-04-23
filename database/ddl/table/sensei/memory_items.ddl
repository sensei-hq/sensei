set search_path to sensei, extensions;

create table if not exists memory_items (
  id                       uuid          primary key default gen_random_uuid()
, project_id               uuid          not null references sensei.projects(id) on delete cascade
, type                     memory_type   not null
, title                    text          not null
, content                  text          not null
, status                   memory_status not null default 'open'
, resolution               text
, closed_at                timestamptz
, modified_at              timestamptz   not null default now()
);

create index if not exists memory_items_project_id_idx
    on memory_items(project_id, type, status);

comment on table memory_items is
'Project-scoped persistent knowledge that survives across sessions.
- type=decision: architectural choice (always surfaced in orientation)
- type=pattern: coding convention (always surfaced)
- type=question: open question needing resolution (surfaced until closed)
Provenance (which session created an item) is tracked via activity.events.';

comment on column memory_items.id
     is 'Surrogate primary key (UUID).';
comment on column memory_items.project_id
     is 'Foreign key to projects — which project this memory item belongs to.';
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
comment on column memory_items.modified_at
     is 'Timestamp of the last modification to this row.';
