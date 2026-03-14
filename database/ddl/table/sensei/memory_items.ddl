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
