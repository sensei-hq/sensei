set search_path to dojo, extensions;

-- How a membership authenticated when it was paired. Human console traffic
-- arrives via `sso` (OIDC/SAML) or `github_oauth`; a desktop/daemon pairing
-- uses `device_code` (a per-membership device token pasted into the app). No
-- anonymous access — every membership records how it was established.
create type dojo.auth_method
    as enum ('sso', 'github_oauth', 'device_code');
