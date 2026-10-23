set search_path to dojo, extensions;

-- Bring an organization dōjō's members in line with a COMPLETE read of the
-- organization's members on one forge (#308 R5b; #309's rule, read from the
-- organization's side — an Azure DevOps organization is read with an admin's
-- stored token, not each person's own).
--
-- p_unit is the unit read (its stable id); p_subjects are the forge's ids for
-- everyone the read listed; p_auth is the identity kind those ids are recorded
-- under (dojo.identities.provider). Each ACTIVE membership THIS UNIT provisioned
-- (source provisioned, source_provider = p_provider, source_unit = p_unit) in
-- p_tenant — an organization dōjō connected to p_provider — whose person holds an
-- identity of that kind, none of them listed, ends (left_org), and with it any
-- admin or lead power. A membership this unit provisioned that ended by leaving,
-- whose person is listed again, is restored — never one another forge or unit
-- made or ended (#308 R5b review). Invited memberships, people with no identity
-- of that kind, personal dōjōs and dōjōs not connected to p_provider are never
-- touched.
--
-- The complete read is also KEPT and FOLLOWED (#308 R5c, D45), when p_unit is
-- connected to p_tenant:
--   * the connection's member set (dojo.connection_members) becomes this list —
--     what a sign-in that cannot read the forge (Microsoft) provisions from;
--   * every listed person who already holds an identity of that kind (signed in
--     earlier) and has no membership in the dōjō at all is provisioned:
--     contributor (a member read proves no admin standing — the forge-read role
--     rule), employer, from this forge and unit. An existing membership, active
--     or ended, invited or another forge's, always wins.
--
-- p_directories, when given, lines up with p_subjects: each member's directory
-- (for Azure DevOps the Entra tenant id, GraphUser.domain), kept in the set so a
-- sign-in is matched only from that tenant (#308 R5c review). NULL for a member
-- with none, who no sign-in then matches; a list of another length is refused.
--
-- A NULL or empty list is refused: only a complete read may end anything (#309),
-- and an organization read with a member's token always lists that member. The
-- refusal leaves the stored set as it was. Returns how many memberships it ended
-- and how many it joined. service_role only: the server calls it after a
-- complete forge read, never on a failed or partial one.
drop function if exists dojo.reconcile_unit_members(uuid, dojo.forge_provider, dojo.auth_method, text[]);
create or replace function dojo.reconcile_unit_members(
    p_tenant      uuid,
    p_provider    dojo.forge_provider,
    p_unit        text,
    p_auth        dojo.auth_method,
    p_subjects    text[],
    p_directories text[] default null
)
    returns table (ended integer, joined integer)
    language plpgsql
    volatile
    security definer
    set search_path = dojo, pg_temp
as $$
declare
    v_ended  integer;
    v_joined integer := 0;
    v_conn   uuid;
begin
    if p_subjects is null or cardinality(p_subjects) = 0 then
        raise exception 'a complete member list is needed to end a membership'
            using errcode = 'invalid_parameter_value';
    end if;
    if p_directories is not null and cardinality(p_directories) <> cardinality(p_subjects) then
        raise exception 'the member directories must line up with the members'
            using errcode = 'invalid_parameter_value';
    end if;

    if not exists (
        select 1 from dojo.tenants t
         where t.id = p_tenant and t.origin = 'organization'
           and exists (select 1 from dojo.tenant_connections c where c.tenant_id = t.id and c.provider = p_provider)
    ) then
        return query select 0, 0;
        return;
    end if;

    update dojo.memberships m
       set disabled_at = now(), disabled_reason = 'left_org', updated_at = now()
     where m.tenant_id = p_tenant
       and m.source = 'provisioned'
       and m.source_provider = p_provider
       and m.source_unit = p_unit
       and m.disabled_at is null
       and exists (select 1 from dojo.identities i where i.principal_id = m.user_id and i.provider = p_auth)
       and not exists (select 1 from dojo.identities i
                        where i.principal_id = m.user_id and i.provider = p_auth and i.subject = any (p_subjects));
    get diagnostics v_ended = row_count;

    update dojo.memberships m
       set disabled_at = null, disabled_reason = null, updated_at = now()
     where m.tenant_id = p_tenant
       and m.disabled_reason = 'left_org'
       and m.source_provider = p_provider
       and m.source_unit = p_unit
       and exists (select 1 from dojo.identities i
                    where i.principal_id = m.user_id and i.provider = p_auth and i.subject = any (p_subjects));

    select c.id into v_conn
      from dojo.tenant_connections c
     where c.tenant_id = p_tenant and c.provider = p_provider and c.external_id = p_unit;

    if v_conn is not null then
        delete from dojo.connection_members where connection_id = v_conn;
        update dojo.tenant_connections set member_set_read_at = now() where id = v_conn;
        insert into dojo.connection_members (connection_id, subject, directory, read_at)
        select distinct on (m.s) v_conn, m.s, m.d, now()
          from unnest(p_subjects, coalesce(p_directories, array_fill(null::text, array[cardinality(p_subjects)]))) as m(s, d)
         order by m.s, m.d nulls last;

        insert into dojo.memberships
            (tenant_id, user_id, role, kind, authenticated_via, source, source_provider, source_unit)
        select distinct p_tenant, i.principal_id, 'contributor'::dojo.member_role, 'employer'::dojo.membership_kind,
               p_auth, 'provisioned'::dojo.membership_source, p_provider, p_unit
          from dojo.identities i
         where i.provider = p_auth
           and i.subject = any (p_subjects)
           and not exists (select 1 from dojo.memberships m
                            where m.tenant_id = p_tenant and m.user_id = i.principal_id)
        on conflict (tenant_id, user_id) do nothing;
        get diagnostics v_joined = row_count;
    end if;

    return query select v_ended, v_joined;
end;
$$;

revoke all on function dojo.reconcile_unit_members(uuid, dojo.forge_provider, text, dojo.auth_method, text[], text[]) from public, anon, authenticated;
grant execute on function dojo.reconcile_unit_members(uuid, dojo.forge_provider, text, dojo.auth_method, text[], text[]) to service_role;

comment on function dojo.reconcile_unit_members(uuid, dojo.forge_provider, text, dojo.auth_method, text[], text[]) is
'End the memberships one forge unit provisioned in an organization dōjō whose person a complete
member read of that unit no longer lists, and restore those that ended by leaving and are
listed again (#308 R5b, #309). Keep the list as the connection''s member set and provision the
listed people who hold the identity but have no membership (#308 R5c). Refuses an empty list.
Returns (ended, joined). service_role only.';
