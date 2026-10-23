-- A connection keeps its organization's last COMPLETE member set, and a complete
-- read provisions the listed people the dōjō can already name (#308 R5c, D45).
--
-- A Microsoft sign-in token cannot read Azure DevOps, so a sign-in is provisioned
-- from what the dōjō already knows: the member set its last complete member read
-- (import, Sync, the scheduled read) stored on the connection
-- (dojo.connection_members — the Entra object ids, and when they were read).
-- dojo.reconcile_unit_members, which only a complete read calls, now also:
--   * replaces that connection's set with the list (a partial read never calls
--     it, and a NULL or empty list is refused — the set stays);
--   * provisions every listed person who already holds the identity (signed in
--     earlier) but has no membership in the dōjō: contributor, employer, from
--     that forge and unit — the role rule of a forge read that proves no admin.
-- dojo.units_listing_member(provider, subject) answers which organization dōjōs'
-- last complete set lists a person — what a Microsoft sign-in provisions into —
-- unless that set is older than dojo.member_set_max_age_hours().
--
-- WHAT BREAKS THIS TEST: a listed signed-in person not provisioned, or
-- provisioned with another role/source/unit; an unlisted one provisioned; a
-- partial (NULL/empty) read replacing the set or provisioning anyone; a
-- GitHub-provisioned or invited membership changed; another dōjō's members
-- touched or joined; a later read not replacing the set; the lookup naming a
-- dōjō whose set does not list the person, a personal dōjō, or another forge's;
-- the set outliving its connection; a stale set naming a dōjō, or a fresh one
-- not; a signed-in caller reading the set or the lookup.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000c51', 'a1000000-0000-4000-8000-000000000c51', 'ztest-cm-admin'),
  ('a0000000-0000-4000-8000-000000000c52', 'a1000000-0000-4000-8000-000000000c52', 'ztest-cm-signed-in'),
  ('a0000000-0000-4000-8000-000000000c53', 'a1000000-0000-4000-8000-000000000c53', 'ztest-cm-unlisted'),
  ('a0000000-0000-4000-8000-000000000c54', 'a1000000-0000-4000-8000-000000000c54', 'ztest-cm-gh-member'),
  ('a0000000-0000-4000-8000-000000000c55', 'a1000000-0000-4000-8000-000000000c55', 'ztest-cm-invited'),
  ('a0000000-0000-4000-8000-000000000c56', 'a1000000-0000-4000-8000-000000000c56', 'ztest-cm-other-dojo'),
  ('a0000000-0000-4000-8000-000000000c57', 'a1000000-0000-4000-8000-000000000c57', 'ztest-cm-late');
insert into dojo.identities (principal_id, provider, subject) values
  ('a0000000-0000-4000-8000-000000000c51', 'azure_devops', 'ztest-oid-1'),
  ('a0000000-0000-4000-8000-000000000c52', 'azure_devops', 'ztest-oid-2'),
  ('a0000000-0000-4000-8000-000000000c53', 'azure_devops', 'ztest-oid-3'),
  ('a0000000-0000-4000-8000-000000000c54', 'azure_devops', 'ztest-oid-4'),
  ('a0000000-0000-4000-8000-000000000c55', 'azure_devops', 'ztest-oid-5'),
  ('a0000000-0000-4000-8000-000000000c56', 'azure_devops', 'ztest-oid-6');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-000000000c5a', 'organization/ztest-cm-az', 'organization', 'ztest-cm-az', 'Azure org', 'x'),
  ('b0000000-0000-4000-8000-000000000c5b', 'organization/ztest-cm-other', 'organization', 'ztest-cm-other', 'Other org', 'x'),
  ('b0000000-0000-4000-8000-000000000c5c', 'personal/ztest-cm-me', 'personal', 'ztest-cm-me', 'Me', 'x');
