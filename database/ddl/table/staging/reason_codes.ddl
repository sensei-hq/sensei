set search_path to staging, extensions;

-- Landing table for database/import/staging/reason_codes.jsonl. Dropped and
-- rebuilt on each deploy like every other staging table; sensei.reason_codes is
-- what persists.
--
-- Text columns, not the sensei enums: a datafile carrying a value the enum lacks
-- must fail in the IMPORT with a readable cast error naming the row, not at COPY
-- time with a bare enum complaint about a file offset.
drop table if exists reason_codes cascade;
create table reason_codes (
  domain      text
, code        text
, kind        text
, precedence  smallint
, summary     text
, detail      text
, remedy      text
, actor       text
, modified_at timestamptz
);
