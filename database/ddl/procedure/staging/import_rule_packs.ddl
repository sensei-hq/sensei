set search_path to staging, sensei, extensions;

-- Seed the GLOBAL rule-pack library from staging.rule_packs (dbd auto-runs this on
-- import). Every seeded pack is owner_namespace_id = NULL (global curated library),
-- status defaults 'active' (visible/adoptable) and is LIBRARY-ONLY — this proc never
-- adopts a pack (no rule_pack_adoptions write), so packs are opt-in, never forced.
--
-- Incremental, not a full reload: on conflict (the global-slug partial index) it
-- updates ONLY when the datafile's modified_at is STRICTLY newer than the live row's
-- updated_at. So re-importing unchanged data is a no-op, and a pack edited in prod
-- (updated_at bumped past the datafile) is never clobbered — mirrors import_models'
-- guard, tightened to `>` so unchanged rows are skipped outright.
create or replace procedure import_rule_packs()
language plpgsql
set search_path = staging, sensei, extensions
as $$
begin
  insert into sensei.rule_packs (
      slug, name, kanji, area, attribution, summary,
      enforcement, owner_namespace_id, status, published_by, updated_at
  )
  select
      stg.slug
    , stg.name
    , nullif(stg.kanji, '')
    , stg.area::sensei.rule_pack_area
    , stg.source
    , nullif(stg.summary, '')
    , coalesce(nullif(stg.enforcement, ''), 'recommended')::sensei.enforcement
    , null                                            -- global library (never owned)
    , coalesce(nullif(stg.status, ''), 'active')
    , coalesce(nullif(stg.published_by, ''), 'sensei')
    , coalesce(stg.modified_at, now())
  from staging.rule_packs stg
  where stg.slug is not null
  on conflict (slug) where owner_namespace_id is null
  do update set
      name         = excluded.name
    , kanji        = excluded.kanji
    , area         = excluded.area
    , attribution  = excluded.attribution
    , summary      = excluded.summary
    , enforcement  = excluded.enforcement
    , status       = excluded.status
    , published_by = excluded.published_by
    , updated_at   = excluded.updated_at
  where excluded.updated_at > sensei.rule_packs.updated_at;
end;
$$;
