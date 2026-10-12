set search_path to dojo, extensions;

-- What KIND of tenant this is — not which forge it came from.
--
-- The forge lives on dojo.tenant_connections, because an organization may be
-- known to several (GitHub and Azure and GitLab are one dōjō, one subscription,
-- one governance set). An origin naming the forge was the trap this replaces.
--
-- The discovery path is `<origin>/<slug>`, so these values are user-visible and
-- keep the namespaces apart without a sigil:
--   personal/dev        organization/sensei-hq        team/platform-guild
--
-- `organization` is a forge-PROVEN organisation (provisioning) and nothing else;
-- a dōjō someone creates by hand is a `team`, so no one can hold an
-- organisation's natural name ahead of it (decision 2026-10-12, GHSA-hqv4).
-- A team has no forge connection (only provisioning makes one, for a proven
-- organisation), so it holds no repositories.
create type dojo.tenant_origin
    as enum ('personal', 'organization', 'team');
