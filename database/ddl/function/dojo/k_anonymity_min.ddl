set search_path to dojo, extensions;

-- The dōjō's k-anonymity floor: the fewest distinct people an aggregate may
-- describe before it is answered (#307 R8 follow-up; D26 keeps k-anonymity ≥ 3).
-- The same floor the knowledge promotion gate applies
-- (dojo/src/lib/server/artifacts-data.ts K_ANONYMITY_MIN_CONTRIBUTORS, pinned
-- equal by team-metrics-data.spec.ts). A function rather than an argument, so no
-- caller can lower it.
create or replace function dojo.k_anonymity_min()
    returns integer
    language sql
    immutable
    set search_path = dojo, pg_temp
as $$
    select 3;
$$;

grant execute on function dojo.k_anonymity_min() to authenticated;

comment on function dojo.k_anonymity_min() is
'The fewest distinct people a team aggregate may describe (3) — the dōjō''s k-anonymity floor (D26, K_ANONYMITY_MIN_CONTRIBUTORS).';
