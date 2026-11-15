set search_path to dojo, extensions;

-- The words of the sdlc_metrics reason codes a figures object cites, code →
-- summary (spec tracker-delivery-metrics D5; #352, #353). NULL when it cites none.
create or replace function dojo.delivery_reason_words(p_reasons jsonb)
    returns jsonb
    language sql
    stable
    set search_path = dojo, pg_temp
as $$
    select jsonb_object_agg(r.code, r.summary)
      from sensei.reason_codes r
     where r.domain = 'sdlc_metrics'
       and r.code in (select x.value
                        from jsonb_each_text(case when jsonb_typeof(p_reasons) = 'object' then p_reasons end) x);
$$;

revoke all on function dojo.delivery_reason_words(jsonb) from public;

comment on function dojo.delivery_reason_words(jsonb) is
'code → summary for every sdlc_metrics reason a {figure: code} object cites; NULL when none (#352, #353).';