insert into dojo.tenant_connections (id, tenant_id, provider, external_id, external_slug, connected_by) values
  ('c0000000-0000-4000-8000-000000000c5a', 'b0000000-0000-4000-8000-000000000c5a', 'azure_devops', 'ztest-cm-org', 'ztest-cm-az', 'a0000000-0000-4000-8000-000000000c51'),
  ('c0000000-0000-4000-8000-000000000c5b', 'b0000000-0000-4000-8000-000000000c5b', 'azure_devops', 'ztest-cm-org-2', 'ztest-cm-other', 'a0000000-0000-4000-8000-000000000c51'),
  ('c0000000-0000-4000-8000-000000000c5c', 'b0000000-0000-4000-8000-000000000c5a', 'github', 'ztest-cm-gh', 'ztest-cm-gh', 'a0000000-0000-4000-8000-000000000c51');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via, source, source_provider, source_unit) values
  ('b0000000-0000-4000-8000-000000000c5a', 'a0000000-0000-4000-8000-000000000c51', 'admin', 'employer', 'azure_devops', 'provisioned', 'azure_devops', 'ztest-cm-org'),
  ('b0000000-0000-4000-8000-000000000c5a', 'a0000000-0000-4000-8000-000000000c54', 'lead', 'employer', 'github_oauth', 'provisioned', 'github', 'ztest-cm-gh'),
  ('b0000000-0000-4000-8000-000000000c5a', 'a0000000-0000-4000-8000-000000000c55', 'maintainer', 'client', 'sso', 'invited', null, null),
  ('b0000000-0000-4000-8000-000000000c5b', 'a0000000-0000-4000-8000-000000000c56', 'contributor', 'employer', 'azure_devops', 'provisioned', 'azure_devops', 'ztest-cm-org-2');

-- A signed-in caller cannot read the sets or ask the lookup: they are the system's.
set local role authenticated;
do $$
begin
    begin
        perform 1 from dojo.connection_members;
        raise exception 'a signed-in caller read a member set';
    exception when insufficient_privilege then null;
    end;
    begin
        perform dojo.units_listing_member('azure_devops', 'ztest-oid-2');
        raise exception 'a signed-in caller ran the member-set lookup';
    exception when insufficient_privilege then null;
    end;
end $$;
reset role;

do $$
begin
    if has_function_privilege('anon', 'dojo.units_listing_member(dojo.forge_provider, text)', 'execute') then
        raise exception 'anon may run the member-set lookup';
    end if;
end $$;

set local role service_role;
do $$
declare
    r record;
begin
    -- The complete read lists the admin, the signed-in member, the GitHub member,
    -- the invited member, the other dōjō's member and someone who has never signed
    -- in. Nobody departed, so nobody ends; two are joined: the signed-in member and
    -- the other dōjō's member (here).
    select * into r from dojo.reconcile_unit_members('b0000000-0000-4000-8000-000000000c5a', 'azure_devops', 'ztest-cm-org', 'azure_devops',
                                                     array['ztest-oid-1', 'ztest-oid-2', 'ztest-oid-4', 'ztest-oid-5', 'ztest-oid-6', 'ztest-oid-7', 'ztest-oid-2']);
    if r.ended <> 0 or r.joined <> 2 then
        raise exception 'member set: expected 0 ended and 2 joined, got %', r;
    end if;
end $$;
reset role;

do $$
declare
    r record;
