set search_path to sensei, extensions;

create table if not exists memory_items (
  id          uuid        primary key default gen_random_uuid()
, repo_id     uuid        not null references sensei.repos(id) on delete cascade
, session_id  uuid        references sensei.sessions(id) on delete set null
, type        text        not null
              check (type in ('decision', 'pattern', 'question'))
, title       text        not null
, content     text        not null
, status      text        not null default 'open'
              check (status in ('open', 'closed'))
, resolution  text
, closed_at   timestamptz
, created_at  timestamptz not null default now()
, modified_at timestamptz not null default now()
, modified_by text        not null default current_user
);

create index if not exists memory_items_repo_id_idx    on memory_items(repo_id, type, status);
create index if not exists memory_items_session_id_idx on memory_items(session_id)
  where session_id is not null;

comment on table memory_items is
'Project-scoped persistent knowledge that survives across sessions.
- type=decision: architectural or design choice (always surfaced in orientation)
- type=pattern:  coding convention the agent should follow (always surfaced)
- type=question: open question needing resolution (surfaced until closed)
- session_id: the session that created the item (nullable; set null on session delete)
- resolution: required when closing a question via close_memory
- Closed items are retained for history but not surfaced in orientation';

comment on column memory_items.id is 'Surrogate primary key (UUID).';
comment on column memory_items.repo_id is 'Foreign key to sensei.repos — scopes this row to a specific repository.';
comment on column memory_items.session_id is 'Foreign key to sensei.sessions — links this row to the agent session that created it.';
comment on column memory_items.type is 'Knowledge category: decision (architectural choice), pattern (coding convention), or question (open issue).';
comment on column memory_items.title is 'Short label for this memory item, used as a heading in orientation output.';
comment on column memory_items.content is 'Full body text of the memory item, surfaced verbatim to the agent during orientation.';
comment on column memory_items.status is 'Lifecycle state: open (active/visible) or closed (resolved, retained for history only).';
comment on column memory_items.resolution is 'Explanation of how the item was resolved; required when closing a question via close_memory.';
comment on column memory_items.closed_at is 'Timestamp when the item was closed; null while status is open.';
comment on column memory_items.created_at is 'Timestamp when the row was first created.';
comment on column memory_items.modified_at is 'Timestamp of the last modification to this row.';
comment on column memory_items.modified_by is 'Identity (user, role, or service) that last modified this row.';
