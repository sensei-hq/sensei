-- phases
-- Optional containers for cards within a project or idea.
-- Phases are available, never required. A project with no phases uses a
-- default 'work' phase created on first card insert (handled in application).
--
-- position: display order; lower = further left in the pipeline view
-- built_in: 1 for the default phase set (exploration/requirements/analysis/
--   design/implementation/review); 0 for custom developer-defined phases

create table if not exists phases (
  id          text    not null primary key
, project_id  text    not null references projects(id) on delete cascade
, name        text    not null
, description text
, position    integer not null default 0
, built_in    integer not null default 0  -- 0=custom 1=default set
, is_archived integer not null default 0
, created_at  text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_at text    not null default (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
, modified_by text    not null default 'system'
, unique(project_id, name)
);

create index if not exists phases_project_id_idx
  on phases(project_id, position);
