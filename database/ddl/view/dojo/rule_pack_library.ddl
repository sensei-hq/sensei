set search_path to dojo, sensei, extensions;

-- The global rule-pack LIBRARY, as the dōjō API reads it (browse). A dojo-schema
-- view over the shared sensei rule-pack tables so the API never needs a grant on
-- `sensei.*`: a view runs with its OWNER's privileges (postgres, via dbd), so the
-- caller (service_role / authenticated) reads the view alone — the reason the
-- shared `sensei.*` DDL can't (and needn't) grant supabase roles. Exposes exactly
-- the browse fields, filtered to the curated global library (owner_namespace_id
-- NULL, status 'active'), with each pack's rule statements pre-aggregated in
-- ordinal order. Adoption is per-namespace via sensei.rule_pack_adoptions, not here.
create or replace view dojo.rule_pack_library as
select
    p.slug
  , p.kanji
  , p.name
  , p.source
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
'Global rule-pack library for the dōjō browse (GET /v1/you/rule-packs): the curated
global packs (owner_namespace_id NULL, status active) with each pack''s rule
statements as a jsonb array in ordinal order. A dojo-schema view over sensei.* so
the API reads it with the view owner''s privileges — no grant on sensei.rule_pack*
(which the shared cross-plane DDL cannot grant, the daemon having no supabase roles).';

grant select on dojo.rule_pack_library to authenticated;
