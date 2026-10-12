-- Who reads seats and the billing account (#236 step 3; docs/spec/dojo/access-matrix.md
-- "Tenancy, identity, billing": an admin reads the tenant's).
--
-- Only a tenant admin reads its seats and billing account. Nobody writes either
-- from a session: seats open and close with relay activity, the account follows
-- the payment provider, and seats_used is a cache derived from the seats — a
-- money-facing number no person sets by hand.
--
-- WHAT BREAKS THIS TEST: a read policy below admin or without the tenant; any
-- write grant on seats or billing_accounts.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-0000000000e1', 'a1000000-0000-4000-8000-0000000000e1', 'ada'),
  ('a0000000-0000-4000-8000-0000000000e2', 'a1000000-0000-4000-8000-0000000000e2', 'lea'),
  ('a0000000-0000-4000-8000-0000000000e3', 'a1000000-0000-4000-8000-0000000000e3', 'uma');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-0000000000ea', 'organization/ztest-bill-t', 'organization',
   'ztest-bill-t', 'billing fixture T', 'dojo.sensei-hq.org/organization/ztest-bill-t'),
  ('b0000000-0000-4000-8000-0000000000eb', 'organization/ztest-bill-u', 'organization',
   'ztest-bill-u', 'billing fixture U', 'dojo.sensei-hq.org/organization/ztest-bill-u');

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-0000000000ea', 'a0000000-0000-4000-8000-0000000000e1', 'admin', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-0000000000ea', 'a0000000-0000-4000-8000-0000000000e2', 'lead',  'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-0000000000eb', 'a0000000-0000-4000-8000-0000000000e3', 'admin', 'employer', 'github_oauth');

insert into sensei.namespaces (id, scope_key, name, slug) values
  ('e0000000-0000-4000-8000-0000000000e1', 'organization', 'ztest billing ns', 'ztest-billing-ns');

-- Seat user ids end in: 1, 2 (T) · 3 (U)
insert into dojo.seats (tenant_id, user_id, namespace_id) values
  ('b0000000-0000-4000-8000-0000000000ea', 'f0000000-0000-4000-8000-0000000000e1', 'e0000000-0000-4000-8000-0000000000e1'),
  ('b0000000-0000-4000-8000-0000000000ea', 'f0000000-0000-4000-8000-0000000000e2', 'e0000000-0000-4000-8000-0000000000e1'),
  ('b0000000-0000-4000-8000-0000000000eb', 'f0000000-0000-4000-8000-0000000000e3', 'e0000000-0000-4000-8000-0000000000e1');
insert into dojo.billing_accounts (tenant_id, plan) values
  ('b0000000-0000-4000-8000-0000000000ea', 'ztest-t'),
  ('b0000000-0000-4000-8000-0000000000eb', 'ztest-u');

set local role authenticated;

create temp table expect (who text, sub text, seats text, plans text) on commit drop;
insert into expect values
  ('ada', 'a1000000-0000-4000-8000-0000000000e1', '1,2', 'ztest-t'),
  ('lea', 'a1000000-0000-4000-8000-0000000000e2', '',    ''),
  ('uma', 'a1000000-0000-4000-8000-0000000000e3', '3',   'ztest-u');

do $$
declare
    r record;
    got text;
begin
    for r in select * from expect loop
        perform set_config('request.jwt.claims', json_build_object('sub', r.sub)::text, true);
        select coalesce(string_agg(right(user_id::text, 1), ',' order by user_id), '') into got
          from dojo.seats where namespace_id = 'e0000000-0000-4000-8000-0000000000e1';
        if got is distinct from r.seats then
            raise exception 'seats RLS: % should see [%], saw [%]', r.who, r.seats, got;
        end if;
        select coalesce(string_agg(plan, ',' order by plan), '') into got
          from dojo.billing_accounts where plan like 'ztest-%';
        if got is distinct from r.plans then
            raise exception 'billing_accounts RLS: % should see [%], saw [%]', r.who, r.plans, got;
        end if;
    end loop;

    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-0000000000e1"}', true);
    begin
        update dojo.billing_accounts set seats_used = 0 where plan = 'ztest-t';
        raise exception 'billing_accounts: an admin set seats_used by hand';
    exception when insufficient_privilege then
        if sqlerrm like '%row-level security%' then raise exception 'billing_accounts: writes must be refused by the grant'; end if;
    end;
    begin
        update dojo.seats set ended_at = now() where user_id = 'f0000000-0000-4000-8000-0000000000e1';
        raise exception 'seats: an admin closed a seat from their session';
    exception when insufficient_privilege then
        if sqlerrm like '%row-level security%' then raise exception 'seats: writes must be refused by the grant'; end if;
    end;
end $$;

rollback;
