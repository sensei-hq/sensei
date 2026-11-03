-- The Supabase security advisor's checks, held as tests (user, 2026-10-27:
-- the production dōjō reported tables without RLS — sensei.metrics, critical —
-- and functions with a mutable search_path).
--
-- Every table the dōjō database carries — its own schema and the sensei /
-- staging tables the `dojo` scope brings — has row level security on, so a
-- table reachable through the API is never open by default. Every function and
-- procedure there pins its search_path, so a caller's search_path cannot
-- redirect the names it resolves.
--
-- WHAT BREAKS THIS TEST: a new table without `enable row level security`; a new
-- function without `set search_path = …`.

do $$
declare
    open_tables text;
    mutable_fns text;
begin
    select string_agg(schemaname || '.' || tablename, ', ' order by 1) into open_tables
      from pg_tables
     where schemaname in ('dojo', 'sensei', 'staging') and not rowsecurity;
    if open_tables is not null then
        raise exception 'tables without row level security: %', open_tables;
    end if;

    select string_agg(n.nspname || '.' || p.proname, ', ' order by 1) into mutable_fns
      from pg_proc p
      join pg_namespace n on n.oid = p.pronamespace
     where n.nspname in ('dojo', 'sensei', 'staging')
       and p.prokind in ('f', 'p')
       and not exists (select 1 from unnest(coalesce(p.proconfig, '{}')) c where c like 'search_path=%');
    if mutable_fns is not null then
        raise exception 'functions with a mutable search_path: %', mutable_fns;
    end if;
end
$$;

-- RLS on the two catalogues keeps them readable: a signed-in person still sees
-- every metric and every reason through the dōjō's views (which run as the
-- caller). An RLS switch with no read policy would empty both, silently.
begin;
set local role authenticated;
do $$
declare
    metrics int;
    reasons int;
begin
    select count(*) into metrics from dojo.metric_catalogue;
    select count(*) into reasons from dojo.reason_codes;
    if metrics = 0 or reasons = 0 then
        raise exception 'catalogue unreadable to a signed-in person: % metrics, % reasons', metrics, reasons;
    end if;
end
$$;
rollback;
