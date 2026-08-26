set search_path to staging, extensions;

-- Landing table for the adoptions that make a library pack actually govern.
--
-- A pack sitting in sensei.rule_packs governs nothing until a namespace adopts
-- it. seed_default_constitution() used to do both in one procedure; splitting the
-- content into a datafile without this would have shipped the constitution and
-- left it unadopted — packs present, get_rules returning nothing.
--
-- The namespace is identified by (scope_key, slug) and carries a FIXED id: a
-- namespace created by an import would otherwise get a fresh uuid per plane and
-- per reset, and adoptions on one plane would point at a namespace that does not
-- exist on another.
drop table if exists rule_pack_adoptions cascade;
create table rule_pack_adoptions (
  pack_slug        text
, namespace_id     text        -- explicit and FIXED
, namespace_scope  text
, namespace_slug   text
, namespace_name   text
, adopted_by       text
, modified_at      timestamptz not null default now()
);
