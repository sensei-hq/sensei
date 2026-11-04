set search_path to staging, extensions;

-- Landing table for database/import/staging/command_patterns.jsonl. Dropped and
-- rebuilt on each deploy like every other staging table; sensei.command_patterns
-- is what persists.
drop table if exists command_patterns cascade;
create table command_patterns (
  toolchain    text
, pattern      text
, category     text
, wrapper      boolean
, compiles     boolean
, owns         text[]
, priority     integer
, modified_at  timestamptz
);
