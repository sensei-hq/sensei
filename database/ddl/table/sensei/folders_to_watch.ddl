set search_path to sensei, extensions;
create table if not exists folders_to_watch (
  id                       uuid         primary key default gen_random_uuid()
, path                     text         not null unique
, name                     text         not null
, note                     text
, status                   watch_status not null default 'scanning'
, excluded                 jsonb        not null default '[]'
, modified_at              timestamptz  not null default now()
);

comment on table folders_to_watch is
'Configuration: directories the user has pointed sensei at.
Initial status is scanning; transitions to watching after first scan completes.
Exclusions are user configuration — adding an entry deletes matching folders + children;
removing an entry triggers a re-scan of that subtree.';

comment on column folders_to_watch.id
     is 'Surrogate primary key (UUID).';
comment on column folders_to_watch.path
     is 'Absolute filesystem path to the watched root directory.';
comment on column folders_to_watch.name
     is 'Display name for the root (e.g. "Developer", "Work Projects").';
comment on column folders_to_watch.note
     is 'Optional user note about this root (e.g. "monorepo root, 3 packages").';
comment on column folders_to_watch.status
     is 'Lifecycle: scanning (initial scan in progress), watching (watchers active), paused (content retained, watchers stopped).';
comment on column folders_to_watch.excluded
     is 'JSON array of relative folder names/paths excluded from scanning, e.g. ["node_modules","dist",".git"].';
comment on column folders_to_watch.modified_at
     is 'Timestamp of the last modification to this row.';
