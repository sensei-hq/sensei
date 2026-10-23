set search_path to staging, extensions;

-- Landing table for database/import/staging/model_prices.jsonl. Dropped and
-- rebuilt on each deploy like every other staging table; sensei.model_prices is
-- what persists.
drop table if exists model_prices cascade;
create table model_prices (
  model          text
, speed          text
, prompt_over    integer
, effective_from date
, effective_to   date
, input          numeric
, cache_write_5m numeric
, cache_write_1h numeric
, cache_read     numeric
, output         numeric
, us_multiplier  numeric
, source         text
, retrieved_on   date
, modified_at    timestamptz
);
