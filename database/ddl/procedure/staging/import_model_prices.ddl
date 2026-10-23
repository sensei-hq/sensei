set search_path to staging, sensei, extensions;

-- Seed the API price list from staging.model_prices. Timestamp-guarded like the
-- other imports, so a hand-corrected row is not reverted by an older datafile.
create or replace procedure import_model_prices()
language plpgsql
set search_path = staging, sensei, extensions
as $$
begin
  insert into sensei.model_prices (
      model, speed, prompt_over, effective_from, effective_to
    , input, cache_write_5m, cache_write_1h, cache_read, output, us_multiplier
    , source, retrieved_on, modified_at
  )
  select
      stg.model
    , coalesce(nullif(stg.speed, ''), 'standard')
    , coalesce(stg.prompt_over, 0)
    , stg.effective_from
    , stg.effective_to
    , stg.input
    , stg.cache_write_5m
    , stg.cache_write_1h
    , stg.cache_read
    , stg.output
    , coalesce(stg.us_multiplier, 1)
    , stg.source
    , stg.retrieved_on
    , coalesce(stg.modified_at, now())
    from staging.model_prices stg
   where stg.model is not null
  on conflict on constraint model_prices_key do update set
      effective_to   = excluded.effective_to
    , input          = excluded.input
    , cache_write_5m = excluded.cache_write_5m
    , cache_write_1h = excluded.cache_write_1h
    , cache_read     = excluded.cache_read
    , output         = excluded.output
    , us_multiplier  = excluded.us_multiplier
    , source         = excluded.source
    , retrieved_on   = excluded.retrieved_on
    , modified_at    = excluded.modified_at
   where excluded.modified_at >= sensei.model_prices.modified_at;
end;
$$;
