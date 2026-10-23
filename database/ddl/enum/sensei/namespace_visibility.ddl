set search_path to sensei, extensions;

-- Whether a namespace (in practice a workspace) is private or public. Drives
-- tenant billing: a member counts toward a tenant's billable seats only through
-- participation in a `private` namespace; `public` work (open-source, shared
-- reference workspaces) is free and never consumes a seat. Meaningful for
-- workspace-scope namespaces; other rungs (organization/repository/…) carry the
-- default and are ignored by billing. `private` is the default — you opt INTO
-- public, never accidentally out of a paid/confidential posture.
create type namespace_visibility
    as enum ('private', 'public');
