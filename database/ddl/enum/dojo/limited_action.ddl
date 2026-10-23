set search_path to dojo, extensions;

-- An action a person may take only so many times in a window (dojo.request_attempts;
-- #311, generalised in #308 R5b): `invite_accept` counts FAILED invite accepts;
-- `azure_connect` counts every step of the Azure DevOps connect flow (start,
-- callback, organization list, import).
create type dojo.limited_action
    as enum ('invite_accept', 'azure_connect');
