set search_path to dojo, extensions;

-- How a membership authenticated when it was paired. Human console traffic
-- arrives via `sso` (OIDC/SAML) or `github_oauth`; a desktop/daemon pairing
-- uses `device_code` (a per-membership device token pasted into the app).
-- `azure_devops_pat`: an Azure DevOps account proven by the personal access token
-- an organization admin pasted (#308 R5b, D35) — the identity it read, and the
-- membership an import made from it. No anonymous access — every membership
-- records how it was established.
create type dojo.auth_method
    as enum ('sso', 'github_oauth', 'device_code', 'azure_devops_pat');
