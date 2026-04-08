-- cards
-- Atomic units of thinking within a phase. Every requirement, analysis finding,
-- design decision, task, or note is a card.
--
-- status lifecycle: open → accepted | deferred | superseded
-- Cards are never deleted — superseded cards link to their successor.
--
-- kind: loose categorisation for catalog filtering
--   requirement   — what the system must do
--   finding       — analysis result, gap, constraint
--   decision      — architectural or design choice (ADR)
--   task          — implementation work item
--   note          — freeform / scratchpad (default for exploration phase)
--   question      — open question needing resolution

create table if not exists cards (
  id            text not null primary key
, project_id    text not null references projects(id) on delete cascade
, phase_id      text references phases(id) on delete set null
, kind          text not null default 'note'
                     check (kind in ('requirement','finding','decision','task','note','question'))
, title         text not null
, body          text
, status        text not null default 'open'
                     check (status in ('open','accepted','deferred','superseded'))
, tags          text not null default '[]'  -- JSON: ["auth","login"]
, superseded_by text references cards(id) on delete set null
, resolved_at   text                        -- ISO 8601; set when status leaves 'open'
, created_at    text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_at   text not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by   text not null default 'system'
);

create index if not exists cards_project_id_idx    on cards(project_id, status, modified_at desc);
create index if not exists cards_phase_id_idx      on cards(phase_id, status);
create index if not exists cards_kind_idx          on cards(project_id, kind, status);
create index if not exists cards_superseded_by_idx on cards(superseded_by) where superseded_by is not null;

-- Full-text search over card title and body
create virtual table if not exists cards_fts using fts5(
  id unindexed
, title
, body
, content='cards'
, content_rowid='rowid'
);
