-- Same repo, every entitlement outcome, identical entitlement_refusal at both
-- grains (org-repositories r2, test 7; the D1 lock: the member read and the
-- org read must compute the verdict from one shared function, never two
-- CASEs that drift).
--
-- WHAT BREAKS THIS TEST: a missing dojo.org_repositories(); any outcome the
-- org grain computes differently from the member grain (a term dropped,
-- reordered, or reworded in the extraction); a new outcome added to one
-- grain and not the other.
--
-- Scope note: reason_code's election pick (not_elected_org/user) is
-- deliberately out — the org read returns entitlement_refusal only, and
-- election rides the separate elected boolean (test 5).
--
-- Phase A: red — dojo.org_repositories(uuid) does not exist yet.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000171', 'a1000000-0000-4000-8000-000000000171', 'ada'),
  ('a0000000-0000-4000-8000-000000000172', 'a1000000-0000-4000-8000-000000000172', 'ola');

-- Nine org tenants, one per entitlement outcome. The unclaimed tenant holds
-- an active billing row on purpose: unclaimed must beat billing.
insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-000000000171', 'organization/ztest-orrepos-punc', 'organization',
   'ztest-orrepos-punc', 'punc', 'dojo.sensei-hq.org/organization/ztest-orrepos-punc', null),
  ('b0000000-0000-4000-8000-000000000172', 'organization/ztest-orrepos-pnor', 'organization',
   'ztest-orrepos-pnor', 'pnor', 'dojo.sensei-hq.org/organization/ztest-orrepos-pnor', now()),
  ('b0000000-0000-4000-8000-000000000173', 'organization/ztest-orrepos-pdue', 'organization',
   'ztest-orrepos-pdue', 'pdue', 'dojo.sensei-hq.org/organization/ztest-orrepos-pdue', now()),
  ('b0000000-0000-4000-8000-000000000174', 'organization/ztest-orrepos-pper', 'organization',
   'ztest-orrepos-pper', 'pper', 'dojo.sensei-hq.org/organization/ztest-orrepos-pper', now()),
  ('b0000000-0000-4000-8000-000000000175', 'organization/ztest-orrepos-pexp', 'organization',
   'ztest-orrepos-pexp', 'pexp', 'dojo.sensei-hq.org/organization/ztest-orrepos-pexp', now()),
  ('b0000000-0000-4000-8000-000000000176', 'organization/ztest-orrepos-punk', 'organization',
   'ztest-orrepos-punk', 'punk', 'dojo.sensei-hq.org/organization/ztest-orrepos-punk', now()),
  ('b0000000-0000-4000-8000-000000000177', 'organization/ztest-orrepos-pstl', 'organization',
   'ztest-orrepos-pstl', 'pstl', 'dojo.sensei-hq.org/organization/ztest-orrepos-pstl', now()),
  ('b0000000-0000-4000-8000-000000000178', 'organization/ztest-orrepos-ppub', 'organization',
   'ztest-orrepos-ppub', 'ppub', 'dojo.sensei-hq.org/organization/ztest-orrepos-ppub', now()),
  ('b0000000-0000-4000-8000-000000000179', 'organization/ztest-orrepos-pok', 'organization',
   'ztest-orrepos-pok', 'pok', 'dojo.sensei-hq.org/organization/ztest-orrepos-pok', now());

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via)
select t, 'a0000000-0000-4000-8000-000000000171', 'admin', 'employer', 'github_oauth'
  from (values ('b0000000-0000-4000-8000-000000000171'::uuid),
               ('b0000000-0000-4000-8000-000000000172'::uuid),
               ('b0000000-0000-4000-8000-000000000173'::uuid),
               ('b0000000-0000-4000-8000-000000000174'::uuid),
               ('b0000000-0000-4000-8000-000000000175'::uuid),
               ('b0000000-0000-4000-8000-000000000176'::uuid),
               ('b0000000-0000-4000-8000-000000000177'::uuid),
               ('b0000000-0000-4000-8000-000000000178'::uuid),
               ('b0000000-0000-4000-8000-000000000179')) v(t);

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via)
select t, 'a0000000-0000-4000-8000-000000000172', 'contributor', 'employer', 'github_oauth'
  from (values ('b0000000-0000-4000-8000-000000000171'::uuid),
               ('b0000000-0000-4000-8000-000000000172'::uuid),
               ('b0000000-0000-4000-8000-000000000173'::uuid),
               ('b0000000-0000-4000-8000-000000000174'::uuid),
               ('b0000000-0000-4000-8000-000000000175'::uuid),
               ('b0000000-0000-4000-8000-000000000176'::uuid),
               ('b0000000-0000-4000-8000-000000000177'::uuid),
               ('b0000000-0000-4000-8000-000000000178'::uuid),
               ('b0000000-0000-4000-8000-000000000179')) v(t);

insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b0000000-0000-4000-8000-000000000171', 'pro', 'active',   current_date - 1, current_date + 30),
  ('b0000000-0000-4000-8000-000000000173', 'pro', 'past_due', current_date - 1, current_date + 30),
  ('b0000000-0000-4000-8000-000000000174', 'pro', 'active',   null,             null),
  ('b0000000-0000-4000-8000-000000000175', 'pro', 'active',   current_date - 30, current_date - 1),
  ('b0000000-0000-4000-8000-000000000176', 'pro', 'active',   current_date - 1, current_date + 30),
  ('b0000000-0000-4000-8000-000000000177', 'pro', 'active',   current_date - 1, current_date + 30),
  ('b0000000-0000-4000-8000-000000000178', 'pro', 'active',   current_date - 1, current_date + 30),
  ('b0000000-0000-4000-8000-000000000179', 'pro', 'active',   current_date - 1, current_date + 30);

insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider) values
  ('90000000-0000-4000-8000-000000000171', 'b0000000-0000-4000-8000-000000000171',
   'github.com/ztest-orrepos-punc/core', 'core', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000172', 'b0000000-0000-4000-8000-000000000172',
   'github.com/ztest-orrepos-pnor/core', 'core', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000173', 'b0000000-0000-4000-8000-000000000173',
   'github.com/ztest-orrepos-pdue/core', 'core', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000174', 'b0000000-0000-4000-8000-000000000174',
   'github.com/ztest-orrepos-pper/core', 'core', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000175', 'b0000000-0000-4000-8000-000000000175',
   'github.com/ztest-orrepos-pexp/core', 'core', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000176', 'b0000000-0000-4000-8000-000000000176',
   'github.com/ztest-orrepos-punk/core', 'core', null, null, 'github'),
  ('90000000-0000-4000-8000-000000000177', 'b0000000-0000-4000-8000-000000000177',
   'github.com/ztest-orrepos-pstl/core', 'core', 'private', now() - interval '40 days', 'github'),
  ('90000000-0000-4000-8000-000000000178', 'b0000000-0000-4000-8000-000000000178',
   'github.com/ztest-orrepos-ppub/core', 'core', 'public', now(), 'github'),
  ('90000000-0000-4000-8000-000000000179', 'b0000000-0000-4000-8000-000000000179',
   'github.com/ztest-orrepos-pok/core', 'core', 'private', now(), 'github');

insert into dojo.repository_roles (repository_id, principal_id, forge_role)
select r, 'a0000000-0000-4000-8000-000000000172', 'contributor'
  from (values ('90000000-0000-4000-8000-000000000171'::uuid),
               ('90000000-0000-4000-8000-000000000172'::uuid),
               ('90000000-0000-4000-8000-000000000173'::uuid),
               ('90000000-0000-4000-8000-000000000174'::uuid),
               ('90000000-0000-4000-8000-000000000175'::uuid),
               ('90000000-0000-4000-8000-000000000176'::uuid),
               ('90000000-0000-4000-8000-000000000177'::uuid),
               ('90000000-0000-4000-8000-000000000178'::uuid),
               ('90000000-0000-4000-8000-000000000179')) v(r);

set local role authenticated;

do $$
declare
    t uuid[];
    k text[];
    want text[];
    i int;
    member_got text[];
    member_one text;
    org_got text;
begin
    -- (tenant, repo_key, expected refusal at BOTH grains).
    t := array['b0000000-0000-4000-8000-000000000171',
               'b0000000-0000-4000-8000-000000000172',
               'b0000000-0000-4000-8000-000000000173',
               'b0000000-0000-4000-8000-000000000174',
               'b0000000-0000-4000-8000-000000000175',
               'b0000000-0000-4000-8000-000000000176',
               'b0000000-0000-4000-8000-000000000177',
               'b0000000-0000-4000-8000-000000000178',
               'b0000000-0000-4000-8000-000000000179'];
    k := array['github.com/ztest-orrepos-punc/core',
               'github.com/ztest-orrepos-pnor/core',
               'github.com/ztest-orrepos-pdue/core',
               'github.com/ztest-orrepos-pper/core',
               'github.com/ztest-orrepos-pexp/core',
               'github.com/ztest-orrepos-punk/core',
               'github.com/ztest-orrepos-pstl/core',
               'github.com/ztest-orrepos-ppub/core',
               'github.com/ztest-orrepos-pok/core'];
    want := array['unclaimed',
                  'not_subscribed',
                  'not_subscribed',
                  'not_subscribed',
                  'subscription_expired',
                  'forge_visibility_unknown',
                  'forge_visibility_stale',
                  null,
                  null];
    -- All 9 member checks BEFORE the first org call, so the file
    -- self-proves the matrix in phase A too (red at outcome 1's org
    -- call, with all 9 member verdicts already confirmed).
    for i in 1 .. 9 loop
        -- Member grain, as the contributor.
        perform set_config('request.jwt.claims',
            json_build_object('sub', 'a1000000-0000-4000-8000-000000000172')::text, true);
        select entitlement_refusal into member_one from dojo.my_repositories()
         where repo_key = k[i];
        if member_one is distinct from want[i] then
            raise exception 'parity(%): member grain expected %, got %', k[i], want[i], member_one;
        end if;
        member_got[i] := member_one;
    end loop;
    for i in 1 .. 9 loop
        -- Org grain, as the admin: identical verdict, same repo.
        perform set_config('request.jwt.claims',
            json_build_object('sub', 'a1000000-0000-4000-8000-000000000171')::text, true);
        select entitlement_refusal into org_got from dojo.org_repositories(t[i])
         where repo_key = k[i];
        if org_got is distinct from want[i] then
            raise exception 'parity(%): org grain expected %, got % (member grain said %)',
                k[i], want[i], org_got, member_got[i];
        end if;
    end loop;
end $$;

rollback;
