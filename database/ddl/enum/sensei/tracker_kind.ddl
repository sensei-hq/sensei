set search_path to sensei, extensions;

-- Which issue tracker a connection speaks to. One value per registered adapter
-- (`crate::tracker::adapters()`); a value with no adapter is a bug, not data.
create type tracker_kind as enum (
  'jira'      -- Jira Cloud (Agile REST); GitLab is the next adapter
);

comment on type tracker_kind is
'The issue tracker a sensei.trackers row connects to. Mirrors the adapter registry in crate::tracker; add a value only together with its adapter.';
