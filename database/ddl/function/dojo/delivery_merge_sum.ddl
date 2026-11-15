set search_path to dojo, extensions;

-- One field merged across contributors' effort rows (spec tracker-delivery-metrics
-- D1, A3; #353). p_rows is a json array of {values, reasons}. p_op is 'sum', 'min'
-- or 'max' over the known values (numbers, or timestamps for min/max).
--
-- Unknown is not zero: a null whose reason is not_started or not_submitted does
-- not apply to that contributor and leaves them out; any other null is unknown.
-- The answer: the merged value (as text, null when no contributor knew it); its
-- reason when null — the contributors' reason when they agree, else
-- no_contributor_value; and partial, the unknown contributors when some knew it.
create or replace function dojo.delivery_merge_sum(p_rows jsonb, p_field text, p_op text)
    returns table (merged text, reason text, partial integer)
    language sql
    immutable
    set search_path = dojo, pg_temp
as $$
    with x as (
        select r -> 'values' -> p_field as v, r -> 'reasons' ->> p_field as why
          from jsonb_array_elements(p_rows) r
    ), k as (
        select count(*) filter (where v is not null and jsonb_typeof(v) <> 'null') as known,
               count(*) filter (where (v is null or jsonb_typeof(v) = 'null')
                                  and (why is null or why not in ('not_started', 'not_submitted'))) as unknown,
               array_agg(distinct why) filter (where (v is null or jsonb_typeof(v) = 'null')
                                                  and why is not null and why not in ('not_started', 'not_submitted')) as whys,
               bool_or(why = 'not_submitted') filter (where v is null or jsonb_typeof(v) = 'null') as any_unsubmitted,
               sum((v #>> '{}')::numeric) filter (where jsonb_typeof(v) = 'number') as total,
               min((v #>> '{}')::timestamptz) filter (where jsonb_typeof(v) = 'string') as earliest,
               max((v #>> '{}')::timestamptz) filter (where jsonb_typeof(v) = 'string') as latest,
               min((v #>> '{}')::numeric) filter (where jsonb_typeof(v) = 'number') as least_n,
               max((v #>> '{}')::numeric) filter (where jsonb_typeof(v) = 'number') as most_n
          from x
    )
    select case when known = 0 then null
                when p_op = 'sum' then total::text
                when p_op = 'min' then case when earliest is not null then earliest::text else least_n::text end
                when p_op = 'max' then case when latest is not null then latest::text else most_n::text end
           end,
           case when known > 0 then null
                -- the unknowns' reason; with none, the not-applicable one
                when cardinality(whys) = 1 then whys[1]
                when cardinality(whys) > 1 then 'no_contributor_value'
                when any_unsubmitted then 'not_submitted'
                when unknown = 0 then 'not_started'
                else 'no_contributor_value' end,
           case when known > 0 then unknown::integer else 0 end
      from k;
$$;

revoke all on function dojo.delivery_merge_sum(jsonb, text, text) from public;

comment on function dojo.delivery_merge_sum(jsonb, text, text) is
'One effort field merged across contributors (D1, A3): sum, min or max over the known values; null with the contributors'' shared reason or no_contributor_value when none knew it; partial = the unknown contributors (not_started / not_submitted do not apply and are not unknown). Pure (#353).';
