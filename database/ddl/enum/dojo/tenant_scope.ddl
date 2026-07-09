set search_path to dojo, extensions;

-- Visibility of a tenant Dōjō. `private` = a normal org/client Dōjō visible
-- only to its members. `global` = the single special-case global-dojo tenant
-- everyone may join; there is no separate "Collective" concept in the schema —
-- it is just the Dōjō at scope global.
create type dojo.tenant_scope
    as enum ('private', 'global');
