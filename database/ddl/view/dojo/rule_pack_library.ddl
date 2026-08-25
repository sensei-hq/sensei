set search_path to dojo, sensei, extensions;

-- The global rule-pack LIBRARY, as the dōjō API reads it (browse). A dojo-schema view
-- over the shared sensei rule-pack tables, exposing exactly the browse fields, filtered to
-- the curated global library (owner_namespace_id NULL, status 'active'), with each pack's
-- rule statements pre-aggregated in ordinal order. Adoption is per-namespace via
-- sensei.rule_pack_adoptions, not here.
--
-- SECURITY: `security_invoker = on` — the view runs with the CALLING role's privileges, so
-- it obeys the caller's grants + RLS rather than the owner's (Supabase lint
-- security_definer_view). For the caller to read the sensei.* base tables, `authenticated`
-- is granted SELECT on sensei.rule_packs / sensei.rule_pack_rules, and those tables carry
-- RLS policies that expose ONLY the global library (owner_namespace_id NULL, active) — so
-- even a direct PostgREST query on the base tables can't reach private/org packs. On the
-- local daemon (pure Postgres) those grants/RLS are inert: it connects as the object owner,
-- which bypasses RLS, and never uses the `authenticated` role.
create or replace view dojo.rule_pack_library
with (security_invoker = on)
as
select
    p.slug
  , p.kanji
  , p.name
  , p.attribution
  , p.summary
  , coalesce(
        (select jsonb_agg(r.statement order by r.ordinal)
           from sensei.rule_pack_rules r
          where r.pack_id = p.id),
        '[]'::jsonb
    ) as rules
from sensei.rule_packs p
where p.owner_namespace_id is null
  and p.status = 'active';

comment on view dojo.rule_pack_library is
'Global rule-pack library for the dōjō browse (GET /v1/you/rule-packs): the curated global
packs (owner_namespace_id NULL, status active) with each pack''s rule statements as a jsonb
array in ordinal order. security_invoker = on — runs as the calling role, so RLS on the
sensei.rule_pack* base tables (global-library-only) governs what it can read; the WHERE here
re-asserts the same filter. No dependence on view-owner privileges.';

grant select on dojo.rule_pack_library to authenticated;
