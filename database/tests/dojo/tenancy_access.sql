-- Memberships and identities as the signed-in person (#236 step 3;
-- docs/spec/dojo/access-matrix.md "Tenancy, identity, billing").
--
-- | data                          | member | admin                                  |
-- |-------------------------------|--------|----------------------------------------|
-- | own membership                | R own  |                                        |
-- | memberships of the tenant     | —      | R tenant; W the role                   |
-- | identities                    | R own  | R the tenant's ACTIVE members'; never W |
--
-- A membership's device fields (device_key, device_token_hash) are the daemon
-- plane's credential lookup and are read by nobody's session. Joining and
-- leaving are the system's (invites, provisioning): a person never inserts or
-- deletes a membership.
--
-- WHAT BREAKS THIS TEST: a membership read wider than own + tenant admin; a
-- SELECT grant on the device columns; an UPDATE grant beyond role; any INSERT or
-- DELETE grant; an identity read that reaches another tenant's members or a
-- disabled member; any identity write grant.
--
-- NOT separable here, on purpose: the identity policy's admin clause. Its
-- subquery reads memberships under the caller's own RLS, which already limits a
-- non-admin to their own rows, so dropping the clause changes nothing today. It
-- stays as defence in depth: it becomes load-bearing the day the memberships
-- read widens.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-0000000000b1', 'a1000000-0000-4000-8000-0000000000b1', 'ada'),
  ('a0000000-0000-4000-8000-0000000000b2', 'a1000000-0000-4000-8000-0000000000b2', 'lea'),
  ('a0000000-0000-4000-8000-0000000000b3', 'a1000000-0000-4000-8000-0000000000b3', 'ola'),
  ('a0000000-0000-4000-8000-0000000000b4', 'a1000000-0000-4000-8000-0000000000b4', 'dis'),
  ('a0000000-0000-4000-8000-0000000000b5', 'a1000000-0000-4000-8000-0000000000b5', 'uma');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-0000000000ba', 'organization/ztest-ten-t', 'organization',
   'ztest-ten-t', 'tenancy fixture T', 'dojo.sensei-hq.org/organization/ztest-ten-t'),
  ('b0000000-0000-4000-8000-0000000000bb', 'organization/ztest-ten-u', 'organization',
   'ztest-ten-u', 'tenancy fixture U', 'dojo.sensei-hq.org/organization/ztest-ten-u');

-- Membership ids end in: 1 ada@T · 2 lea@T · 3 ola@T · 4 dis@T (disabled) · 5 uma@U · 6 ola@U
insert into dojo.memberships (id, tenant_id, user_id, role, kind, authenticated_via, disabled_at, device_token_hash) values
  ('db000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-0000000000ba', 'a0000000-0000-4000-8000-0000000000b1', 'admin',       'employer', 'github_oauth', null,  'h1'),
  ('db000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-0000000000ba', 'a0000000-0000-4000-8000-0000000000b2', 'lead',        'employer', 'github_oauth', null,  'h2'),
  ('db000000-0000-4000-8000-000000000003', 'b0000000-0000-4000-8000-0000000000ba', 'a0000000-0000-4000-8000-0000000000b3', 'contributor', 'employer', 'github_oauth', null,  'h3'),
  ('db000000-0000-4000-8000-000000000004', 'b0000000-0000-4000-8000-0000000000ba', 'a0000000-0000-4000-8000-0000000000b4', 'contributor', 'employer', 'github_oauth', now(), 'h4'),
  ('db000000-0000-4000-8000-000000000005', 'b0000000-0000-4000-8000-0000000000bb', 'a0000000-0000-4000-8000-0000000000b5', 'admin',       'employer', 'github_oauth', null,  'h5'),
  ('db000000-0000-4000-8000-000000000006', 'b0000000-0000-4000-8000-0000000000bb', 'a0000000-0000-4000-8000-0000000000b3', 'contributor', 'employer', 'github_oauth', null,  'h6');

-- Identity subjects end in the principal's letter: a ada · o ola · d dis · u uma
insert into dojo.identities (principal_id, provider, subject, display_name) values
  ('a0000000-0000-4000-8000-0000000000b1', 'github_oauth', 'ztest-id-a', 'ada'),
  ('a0000000-0000-4000-8000-0000000000b3', 'github_oauth', 'ztest-id-o', 'ola'),
  ('a0000000-0000-4000-8000-0000000000b4', 'github_oauth', 'ztest-id-d', 'dis'),
  ('a0000000-0000-4000-8000-0000000000b5', 'github_oauth', 'ztest-id-u', 'uma');

