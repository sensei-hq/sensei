set search_path to sensei, extensions;

-- What kind of thing a sync_state row tracks.
--
-- Enumerated rather than free text for the same reason task_kind was: a renamed
-- or retired entity would otherwise leave rows nothing ever reads again, and
-- nothing would notice.
create type sync_entity as enum (
    'repository'
  , 'project'
  , 'project_repository'
  , 'repository_metric'
  , 'metric_catalogue'
);
