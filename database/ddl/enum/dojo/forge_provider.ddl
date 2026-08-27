set search_path to dojo, extensions;

-- Which source forge a tenant connection speaks to.
--
-- Deliberately a type rather than free text: a connection's provider decides
-- how its external ids are shaped and which API proves org membership, so an
-- unrecognised value is a bug, not data.
create type dojo.forge_provider
    as enum ('github', 'gitlab', 'bitbucket', 'azure_devops');
