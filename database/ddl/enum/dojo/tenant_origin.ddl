set search_path to dojo, extensions;

-- What KIND of tenant this is — not which forge it came from.
--
-- The forge lives on dojo.tenant_connections, because an organization may be
-- known to several (GitHub and Azure and GitLab are one dōjō, one subscription,
-- one governance set). An origin naming the forge was the trap this replaces.
--
-- The discovery path is `<origin>/<slug>`, so these values are user-visible and
-- keep the two namespaces apart without a sigil:
--   personal/jerry        organization/sensei-hq
create type dojo.tenant_origin
    as enum ('personal', 'organization');
