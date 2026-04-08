-- memory_items
-- Project-scoped persistent knowledge that survives across sessions.
-- type=decision: architectural or design choice (always surfaced in orientation)
-- type=pattern:  coding convention the agent should follow
-- type=question: open question; surfaced until closed
-- Closed items retained for history but excluded from orientation output.

create table if not exists memory_items (
  id          text not null primary key
, project_id  text not null references projects(id) on delete cascade
, session_id  text references sessions(id) on delete set null
, type        text not null check (type in ('decision','pattern','question'))
, title       text not null
, content     text not null
, status      text not null default 'open' check (status in ('open','closed'))
, resolution  text
, closed_at   text
, created_at  text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_at text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by text not null default 'system'
);

create index if not exists memory_items_project_idx  on memory_items(project_id, type, status);
create index if not exists memory_items_session_idx  on memory_items(session_id)
  where session_id is not null;
