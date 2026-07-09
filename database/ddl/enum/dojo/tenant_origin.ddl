set search_path to dojo, extensions;

-- How a tenant Dōjō's identity was established, per the discovery URL
-- structure dojo.sensei-hq.org/<origin>/<org>/<dojo?>. `github` = the tenant
-- is backed by a GitHub org identity; `org` = a custom-registered name (also
-- used by the special global-dojo tenant).
create type dojo.tenant_origin
    as enum ('github', 'org');
