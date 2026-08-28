set search_path to staging, sensei, extensions;

-- Seed the reason vocabulary from staging.reason_codes.
--
-- Same timestamp guard as import_schedules/import_tenants: deploy order is
-- apply → import, so the seed otherwise has the last word and a hand-edited
-- summary would be reverted on the next deploy with nothing to show why.
create or replace procedure import_reason_codes()
language plpgsql
set search_path = staging, sensei, extensions
as $$
begin
  insert into sensei.reason_codes (
      domain, code, kind, precedence, summary, detail, remedy, actor, modified_at
  )
  select
      stg.domain
    , stg.code
    , stg.kind::sensei.reason_kind
    , stg.precedence
    , stg.summary
    , stg.detail
    -- Empty string in a datafile means "no remedy", not a remedy of "". The
    -- CHECK on a `normal` row would reject the latter, and the error would point
    -- at the constraint rather than at the datafile that caused it.
    , nullif(stg.remedy, '')
    , nullif(stg.actor, '')::sensei.reason_actor
    , coalesce(stg.modified_at, now())
    from staging.reason_codes stg
   where stg.domain is not null
     and stg.code   is not null
  on conflict (domain, code) do update set
      kind        = excluded.kind
    , precedence  = excluded.precedence
    , summary     = excluded.summary
    , detail      = excluded.detail
    , remedy      = excluded.remedy
    , actor       = excluded.actor
    , modified_at = excluded.modified_at
   where excluded.modified_at >= sensei.reason_codes.modified_at;
end;
$$;
