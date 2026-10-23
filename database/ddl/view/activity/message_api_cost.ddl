set search_path to activity, sensei, extensions;

-- Each transcript message priced at its model's API list price
-- (docs/spec/daemon/session-api-cost.md C4). The price row is the one for the
-- message's model (date suffix stripped), speed, prompt band (the highest band
-- its prompt is over) and day. No row → cost NULL, never 0.
create or replace view activity.message_api_cost as
select m.source
     , m.session_id
     , m.message_id
     , m.turn_index
     , m.sent_at
     , m.model
     , ( m.input          * p.input
       + m.cache_write_5m * p.cache_write_5m
       + m.cache_write_1h * p.cache_write_1h
       + m.cache_read     * p.cache_read
       + m.output         * p.output
       ) / 1000000
       * case when m.inference_geo = 'us' then p.us_multiplier else 1 end as cost
  from activity.transcript_messages m
  left join lateral (
       select mp.*
         from sensei.model_prices mp
        where mp.model = regexp_replace(m.model, '-[0-9]{8}$', '')
          and mp.speed = coalesce(m.speed, 'standard')
          and (mp.prompt_over = 0
               or m.input + m.cache_read + m.cache_write_5m + m.cache_write_1h > mp.prompt_over)
          and (mp.effective_from is null or mp.effective_from <= (m.sent_at at time zone 'UTC')::date)
          and (mp.effective_to   is null or (m.sent_at at time zone 'UTC')::date < mp.effective_to)
          and m.sent_at is not null
        order by mp.prompt_over desc
        limit 1
       ) p on true;

comment on view activity.message_api_cost is
'Each transcript message at its model''s API list price; cost NULL when no price row matches.';
