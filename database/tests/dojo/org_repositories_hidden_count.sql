-- billing_hidden marks exactly the org-private rows the entitlement CASE
-- refuses for billing reasons, and the hidden-private count is the count of
-- those marks (org-repositories r2, test 6; D88: past due hides at once).
--
-- WHAT BREAKS THIS TEST: a missing dojo.org_repositories(); a hide set that
-- drifts from isBillingHiddenRepo (unclaimed / not_subscribed /
-- subscription_expired on org-private); capture refusals or public repos
-- counted as billing-hidden; a billing term misordered (unclaimed must beat
-- not_subscribed; missing-row and period-less are their own triggers).
--
-- Billing vocabulary note: the mockup's "lapsed" copy state is the
-- `canceled` status (billing_status has no `lapsed` label); period expiry
-- is pinned separately as subscription_expired.
--
-- Phase A: red — dojo.org_repositories(uuid) does not exist yet.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000161', 'a1000000-0000-4000-8000-000000000161', 'ada'),
  ('a0000000-0000-4000-8000-000000000162', 'a1000000-0000-4000-8000-000000000162', 'ola');

-- Eight org tenants, one per billing state. One admin principal holds all
-- eight admin memberships; one member principal holds the standing roles.
insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-000000000161', 'organization/ztest-orrepos-bact', 'organization',
   'ztest-orrepos-bact', 'bact', 'dojo.sensei-hq.org/organization/ztest-orrepos-bact', now()),
  ('b0000000-0000-4000-8000-000000000162', 'organization/ztest-orrepos-btri', 'organization',
   'ztest-orrepos-btri', 'btri', 'dojo.sensei-hq.org/organization/ztest-orrepos-btri', now()),
  ('b0000000-0000-4000-8000-000000000163', 'organization/ztest-orrepos-bdue', 'organization',
   'ztest-orrepos-bdue', 'bdue', 'dojo.sensei-hq.org/organization/ztest-orrepos-bdue', now()),
  ('b0000000-0000-4000-8000-000000000164', 'organization/ztest-orrepos-bcan', 'organization',
   'ztest-orrepos-bcan', 'bcan', 'dojo.sensei-hq.org/organization/ztest-orrepos-bcan', now()),
  ('b0000000-0000-4000-8000-000000000165', 'organization/ztest-orrepos-bunc', 'organization',
   'ztest-orrepos-bunc', 'bunc', 'dojo.sensei-hq.org/organization/ztest-orrepos-bunc', null),
  ('b0000000-0000-4000-8000-000000000166', 'organization/ztest-orrepos-bnor', 'organization',
   'ztest-orrepos-bnor', 'bnor', 'dojo.sensei-hq.org/organization/ztest-orrepos-bnor', now()),
  ('b0000000-0000-4000-8000-000000000167', 'organization/ztest-orrepos-bper', 'organization',
   'ztest-orrepos-bper', 'bper', 'dojo.sensei-hq.org/organization/ztest-orrepos-bper', now()),
  ('b0000000-0000-4000-8000-000000000168', 'organization/ztest-orrepos-bexp', 'organization',
   'ztest-orrepos-bexp', 'bexp', 'dojo.sensei-hq.org/organization/ztest-orrepos-bexp', now());

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via)
select t, 'a0000000-0000-4000-8000-000000000161', 'admin', 'employer', 'github_oauth'
  from (values ('b0000000-0000-4000-8000-000000000161'::uuid),
               ('b0000000-0000-4000-8000-000000000162'::uuid),
               ('b0000000-0000-4000-8000-000000000163'::uuid),
               ('b0000000-0000-4000-8000-000000000164'::uuid),
               ('b0000000-0000-4000-8000-000000000165'::uuid),
               ('b0000000-0000-4000-8000-000000000166'::uuid),
               ('b0000000-0000-4000-8000-000000000167'::uuid),
               ('b0000000-0000-4000-8000-000000000168')) v(t);

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via)
select t, 'a0000000-0000-4000-8000-000000000162', 'contributor', 'employer', 'github_oauth'
  from (values ('b0000000-0000-4000-8000-000000000161'::uuid),
               ('b0000000-0000-4000-8000-000000000162'::uuid),
               ('b0000000-0000-4000-8000-000000000163'::uuid),
               ('b0000000-0000-4000-8000-000000000164'::uuid),
               ('b0000000-0000-4000-8000-000000000165'::uuid),
               ('b0000000-0000-4000-8000-000000000166'::uuid),
               ('b0000000-0000-4000-8000-000000000167'::uuid),
               ('b0000000-0000-4000-8000-000000000168')) v(t);

