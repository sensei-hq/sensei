-- Who reads and changes client engagements (#236 step 3; docs/spec/dojo/access-matrix.md
-- "Client work and compliance": engagements R/W tenant for a tenant-role lead and
-- an admin; nobody below).
--
-- Engagements name clients, so a tenant's maintainers and contributors do not
-- see them, and nobody sees another tenant's. An engagement never changes
-- tenant, and the client it names is set when it is opened.
--
-- WHAT BREAKS THIS TEST: a policy below lead; a policy without the tenant; an
-- UPDATE grant on tenant_id or client_name; a delete policy
-- wider than the tenant.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000091', 'a1000000-0000-4000-8000-000000000091', 'lea'),
  ('a0000000-0000-4000-8000-000000000092', 'a1000000-0000-4000-8000-000000000092', 'mia'),
  ('a0000000-0000-4000-8000-000000000093', 'a1000000-0000-4000-8000-000000000093', 'uma');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000009a', 'organization/ztest-eng-t', 'organization',
   'ztest-eng-t', 'engagements fixture T', 'dojo.sensei-hq.org/organization/ztest-eng-t'),
  ('b0000000-0000-4000-8000-00000000009b', 'organization/ztest-eng-u', 'organization',
   'ztest-eng-u', 'engagements fixture U', 'dojo.sensei-hq.org/organization/ztest-eng-u');

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000009a', 'a0000000-0000-4000-8000-000000000091', 'lead',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000009a', 'a0000000-0000-4000-8000-000000000092', 'maintainer', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000009b', 'a0000000-0000-4000-8000-000000000093', 'admin',      'employer', 'github_oauth');

insert into dojo.engagements (id, tenant_id, client_name) values
  ('c9000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-00000000009a', 'ztest client one'),
  ('c9000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-00000000009b', 'ztest client two');

set local role authenticated;

do $$
declare
    n int;
    got text;
    t constant uuid := 'b0000000-0000-4000-8000-00000000009a';
    u constant uuid := 'b0000000-0000-4000-8000-00000000009b';
    e1 constant uuid := 'c9000000-0000-4000-8000-000000000001';
    e2 constant uuid := 'c9000000-0000-4000-8000-000000000002';
begin
    -- mia (maintainer) sees no engagement and changes none.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000092"}', true);
    select count(*) into n from dojo.engagements where client_name like 'ztest %';
    if n <> 0 then raise exception 'engagements: a maintainer saw % engagement(s)', n; end if;
    begin
        insert into dojo.engagements (tenant_id, client_name) values (t, 'ztest by mia');
        raise exception 'engagements: a maintainer opened an engagement';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'maintainer insert refused for the wrong reason: %', sqlerrm; end if;
    end;

    -- lea (tenant lead) reads, opens, changes and closes T's engagements only.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000091"}', true);
    select coalesce(string_agg(client_name, ',' order by client_name), '') into got
      from dojo.engagements where client_name like 'ztest %';
    if got <> 'ztest client one' then raise exception 'engagements: lea should see T''s only, saw [%]', got; end if;

    insert into dojo.engagements (tenant_id, client_tenant_id, client_name, description, project_bindings, policy_overrides)
    values (t, null, 'ztest by lea', 'x', '[]', '{}');
    begin
        insert into dojo.engagements (tenant_id, client_name) values (u, 'ztest into U');
        raise exception 'engagements: lea opened an engagement in another tenant';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'cross-tenant insert refused for the wrong reason: %', sqlerrm; end if;
    end;

    update dojo.engagements set status = 'ended', description = 'y', starts_on = current_date, ends_on = current_date,
                                policy_overrides = '{}', project_bindings = '[]', updated_at = now()
     where id = e1;
    get diagnostics n = row_count;
    if n <> 1 then raise exception 'engagements: lea should change T''s engagement, changed %', n; end if;

    begin
        update dojo.engagements set tenant_id = u where id = e1;
        raise exception 'engagements: an engagement changed tenant';
    exception when insufficient_privilege then
        if sqlerrm like '%row-level security%' then raise exception 'engagements: tenant_id must be refused by the column grant'; end if;
    end;
    begin
        update dojo.engagements set client_name = 'renamed' where id = e1;
        raise exception 'engagements: the client an engagement names is set when it opens';
    exception when insufficient_privilege then
        if sqlerrm like '%row-level security%' then raise exception 'engagements: client_name must be refused by the column grant'; end if;
    end;

    delete from dojo.engagements where id = e2;
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'engagements: lea deleted U''s engagement'; end if;
    delete from dojo.engagements where client_name = 'ztest by lea';
    get diagnostics n = row_count;
    if n <> 1 then raise exception 'engagements: lea should close T''s engagement, deleted %', n; end if;

    -- mia, again: a change to T's engagement she cannot see is no change.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000092"}', true);
    update dojo.engagements set status = 'ended' where id = e1;
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'engagements: a maintainer changed an engagement'; end if;
end $$;

rollback;