set local role authenticated;

create temp table expect (who text, sub text, memberships text, identities text) on commit drop;
insert into expect values
  ('ada', 'a1000000-0000-4000-8000-0000000000b1', '1,2,3,4', 'a,o'),   -- admin of T: all of T; active members' identities
  ('lea', 'a1000000-0000-4000-8000-0000000000b2', '2',       ''),      -- lead: own only (no identity row)
  ('ola', 'a1000000-0000-4000-8000-0000000000b3', '3,6',     'o'),     -- own memberships in every dōjō; own identity
  ('uma', 'a1000000-0000-4000-8000-0000000000b5', '5,6',     'o,u');   -- admin of U

do $$
declare
    r record;
    got text;
begin
    for r in select * from expect loop
        perform set_config('request.jwt.claims', json_build_object('sub', r.sub)::text, true);
        select coalesce(string_agg(right(id::text, 1), ',' order by id), '') into got
          from dojo.memberships where id::text like 'db000000-%';
        if got is distinct from r.memberships then
            raise exception 'memberships RLS: % should see [%], saw [%]', r.who, r.memberships, got;
        end if;
        select coalesce(string_agg(right(subject, 1), ',' order by subject), '') into got
          from dojo.identities where subject like 'ztest-id-%';
        if got is distinct from r.identities then
            raise exception 'identities RLS: % should see [%], saw [%]', r.who, r.identities, got;
        end if;
    end loop;
end $$;

do $$
declare
    n int;
    h text;
    t constant uuid := 'b0000000-0000-4000-8000-0000000000ba';
    ola constant uuid := 'a0000000-0000-4000-8000-0000000000b3';
begin
    -- ── the device columns are nobody's to read ───────────────────────────────
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-0000000000b1"}', true);
    begin
        select device_token_hash into h from dojo.memberships where id = 'db000000-0000-4000-8000-000000000003';
        raise exception 'memberships: an admin read a member''s device_token_hash';
    exception when insufficient_privilege then null;
    end;
    begin
        select device_key into h from dojo.memberships where id = 'db000000-0000-4000-8000-000000000001';
        raise exception 'memberships: a person read their own device_key from their session';
    exception when insufficient_privilege then null;
    end;

    -- ── the admin sets a role in their tenant, and only a role ────────────────
    update dojo.memberships set role = 'maintainer', updated_at = now() where tenant_id = t and user_id = ola;
    get diagnostics n = row_count;
    if n <> 1 then raise exception 'memberships: the admin should set ola''s role in T, changed %', n; end if;
    update dojo.memberships set role = 'admin' where id = 'db000000-0000-4000-8000-000000000006';
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'memberships: an admin of T set a role in U'; end if;
    begin
        update dojo.memberships set disabled_at = null where id = 'db000000-0000-4000-8000-000000000004';
        raise exception 'memberships: only the role is the admin''s to change';
    exception when insufficient_privilege then
        if sqlerrm like '%row-level security%' then raise exception 'memberships: disabled_at must be refused by the column grant'; end if;
    end;
    begin
        insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via)
        values (t, 'a0000000-0000-4000-8000-0000000000b5', 'contributor', 'employer', 'github_oauth');
        raise exception 'memberships: an admin added a person by id (joining is by invitation)';
    exception when insufficient_privilege then
        if sqlerrm like '%row-level security%' then raise exception 'memberships: insert must be refused by the grant'; end if;
    end;
    begin
        delete from dojo.memberships where id = 'db000000-0000-4000-8000-000000000003';
        raise exception 'memberships: an admin deleted a membership';
    exception when insufficient_privilege then
        if sqlerrm like '%row-level security%' then raise exception 'memberships: delete must be refused by the grant'; end if;
    end;
    begin
        update dojo.identities set display_name = 'renamed' where subject = 'ztest-id-o';
        raise exception 'identities: an admin wrote a member''s identity';
    exception when insufficient_privilege then
        if sqlerrm like '%row-level security%' then raise exception 'identities: writes must be refused by the grant'; end if;
    end;

    -- ── a lead is not an admin ────────────────────────────────────────────────
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-0000000000b2"}', true);
    update dojo.memberships set role = 'admin' where id = 'db000000-0000-4000-8000-000000000002';
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'memberships: a lead promoted themselves'; end if;
end $$;

rollback;
