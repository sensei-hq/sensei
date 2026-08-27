set search_path to dojo, extensions;

-- What KIND of tenant this is — not which forge it came from.
--
-- `github`/`org` are the retired GitHub-era labels. They named the forge, which
-- stopped being right once a tenant could connect to several
-- (dojo.tenant_connections). They remain in the type only until the data
-- migration in apply/after/tenant_origin_migration.sql has run everywhere; the
-- narrowing to two values is a separate, later change so no deploy is ever
-- caught with rows holding a value the type no longer has.
--
-- The discovery path is `<origin>/<slug>`, so these values are user-visible:
--   personal/jerry        organization/sensei-hq
create type dojo.tenant_origin
    as enum ('github', 'org', 'personal', 'organization');