-- Active, trialing, past_due, canceled(lapsed copy), period-less,
-- expired. Unclaimed (bunc) holds a billing row on purpose: the unclaimed
-- term must beat billing, so billing-first would mislabel it.
-- bnor holds NO row: the missing-row trigger.
insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b0000000-0000-4000-8000-000000000161', 'pro', 'active',   current_date - 1, current_date + 30),
  ('b0000000-0000-4000-8000-000000000162', 'pro', 'trialing', current_date - 1, current_date + 30),
  ('b0000000-0000-4000-8000-000000000163', 'pro', 'past_due', current_date - 1, current_date + 30),
  ('b0000000-0000-4000-8000-000000000164', 'pro', 'canceled', current_date - 1, current_date + 30),
  ('b0000000-0000-4000-8000-000000000165', 'pro', 'active',   current_date - 1, current_date + 30),
  ('b0000000-0000-4000-8000-000000000167', 'pro', 'active',   null,             null),
  ('b0000000-0000-4000-8000-000000000168', 'pro', 'active',   current_date - 30, current_date - 1);

-- Two private repos per tenant; the past-due tenant also carries a public
-- repo and a capture-failed repo, neither of which may count as hidden.
insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider) values
  ('90000000-0000-4000-8000-000000000161', 'b0000000-0000-4000-8000-000000000161',
   'github.com/ztest-orrepos-bact/p1', 'p1', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000162', 'b0000000-0000-4000-8000-000000000161',
   'github.com/ztest-orrepos-bact/p2', 'p2', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000163', 'b0000000-0000-4000-8000-000000000162',
   'github.com/ztest-orrepos-btri/p1', 'p1', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000164', 'b0000000-0000-4000-8000-000000000162',
   'github.com/ztest-orrepos-btri/p2', 'p2', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000165', 'b0000000-0000-4000-8000-000000000163',
   'github.com/ztest-orrepos-bdue/p1', 'p1', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000166', 'b0000000-0000-4000-8000-000000000163',
   'github.com/ztest-orrepos-bdue/p2', 'p2', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000167', 'b0000000-0000-4000-8000-000000000163',
   'github.com/ztest-orrepos-bdue/pub', 'pub', 'public', now(), 'github'),
  ('90000000-0000-4000-8000-000000000168', 'b0000000-0000-4000-8000-000000000163',
   'github.com/ztest-orrepos-bdue/uncap', 'uncap', null, null, 'github'),
  ('90000000-0000-4000-8000-000000000169', 'b0000000-0000-4000-8000-000000000164',
   'github.com/ztest-orrepos-bcan/p1', 'p1', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000170', 'b0000000-0000-4000-8000-000000000164',
   'github.com/ztest-orrepos-bcan/p2', 'p2', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000171', 'b0000000-0000-4000-8000-000000000165',
   'github.com/ztest-orrepos-bunc/p1', 'p1', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000172', 'b0000000-0000-4000-8000-000000000165',
   'github.com/ztest-orrepos-bunc/p2', 'p2', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000173', 'b0000000-0000-4000-8000-000000000166',
   'github.com/ztest-orrepos-bnor/p1', 'p1', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000174', 'b0000000-0000-4000-8000-000000000166',
   'github.com/ztest-orrepos-bnor/p2', 'p2', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000175', 'b0000000-0000-4000-8000-000000000167',
   'github.com/ztest-orrepos-bper/p1', 'p1', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000176', 'b0000000-0000-4000-8000-000000000167',
   'github.com/ztest-orrepos-bper/p2', 'p2', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000177', 'b0000000-0000-4000-8000-000000000168',
   'github.com/ztest-orrepos-bexp/p1', 'p1', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000178', 'b0000000-0000-4000-8000-000000000168',
   'github.com/ztest-orrepos-bexp/p2', 'p2', 'private', now(), 'github');

