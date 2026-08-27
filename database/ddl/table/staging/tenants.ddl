set search_path to staging, extensions;

-- Landing table for seeded dōjō tenants.
--
-- Only the bootstrap rows live here — the global-dōjō collective, which must
-- exist before anything can reference it. Real tenants are created by
-- provisioning at sign-in, never seeded.
--
-- Enum facets arrive as text and are cast to their dojo.<enum> types on import,
-- mirroring staging.rule_packs.
drop table if exists tenants cascade;
create table tenants (
  id            text        -- explicit and FIXED; see import_tenants
, key           text
, origin        text        -- cast to dojo.tenant_origin on import
, slug          text
, dojo          text
, scope         text        -- cast to dojo.tenant_scope on import
, name          text
, dojo_url      text
, self_hosted   boolean
, modified_at   timestamptz not null default now()
);
