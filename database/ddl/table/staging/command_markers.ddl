set search_path to staging, extensions;

-- Landing table for database/import/staging/command_markers.jsonl. Dropped and
-- rebuilt on each deploy like every other staging table; sensei.command_markers
-- is what persists.
drop table if exists command_markers cascade;
create table command_markers (
  toolchain    text
, category     text
, kind         text
, marker       text
, modified_at  timestamptz
);