insert into dojo.repository_roles (repository_id, principal_id, forge_role)
select r, 'a0000000-0000-4000-8000-000000000162', 'contributor'
  from (values ('90000000-0000-4000-8000-000000000161'::uuid),
               ('90000000-0000-4000-8000-000000000162'::uuid),
               ('90000000-0000-4000-8000-000000000163'::uuid),
               ('90000000-0000-4000-8000-000000000164'::uuid),
               ('90000000-0000-4000-8000-000000000165'::uuid),
               ('90000000-0000-4000-8000-000000000166'::uuid),
               ('90000000-0000-4000-8000-000000000167'::uuid),
               ('90000000-0000-4000-8000-000000000168'::uuid),
               ('90000000-0000-4000-8000-000000000169'::uuid),
               ('90000000-0000-4000-8000-000000000170'::uuid),
               ('90000000-0000-4000-8000-000000000171'::uuid),
               ('90000000-0000-4000-8000-000000000172'::uuid),
               ('90000000-0000-4000-8000-000000000173'::uuid),
               ('90000000-0000-4000-8000-000000000174'::uuid),
               ('90000000-0000-4000-8000-000000000175'::uuid),
               ('90000000-0000-4000-8000-000000000176'::uuid),
               ('90000000-0000-4000-8000-000000000177'::uuid),
               ('90000000-0000-4000-8000-000000000178')) v(r);

set local role authenticated;

do $$
declare
    t uuid[];
    want int[];
    i int;
    got int;
begin
    perform set_config('request.jwt.claims',
        json_build_object('sub', 'a1000000-0000-4000-8000-000000000161')::text, true);

    -- (tenant, hidden-private count): the whole billing matrix.
    t := array['b0000000-0000-4000-8000-000000000161',  -- active
               'b0000000-0000-4000-8000-000000000162',  -- trialing
               'b0000000-0000-4000-8000-000000000163',  -- past_due
               'b0000000-0000-4000-8000-000000000164',  -- canceled
               'b0000000-0000-4000-8000-000000000165',  -- unclaimed
               'b0000000-0000-4000-8000-000000000166',  -- missing row
               'b0000000-0000-4000-8000-000000000167',  -- period-less
               'b0000000-0000-4000-8000-000000000168']; -- expired
    want := array[0, 0, 2, 2, 2, 2, 2, 2];
    for i in 1 .. 8 loop
        select count(*) into got from dojo.org_repositories(t[i]) where billing_hidden;
        if got is distinct from want[i] then
            raise exception 'hidden count tenant %: expected %, got %', t[i], want[i], got;
        end if;
    end loop;

    -- The past-due extras: public and capture-failed repos are listed but
    -- never billing-hidden (capture refusals never hide, D88).
    select count(*) into got from dojo.org_repositories('b0000000-0000-4000-8000-000000000163')
     where name in ('pub', 'uncap') and billing_hidden;
    if got <> 0 then
        raise exception 'hidden count past_due: public/capture-failed repos must never mark hidden, % did', got;
    end if;
    select count(*) into got from dojo.org_repositories('b0000000-0000-4000-8000-000000000163')
     where name in ('pub', 'uncap');
    if got <> 2 then
        raise exception 'hidden count past_due: public/capture-failed repos must still be listed, % were', got;
    end if;
end $$;

rollback;
