set search_path to staging, extensions;

-- Staging landing for the GLOBAL rule-pack library seed (import/staging/rule_packs.jsonl).
-- Flat, untyped-ish columns mirroring the datafile; import_rule_packs() casts + moves
-- them into sensei.rule_packs (always owner_namespace_id = NULL → global library) with
-- a strict timestamp guard so a re-import only touches packs whose `modified_at` is
-- newer than the live row (incremental — never a full reload, never clobbers a prod edit).
drop table if exists rule_packs cascade;
create table rule_packs (
  slug          text
, name          text
, kanji         text
, area          text        -- cast to sensei.rule_pack_area on import
, source        text
, summary       text
, enforcement   text        -- cast to sensei.enforcement on import (pack default)
, status        text        -- active | draft | archived
, published_by  text
, modified_at   timestamptz not null default now()
);
