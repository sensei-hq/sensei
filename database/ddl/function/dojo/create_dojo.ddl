set search_path to dojo, extensions;

-- Create a dōjō the signed-in person owns (POST /v1/you/dojos, issue #117; #236).
--
-- One transaction: the `organization` tenant and the creator's admin membership,
-- so a failed membership never leaves a tenant nobody administers. The member is
-- the SESSION's principal (dojo.current_principal_id), never a user the caller
-- names; no session → insufficient_privilege. The key and url are built HERE —
-- a caller-supplied url would let a dōjō's creator point its members' daemons
-- at any host. The slug is the caller's (admin-data.ts slugify) and must be one
-- (invalid_parameter_value otherwise). A taken key is the unique_violation the
-- route maps to 409.
--
-- The key's first segment is the ORIGIN: dojo-auth.ts resolves a tenant by
-- joining the two URL segments back into `{origin}/{slug}`, so the prefix and
-- `origin` must agree or the tenant is unreachable at its own URL (spec §IV.7).
create or replace function dojo.create_dojo(p_name text, p_slug text, p_kind dojo.membership_kind)
    returns table (id uuid, key text, name text)
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
#variable_conflict use_column
declare
    v_principal uuid := dojo.current_principal_id();
    v_key       text := 'organization/' || p_slug;
    v_id        uuid;
begin
    if v_principal is null then
        raise exception 'a signed-in session is required' using errcode = 'insufficient_privilege';
    end if;
    if p_slug is null or p_slug !~ '^[a-z0-9]+(-[a-z0-9]+)*$' then
        raise exception 'invalid dōjō slug %', p_slug using errcode = 'invalid_parameter_value';
    end if;

    insert into dojo.tenants (key, origin, slug, name, dojo_url, scope)
    values (v_key, 'organization', p_slug, p_name, 'dojo.sensei-hq.org/' || v_key, 'private')
    returning tenants.id into v_id;

    insert into dojo.memberships (tenant_id, user_id, kind, authenticated_via, role)
    values (v_id, v_principal, p_kind, 'sso', 'admin');

    return query select v_id, v_key, p_name;
end;
$$;

revoke all on function dojo.create_dojo(text, text, dojo.membership_kind) from public;
grant execute on function dojo.create_dojo(text, text, dojo.membership_kind) to authenticated;

comment on function dojo.create_dojo(text, text, dojo.membership_kind) is
'Create an organization dōjō with the signed-in person as its admin, in one
transaction. The member is the session''s principal; the key and url are built
here. insufficient_privilege with no session; invalid_parameter_value for a bad
slug; unique_violation for a taken key (#236).';
