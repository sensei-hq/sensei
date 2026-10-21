set search_path to dojo, extensions;

-- How a membership came to be (#309). `provisioned`: org provisioning made it,
-- because the forge proved the person a member of the org — so leaving the org
-- ends it (dojo.reconcile_org_memberships). `invited`: an admin invited the
-- person (a contractor, a client) — the forge says nothing about them, and
-- provisioning never touches it.
create type dojo.membership_source
    as enum ('provisioned', 'invited');
