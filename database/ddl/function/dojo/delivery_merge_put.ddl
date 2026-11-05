set search_path to dojo, extensions;

-- Add one merged figure to a {values, reasons, partial} accumulator (spec
-- tracker-delivery-metrics A3; #353): the value always; its reason only when it
-- is null; its partial count only when some contributor was unknown.
create or replace function dojo.delivery_merge_put(
    p_acc     jsonb,
    p_field   text,
    p_value   jsonb,
    p_reason  text,
    p_partial integer
)
    returns jsonb
    language sql
    immutable
    set search_path = dojo, pg_temp
as $$
    select jsonb_build_object(
        'values',  (p_acc -> 'values') || jsonb_build_object(p_field, p_value),
        'reasons', case when p_value is null or jsonb_typeof(p_value) = 'null'
                        then (p_acc -> 'reasons') || jsonb_build_object(p_field, p_reason)
                        else p_acc -> 'reasons' end,
        'partial', case when p_partial > 0
                        then (p_acc -> 'partial') || jsonb_build_object(p_field, p_partial)
                        else p_acc -> 'partial' end);
$$;

revoke all on function dojo.delivery_merge_put(jsonb, text, jsonb, text, integer) from public;

comment on function dojo.delivery_merge_put(jsonb, text, jsonb, text, integer) is
'Adds one merged figure to a {values, reasons, partial} accumulator: a null carries its reason, a partial figure its count of unknown contributors. Pure (#353).';
