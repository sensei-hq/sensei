-- WP8: an organisation's private repository keeps EVERY billing term (D46
-- changes personal only; D88: no grace — `subscription_expired` hides too,
-- both halves: the lapse behind and the period ahead).
--
-- Green before and after: this pins that WP8's entitlement rewrite preserves
-- the org-private terms, each on a CLAIMED org tenant (an unclaimed org
-- refuses `unclaimed` above billing). After WP8 a personal tenant never
-- reaches these terms, so these are their only pins.
--
-- Run through --apply (the view DDL is passed in, not \ir'd): the file seeds,
-- asserts, rolls back. Nothing persists.
begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a2222222-2222-2222-2222-222222222222', 'aa222222-2222-2222-2222-222222222222', 'Alice');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b2222222-2222-2222-2222-222222222222', 'organization/ztest-wp8-nobill', 'organization', 'ztest-wp8-nobill',
   'No billing row', 'dojo.sensei-hq.org/organization/ztest-wp8-nobill', now()),
  ('b7777777-7777-7777-7777-777777777777', 'organization/ztest-wp8-unowned', 'organization', 'ztest-wp8-unowned',
   'Unclaimed', 'dojo.sensei-hq.org/organization/ztest-wp8-unowned', null),
  ('b8888888-8888-8888-8888-888888888888', 'organization/ztest-wp8-noperiod', 'organization', 'ztest-wp8-noperiod',
   'No period', 'dojo.sensei-hq.org/organization/ztest-wp8-noperiod', now()),
  ('b9999999-9999-9999-9999-999999999999', 'organization/ztest-wp8-pastdue', 'organization', 'ztest-wp8-pastdue',
   'Past due', 'dojo.sensei-hq.org/organization/ztest-wp8-pastdue', now()),
  ('baaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'organization/ztest-wp8-trial', 'organization', 'ztest-wp8-trial',
   'Trialing', 'dojo.sensei-hq.org/organization/ztest-wp8-trial', now()),
  ('bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', 'organization/ztest-wp8-lapsed', 'organization', 'ztest-wp8-lapsed',
   'Lapsed', 'dojo.sensei-hq.org/organization/ztest-wp8-lapsed', now()),
  ('bccccccc-cccc-cccc-cccc-cccccccccccc', 'organization/ztest-wp8-lastday', 'organization', 'ztest-wp8-lastday',
   'Last day', 'dojo.sensei-hq.org/organization/ztest-wp8-lastday', now()),
  ('bddddddd-dddd-dddd-dddd-dddddddddddd', 'organization/ztest-wp8-future', 'organization', 'ztest-wp8-future',
   'Period not started', 'dojo.sensei-hq.org/organization/ztest-wp8-future', now());

insert into dojo.memberships (tenant_id, user_id, kind, authenticated_via, role, disabled_at)
  select t.id, 'a2222222-2222-2222-2222-222222222222', 'employer', 'github_oauth', 'admin', null
    from dojo.tenants t
   where t.key like 'organization/ztest-wp8-%';

insert into dojo.repositories (tenant_id, repo_key, remote_url, name, provider, visibility, visibility_captured_at)
  select t.id, 'github.com/' || t.slug || '/api', 'git@github.com:' || t.slug || '/api.git',
         'api', 'github', 'private', now()
    from dojo.tenants t
   where t.key like 'organization/ztest-wp8-%';

-- The billing shapes. No row for nobill and unowned: nothing to bill, and an
-- unclaimed org refuses above billing anyway.
insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b8888888-8888-8888-8888-888888888888', 'team', 'active',   null,              null),
  ('b9999999-9999-9999-9999-999999999999', 'team', 'past_due', current_date - 40, current_date - 1),
  ('baaaaaaa-aaaa-aaaa-aaaa-aaaaaaaaaaaa', 'team', 'trialing', current_date - 3,  current_date + 11),
  ('bbbbbbbb-bbbb-bbbb-bbbb-bbbbbbbbbbbb', 'team', 'active',   current_date - 40, current_date - 1),
  ('bccccccc-cccc-cccc-cccc-cccccccccccc', 'team', 'active',   current_date - 10, current_date),
  ('bddddddd-dddd-dddd-dddd-dddddddddddd', 'team', 'active',   current_date + 1,  current_date + 31);

-- The entitled cases elect, so the verdict is the full syncing row: trialing
-- and the last day are subscriptions, not almost-subscriptions.
insert into dojo.repository_elections (tenant_id, repository_id, authority, principal_id, elected)
  select r.tenant_id, r.id, 'organization', null, true
    from dojo.repositories r
    join dojo.tenants t on t.id = r.tenant_id
   where t.key in ('organization/ztest-wp8-trial', 'organization/ztest-wp8-lastday');

do $$
declare
    expected record;
    got      record;
begin
    for expected in
        select * from (values
            --  label                        repo_key                                  may_share elected sync   refused_by     reason_code
            ('no billing row',            'github.com/ztest-wp8-nobill/api',       false, false,  false, 'entitlement', 'not_subscribed'),
            ('unclaimed org',             'github.com/ztest-wp8-unowned/api',      false, false,  false, 'entitlement', 'unclaimed'),
            ('active row with no period', 'github.com/ztest-wp8-noperiod/api',    false, false,  false, 'entitlement', 'not_subscribed'),
            ('past due',                  'github.com/ztest-wp8-pastdue/api',      false, false,  false, 'entitlement', 'not_subscribed'),
            ('trialing is a subscription','github.com/ztest-wp8-trial/api',        true,  true,   true,  null,          null),
            ('the period ended yesterday','github.com/ztest-wp8-lapsed/api',      false, false,  false, 'entitlement', 'subscription_expired'),
            ('last day of the period',    'github.com/ztest-wp8-lastday/api',      true,  true,   true,  null,          null),
            ('period starts tomorrow',    'github.com/ztest-wp8-future/api',       false, false,  false, 'entitlement', 'subscription_expired')
        ) as t(label, repo_key, may_share, elected, sync_enabled, refused_by, reason_code)
    loop
        select v.may_share, v.elected, v.sync_enabled, v.refused_by, v.reason_code
          into got
          from dojo.all_my_repositories v
         where v.repo_key = expected.repo_key
           and v.principal_id = 'a2222222-2222-2222-2222-222222222222';
        if not found then
            raise exception 'no row for % (%).', expected.label, expected.repo_key;
        end if;
        if got.may_share is distinct from expected.may_share
           or got.elected is distinct from expected.elected
           or got.sync_enabled is distinct from expected.sync_enabled
           or got.refused_by is distinct from expected.refused_by
           or got.reason_code is distinct from expected.reason_code then
            raise exception '% (%): expected (may=%, elected=%, sync=%, by=%, reason=%), got (may=%, elected=%, sync=%, by=%, reason=%).',
                expected.label, expected.repo_key,
                expected.may_share, expected.elected, expected.sync_enabled,
                coalesce(expected.refused_by, 'NONE'), coalesce(expected.reason_code, 'NONE'),
                got.may_share, got.elected, got.sync_enabled,
                coalesce(got.refused_by, 'NONE'), coalesce(got.reason_code, 'NONE');
        end if;
    end loop;
end $$;

rollback;
