set search_path to sensei, extensions;

-- A model's published API list prices, dated (docs/spec/daemon/session-api-cost.md
-- C3). What a session's tokens would cost at the API — `activity.message_api_cost`
-- prices each message from the row whose model, speed, prompt band and period
-- hold it. Seeded from database/import/staging/model_prices.jsonl.
--
-- Facts only: no row means no price (the message's cost is NULL), never 0.
create table if not exists model_prices (
  id             uuid        primary key default gen_random_uuid()
, model          text        not null
, speed          text        not null default 'standard'
, prompt_over    integer     not null default 0
, effective_from date
, effective_to   date
, input          numeric     not null
, cache_write_5m numeric     not null
, cache_write_1h numeric     not null
, cache_read     numeric     not null
, output         numeric     not null
, us_multiplier  numeric     not null default 1
, source         text        not null
, retrieved_on   date        not null
, modified_at    timestamptz not null default now()
, constraint model_prices_key unique nulls not distinct (model, speed, prompt_over, effective_from)
, constraint model_prices_period check (effective_to is null or effective_from is null or effective_from < effective_to)
);

comment on table model_prices is
'Published API list prices per model, USD per million tokens, by speed, prompt band and period.';
comment on column model_prices.model is
'The API model id without a date suffix (claude-opus-5-5); a message''s model matches after its -YYYYMMDD is stripped.';
comment on column model_prices.speed is
'standard or fast; fast rows exist only for models the price list prices fast.';
comment on column model_prices.prompt_over is
'The row applies to a request whose prompt (input + cache reads + cache writes) is over this many tokens; the highest matching band wins. 0 = every request.';
comment on column model_prices.effective_from is
'First day the price applied; NULL = since before any known change.';
comment on column model_prices.effective_to is
'First day it no longer applied; NULL = still current.';
comment on column model_prices.us_multiplier is
'Applied to every category when the request ran with inference_geo = us.';
comment on column model_prices.source is
'Where the row came from: the pricing page URL.';
comment on column model_prices.retrieved_on is
'The day the source was read.';
