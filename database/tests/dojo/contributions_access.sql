-- A person's contributions plane (#236 step 3; docs/spec/dojo/access-matrix.md
-- entity model §3: personal work is the person's, across every dōjō).
--
-- | data                                   | who                                    |
-- |----------------------------------------|----------------------------------------|
-- | downstream inbox (what reached them)   | its own person: read, and mark adopted |
-- | a dōjō's identity (name, key, url …)   | its members                            |
--
-- A tenant's settings and claim are not part of its identity, and no session
-- reads them. A downstream row's membership and artifact never change.
--
-- WHAT BREAKS THIS TEST: a downstream policy wider than own (an admin included);
-- an UPDATE grant beyond state/acted_at/updated_at; a tenant read without
-- is_member; a SELECT grant on tenants.settings.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-0000000000f1', 'a1000000-0000-4000-8000-0000000000f1', 'ola'),
  ('a0000000-0000-4000-8000-0000000000f2', 'a1000000-0000-4000-8000-0000000000f2', 'ben'),
  ('a0000000-0000-4000-8000-0000000000f3', 'a1000000-0000-4000-8000-0000000000f3', 'ada');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url, settings) values
  ('b0000000-0000-4000-8000-0000000000fa', 'organization/ztest-con-t', 'organization',
   'ztest-con-t', 'ztest contributions T', 'dojo.sensei-hq.org/organization/ztest-con-t', '{"secret":"x"}'),
  ('b0000000-0000-4000-8000-0000000000fb', 'organization/ztest-con-u', 'organization',
   'ztest-con-u', 'ztest contributions U', 'dojo.sensei-hq.org/organization/ztest-con-u', '{}');

insert into dojo.memberships (id, tenant_id, user_id, role, kind, authenticated_via) values
  ('df000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-0000000000fa', 'a0000000-0000-4000-8000-0000000000f1', 'contributor', 'employer', 'github_oauth'),
  ('df000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-0000000000fa', 'a0000000-0000-4000-8000-0000000000f2', 'contributor', 'employer', 'github_oauth'),
  ('df000000-0000-4000-8000-000000000003', 'b0000000-0000-4000-8000-0000000000fa', 'a0000000-0000-4000-8000-0000000000f3', 'admin',       'employer', 'github_oauth');

insert into dojo.artifacts (id, tenant_id, kind, status, title, body, signature) values
  ('ff000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-0000000000fa', 'principle', 'published', 'one', 'x', 'ztest-con-1');

-- 1 ola's · 2 ben's
insert into dojo.downstream_inbox (id, tenant_id, membership_id, artifact_id) values
  ('ee000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-0000000000fa',
   'df000000-0000-4000-8000-000000000001', 'ff000000-0000-4000-8000-000000000001'),
  ('ee000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-0000000000fa',
   'df000000-0000-4000-8000-000000000002', 'ff000000-0000-4000-8000-000000000001');

set local role authenticated;

do $$
declare
    n int;
    got text;
begin
    -- ── ola: her own downstream, her own dōjōs ───────────────────────────────
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-0000000000f1"}', true);
    select coalesce(string_agg(right(id::text, 1), ',' order by id), '') into got
      from dojo.downstream_inbox where id::text like 'ee000000-%';
    if got <> '1' then raise exception 'downstream: ola should see her own only, saw [%]', got; end if;

    update dojo.downstream_inbox set state = 'pinned', acted_at = now(), updated_at = now()
     where id::text like 'ee000000-%';
    get diagnostics n = row_count;
    if n <> 1 then raise exception 'downstream: ola should adopt her own row only, changed %', n; end if;
    begin
        update dojo.downstream_inbox set membership_id = 'df000000-0000-4000-8000-000000000002'
         where id = 'ee000000-0000-4000-8000-000000000001';
        raise exception 'downstream: a row changed hands';
    exception when insufficient_privilege then
        if sqlerrm like '%row-level security%' then raise exception 'downstream: membership_id must be refused by the column grant'; end if;
    end;

    select coalesce(string_agg(name, ',' order by name), '') into got from dojo.tenants where key like 'organization/ztest-con-%';
    if got <> 'ztest contributions T' then raise exception 'tenants: ola should see her own dōjō only, saw [%]', got; end if;
    begin
        select settings::text into got from dojo.tenants where key = 'organization/ztest-con-t';
        raise exception 'tenants: a member read the tenant''s settings';
    exception when insufficient_privilege then null;
    end;

    -- ── ada (admin of T): another person's downstream is theirs, not the admin's
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-0000000000f3"}', true);
    select count(*) into n from dojo.downstream_inbox where id::text like 'ee000000-%';
    if n <> 0 then raise exception 'downstream: an admin read members'' downstream rows (%)', n; end if;
end $$;

rollback;