begin
    -- The connection holds the set: each listed id once, read now.
    if (select array_agg(subject order by subject) from dojo.connection_members
         where connection_id = 'c0000000-0000-4000-8000-000000000c5a')
       <> array['ztest-oid-1', 'ztest-oid-2', 'ztest-oid-4', 'ztest-oid-5', 'ztest-oid-6', 'ztest-oid-7'] then
        raise exception 'member set: the connection does not hold the complete list';
    end if;
    if exists (select 1 from dojo.connection_members
                where connection_id = 'c0000000-0000-4000-8000-000000000c5a' and read_at <> now()) then
        raise exception 'member set: read_at is not when it was read';
    end if;

    -- A listed signed-in person gets a membership, as a forge read provisions one.
    select * into r from dojo.memberships
     where tenant_id = 'b0000000-0000-4000-8000-000000000c5a' and user_id = 'a0000000-0000-4000-8000-000000000c52';
    if r is null or r.role <> 'contributor' or r.kind <> 'employer' or r.authenticated_via <> 'azure_devops'
       or r.source <> 'provisioned' or r.source_provider <> 'azure_devops' or r.source_unit <> 'ztest-cm-org'
       or r.disabled_at is not null then
        raise exception 'member set: the listed signed-in member was not provisioned as a contributor of the unit (%)', r;
    end if;

    -- An unlisted one does not.
    if exists (select 1 from dojo.memberships
                where tenant_id = 'b0000000-0000-4000-8000-000000000c5a' and user_id = 'a0000000-0000-4000-8000-000000000c53') then
        raise exception 'member set: an unlisted person was provisioned';
    end if;

    -- A GitHub-provisioned membership and an invited one are untouched.
    if not exists (select 1 from dojo.memberships
                    where tenant_id = 'b0000000-0000-4000-8000-000000000c5a' and user_id = 'a0000000-0000-4000-8000-000000000c54'
                      and role = 'lead' and source_provider = 'github' and source_unit = 'ztest-cm-gh' and disabled_at is null) then
        raise exception 'member set: a GitHub-provisioned membership was changed';
    end if;
    if not exists (select 1 from dojo.memberships
                    where tenant_id = 'b0000000-0000-4000-8000-000000000c5a' and user_id = 'a0000000-0000-4000-8000-000000000c55'
                      and role = 'maintainer' and source = 'invited' and source_provider is null and disabled_at is null) then
        raise exception 'member set: an invited membership was changed';
    end if;

    -- Another dōjō's members are untouched: nobody is joined there, and its own
    -- member keeps their membership as it was.
    if (select count(*) from dojo.memberships where tenant_id = 'b0000000-0000-4000-8000-000000000c5b') <> 1
       or not exists (select 1 from dojo.memberships
                       where tenant_id = 'b0000000-0000-4000-8000-000000000c5b' and user_id = 'a0000000-0000-4000-8000-000000000c56'
                         and source_unit = 'ztest-cm-org-2' and disabled_at is null) then
        raise exception 'member set: another dōjō''s members were touched';
    end if;
    if exists (select 1 from dojo.connection_members where connection_id = 'c0000000-0000-4000-8000-000000000c5b') then
        raise exception 'member set: another connection''s set was written';
    end if;
end $$;

set local role service_role;
do $$
declare
    n integer;
begin
    -- A partial read changes nothing: no list, or an empty one, is refused, and
    -- the set stays the last complete one.
    begin
        perform dojo.reconcile_unit_members('b0000000-0000-4000-8000-000000000c5a', 'azure_devops', 'ztest-cm-org', 'azure_devops', null);
        raise exception 'member set: a NULL list was accepted';
    exception when invalid_parameter_value then null;
    end;
    begin
        perform dojo.reconcile_unit_members('b0000000-0000-4000-8000-000000000c5a', 'azure_devops', 'ztest-cm-org', 'azure_devops', '{}'::text[]);
        raise exception 'member set: an empty list was accepted';
    exception when invalid_parameter_value then null;
    end;
    select count(*) into n from dojo.connection_members where connection_id = 'c0000000-0000-4000-8000-000000000c5a';
    if n <> 6 then
        raise exception 'member set: a refused read changed the set (% ids)', n;
    end if;

    -- The lookup: the dōjōs whose last complete set lists a person.
    if (select array_agg(unit) from dojo.units_listing_member('azure_devops', 'ztest-oid-2')) <> array['ztest-cm-org']
       or (select array_agg(tenant_id) from dojo.units_listing_member('azure_devops', 'ztest-oid-2'))
          <> array['b0000000-0000-4000-8000-000000000c5a'::uuid] then
        raise exception 'member set: the lookup does not name the dōjō whose set lists the person';
    end if;
    if exists (select 1 from dojo.units_listing_member('azure_devops', 'ztest-oid-3'))
       or exists (select 1 from dojo.units_listing_member('github', 'ztest-oid-2')) then
        raise exception 'member set: the lookup named a dōjō for an unlisted person, or for another forge';
    end if;
