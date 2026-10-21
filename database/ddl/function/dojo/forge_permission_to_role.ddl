set search_path to dojo, extensions;

-- The forge's per-repo permission as a dōjō role (#307 D5): read, triage and write
-- are contributor; maintain is maintainer; admin is admin. `lead` never comes from
-- the forge — it is the tenant's client-confidentiality role. Anything else maps to
-- nothing.
create or replace function dojo.forge_permission_to_role(p_permission text)
    returns dojo.member_role
    language sql
    immutable
as $$
    select case lower(p_permission)
        when 'read' then 'contributor'::dojo.member_role
        when 'triage' then 'contributor'::dojo.member_role
        when 'write' then 'contributor'::dojo.member_role
        when 'maintain' then 'maintainer'::dojo.member_role
        when 'admin' then 'admin'::dojo.member_role
    end;
$$;

comment on function dojo.forge_permission_to_role(text) is
'The forge''s per-repo permission as a dōjō role (#307 D5): read/triage/write →
contributor, maintain → maintainer, admin → admin; otherwise null.';
