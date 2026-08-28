set search_path to staging, extensions;

-- Landing table for database/import/staging/schedules.jsonl — the default
-- schedule for every schedulable worker. Dropped and rebuilt on each deploy like
-- every other staging table; sensei.schedules is what persists.
drop table if exists schedules cascade;
create table schedules (
  name           text
, enabled        boolean     default true
, interval_secs  integer
, window_start   time
, window_end     time
, days           smallint[]
, modified_at    timestamptz not null default now()
);
