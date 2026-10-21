set search_path to dojo, extensions;

-- What KIND of tenant this is — not which forge it came from.
--
-- The forge lives on dojo.tenant_connections, because an organization may be
-- known to several (GitHub and Azure and GitLab are one dōjō, one subscription,
-- one governance set). An origin naming the forge was the trap this replaces.
--
-- The discovery path is `<origin>/<slug>`, so these values are user-visible and
-- keep the namespaces apart without a sigil:
--   personal/dev        organization/sensei-hq
--
-- A dōjō is only ever one of these two (#307 D11, user 2026-10-21): a person's,
-- or a forge-PROVEN organisation's (provisioning, and nothing else). There is
-- no hand-made dōjō — the `team` origin is gone; a team is a grouping of people
-- inside a dōjō (D7).
create type dojo.tenant_origin
    as enum ('personal', 'organization');
