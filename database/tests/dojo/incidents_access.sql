-- Who reads and changes incidents (#236 step 3; docs/spec/dojo/access-matrix.md
-- "Client work and compliance": incidents R/W tenant for a tenant-role lead and
-- an admin; nobody below).
--
-- An incident's references must name objects of its own tenant: its engagement,
-- its artifact, and its owner — an ACTIVE member of the tenant. The owner check
-- is dojo.principal_is_member, which answers only for a tenant the caller is a
-- member of (it is not a way to ask who belongs to someone else's dōjō).
--
-- WHAT BREAKS THIS TEST: a policy below lead or without the tenant; a with-check
-- that drops any one reference check; an owner check that ignores disabled_at;
-- a principal_is_member that answers for a tenant the caller is not in; an
-- UPDATE grant on tenant_id.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-0000000000a1', 'a1000000-0000-4000-8000-0000000000a1', 'lea'),
  ('a0000000-0000-4000-8000-0000000000a2', 'a1000000-0000-4000-8000-0000000000a2', 'mia'),
  ('a0000000-0000-4000-8000-0000000000a3', 'a1000000-0000-4000-8000-0000000000a3', 'ola'),
  ('a0000000-0000-4000-8000-0000000000a4', 'a1000000-0000-4000-8000-0000000000a4', 'dis'),
  ('a0000000-0000-4000-8000-0000000000a5', 'a1000000-0000-4000-8000-0000000000a5', 'uma');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-0000000000aa', 'organization/ztest-inc-t', 'organization',
   'ztest-inc-t', 'incidents fixture T', 'dojo.sensei-hq.org/organization/ztest-inc-t'),
  ('b0000000-0000-4000-8000-0000000000ab', 'organization/ztest-inc-u', 'organization',
   'ztest-inc-u', 'incidents fixture U', 'dojo.sensei-hq.org/organization/ztest-inc-u');

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via, disabled_at) values
  ('b0000000-0000-4000-8000-0000000000aa', 'a0000000-0000-4000-8000-0000000000a1', 'lead',        'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-0000000000aa', 'a0000000-0000-4000-8000-0000000000a2', 'maintainer',  'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-0000000000aa', 'a0000000-0000-4000-8000-0000000000a3', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-0000000000aa', 'a0000000-0000-4000-8000-0000000000a4', 'contributor', 'employer', 'github_oauth', now()),
  ('b0000000-0000-4000-8000-0000000000ab', 'a0000000-0000-4000-8000-0000000000a5', 'admin',       'employer', 'github_oauth', null);

insert into dojo.engagements (id, tenant_id, client_name) values
  ('ca000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-0000000000aa', 'ztest T client'),
  ('ca000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-0000000000ab', 'ztest U client');
insert into dojo.artifacts (id, tenant_id, kind, status, title, body, signature) values
  ('fa000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-0000000000aa', 'principle', 'published', 'T', 'x', 'ztest-inc-1'),
  ('fa000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-0000000000ab', 'principle', 'published', 'U', 'x', 'ztest-inc-2');
insert into dojo.incidents (id, tenant_id, title) values
  ('ea000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-0000000000aa', 'ztest T incident'),
  ('ea000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-0000000000ab', 'ztest U incident');

set local role authenticated;

do $$
declare
    n int;
    got text;
    t   constant uuid := 'b0000000-0000-4000-8000-0000000000aa';
    u   constant uuid := 'b0000000-0000-4000-8000-0000000000ab';
    ola constant uuid := 'a0000000-0000-4000-8000-0000000000a3';
    dis constant uuid := 'a0000000-0000-4000-8000-0000000000a4';
    uma constant uuid := 'a0000000-0000-4000-8000-0000000000a5';
    et  constant uuid := 'ca000000-0000-4000-8000-000000000001';
    eu  constant uuid := 'ca000000-0000-4000-8000-000000000002';
    at_ constant uuid := 'fa000000-0000-4000-8000-000000000001';
    au  constant uuid := 'fa000000-0000-4000-8000-000000000002';
    i1  constant uuid := 'ea000000-0000-4000-8000-000000000001';
    i2  constant uuid := 'ea000000-0000-4000-8000-000000000002';
begin
    -- ── the owner predicate is not an oracle ──────────────────────────────────
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-0000000000a5"}', true);
    if dojo.principal_is_member(t, ola) then
        raise exception 'principal_is_member: answered for a tenant the caller is not in';
    end if;

    -- ── mia (maintainer): nothing ────────────────────────────────────────────
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-0000000000a2"}', true);
    select count(*) into n from dojo.incidents where title like 'ztest %';
    if n <> 0 then raise exception 'incidents: a maintainer saw % incident(s)', n; end if;
    begin
        insert into dojo.incidents (tenant_id, title) values (t, 'ztest by mia');
        raise exception 'incidents: a maintainer opened an incident';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'maintainer insert refused for the wrong reason: %', sqlerrm; end if;
    end;

    -- ── lea (tenant lead) ────────────────────────────────────────────────────
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-0000000000a1"}', true);
    if not dojo.principal_is_member(t, ola) then raise exception 'principal_is_member: ola is an active member of T'; end if;
    if dojo.principal_is_member(t, dis) then raise exception 'principal_is_member: a disabled membership is no membership'; end if;
    if dojo.principal_is_member(t, uma) then raise exception 'principal_is_member: uma is not a member of T'; end if;

    select coalesce(string_agg(title, ',' order by title), '') into got from dojo.incidents where title like 'ztest %';
    if got <> 'ztest T incident' then raise exception 'incidents: lea should see T''s only, saw [%]', got; end if;

    insert into dojo.incidents (tenant_id, title, description, severity, engagement_id, artifact_id, owner_id, sla_due_at)
    values (t, 'ztest by lea', 'x', 'high', et, at_, ola, now());

    begin
        insert into dojo.incidents (tenant_id, title, owner_id) values (t, 'ztest foreign owner', uma);
        raise exception 'incidents: an owner outside the tenant was accepted';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'foreign owner refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        insert into dojo.incidents (tenant_id, title, owner_id) values (t, 'ztest disabled owner', dis);
        raise exception 'incidents: a disabled member was accepted as owner';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'disabled owner refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        insert into dojo.incidents (tenant_id, title, engagement_id) values (t, 'ztest foreign engagement', eu);
        raise exception 'incidents: another tenant''s engagement was accepted';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'foreign engagement refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        insert into dojo.incidents (tenant_id, title, artifact_id) values (t, 'ztest foreign artifact', au);
        raise exception 'incidents: another tenant''s artifact was accepted';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'foreign artifact refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        insert into dojo.incidents (tenant_id, title) values (u, 'ztest into U');
        raise exception 'incidents: lea opened an incident in another tenant';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'cross-tenant insert refused for the wrong reason: %', sqlerrm; end if;
    end;

    update dojo.incidents set status = 'resolved', severity = 'low', owner_id = ola, sla_due_at = now(),
                              resolution = 'fixed', resolved_at = now(), updated_at = now()
     where id = i1;
    get diagnostics n = row_count;
    if n <> 1 then raise exception 'incidents: lea should change T''s incident, changed %', n; end if;
    begin
        update dojo.incidents set owner_id = uma where id = i1;
        raise exception 'incidents: re-assigned to an owner outside the tenant';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'foreign re-assignment refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        update dojo.incidents set tenant_id = u where id = i1;
        raise exception 'incidents: an incident changed tenant';
    exception when insufficient_privilege then
        if sqlerrm like '%row-level security%' then raise exception 'incidents: tenant_id must be refused by the column grant'; end if;
    end;

    update dojo.incidents set status = 'resolved' where id = i2;
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'incidents: lea changed U''s incident'; end if;
    delete from dojo.incidents where id = i2;
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'incidents: lea deleted U''s incident'; end if;
    delete from dojo.incidents where id = i1;
    get diagnostics n = row_count;
    if n <> 1 then raise exception 'incidents: lea should delete T''s incident, deleted %', n; end if;

    -- mia, again: T's incidents are not hers to change.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-0000000000a2"}', true);
    update dojo.incidents set status = 'open' where title = 'ztest by lea';
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'incidents: a maintainer changed an incident'; end if;
end $$;

rollback;