end $$;
reset role;

-- A STALE set joins nobody (#308 R5c review): past dojo.member_set_max_age_hours()
-- (48 h, eight scheduled reads) since its complete read — a dead grant or a stopped
-- schedule — a sign-in provisions nothing from it.
do $$
begin
    if dojo.member_set_max_age_hours() <> 48 then
        raise exception 'member set: the maximum age is not 48 hours';
    end if;
    if (select member_set_read_at from dojo.tenant_connections where id = 'c0000000-0000-4000-8000-000000000c5a') <> now() then
        raise exception 'member set: the complete read did not stamp the connection''s member_set_read_at';
    end if;
end $$;
update dojo.tenant_connections set member_set_read_at = now() - interval '47 hours'
 where id = 'c0000000-0000-4000-8000-000000000c5a';
set local role service_role;
do $$
begin
    if not exists (select 1 from dojo.units_listing_member('azure_devops', 'ztest-oid-2')) then
        raise exception 'member set: a set inside the maximum age joined nobody';
    end if;
end $$;
reset role;
update dojo.tenant_connections set member_set_read_at = now() - interval '49 hours'
 where id = 'c0000000-0000-4000-8000-000000000c5a';
set local role service_role;
do $$
begin
    if exists (select 1 from dojo.units_listing_member('azure_devops', 'ztest-oid-2')) then
        raise exception 'member set: a stale set still names a dōjō to join';
    end if;
end $$;
reset role;
update dojo.tenant_connections set member_set_read_at = now() where id = 'c0000000-0000-4000-8000-000000000c5a';

-- A later complete read replaces the set: the person it no longer lists is no
-- longer found, and their membership ends (#309).
set local role service_role;
do $$
declare
    r record;
begin
    select * into r from dojo.reconcile_unit_members('b0000000-0000-4000-8000-000000000c5a', 'azure_devops', 'ztest-cm-org', 'azure_devops',
                                                     array['ztest-oid-1', 'ztest-oid-4']);
    if r.ended <> 2 or r.joined <> 0 then
        -- the signed-in member and the other dōjō's member, both this unit's
        raise exception 'member set: expected 2 ended and 0 joined on the later read, got %', r;
    end if;
    if exists (select 1 from dojo.units_listing_member('azure_devops', 'ztest-oid-2')) then
        raise exception 'member set: a later read did not replace the set';
    end if;
end $$;
reset role;

-- A set on a personal dōjō's connection is never a place to provision into.
insert into dojo.tenant_connections (id, tenant_id, provider, external_id, external_slug, connected_by, member_set_read_at) values
  ('c0000000-0000-4000-8000-000000000c5d', 'b0000000-0000-4000-8000-000000000c5c', 'azure_devops', 'ztest-cm-me', 'ztest-cm-me', 'a0000000-0000-4000-8000-000000000c51', now());
insert into dojo.connection_members (connection_id, subject, read_at) values
  ('c0000000-0000-4000-8000-000000000c5d', 'ztest-oid-3', now());
set local role service_role;
do $$
begin
    if exists (select 1 from dojo.units_listing_member('azure_devops', 'ztest-oid-3')) then
        raise exception 'member set: the lookup named a personal dōjō';
    end if;
end $$;
reset role;

-- The set belongs to its connection.
delete from dojo.tenant_connections where id = 'c0000000-0000-4000-8000-000000000c5a';
do $$
begin
    if exists (select 1 from dojo.connection_members where connection_id = 'c0000000-0000-4000-8000-000000000c5a') then
        raise exception 'member set: the set outlived its connection';
    end if;
end $$;

rollback;
