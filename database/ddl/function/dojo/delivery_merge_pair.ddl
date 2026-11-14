set search_path to dojo, extensions;

-- A ratio's numerator and denominator summed over the SAME contributors — those
-- who know both (spec tracker-delivery-metrics A3; #353) — so a contributor who
-- has a denominator but no numerator never dilutes the ratio. p_rows is a json
-- array of {values, reasons}. partial counts the contributors left out as unknown
-- (a null whose reason is not_started or not_submitted does not apply to them).
create or replace function dojo.delivery_merge_pair(p_rows jsonb, p_num text, p_den text)
    returns table (num numeric, den numeric, partial integer, known integer)
    language sql
    immutable
    set search_path = dojo, pg_temp
as $$
    with x as (
        select r -> 'values' -> p_num as n, r -> 'values' -> p_den as d,
               r -> 'reasons' ->> p_num as why_n, r -> 'reasons' ->> p_den as why_d
          from jsonb_array_elements(p_rows) r
    )
    select sum((n #>> '{}')::numeric) filter (where jsonb_typeof(n) = 'number' and jsonb_typeof(d) = 'number'),
           sum((d #>> '{}')::numeric) filter (where jsonb_typeof(n) = 'number' and jsonb_typeof(d) = 'number'),
           (count(*) filter (where (jsonb_typeof(n) is distinct from 'number'
                                     and (why_n is null or why_n not in ('not_started', 'not_submitted')))
                                or (jsonb_typeof(d) is distinct from 'number'
                                     and (why_d is null or why_d not in ('not_started', 'not_submitted')))))::integer,
           (count(*) filter (where jsonb_typeof(n) = 'number' and jsonb_typeof(d) = 'number'))::integer
      from x;
$$;

revoke all on function dojo.delivery_merge_pair(jsonb, text, text) from public;

comment on function dojo.delivery_merge_pair(jsonb, text, text) is
'A ratio''s numerator and denominator summed over the contributors who know both (A3), with partial = those left out as unknown and known = those summed. Pure (#353).';
