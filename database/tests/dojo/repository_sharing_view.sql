-- `dojo.all_my_repositories` — the mechanics behind the §8b verdicts.
--
-- The scenario table (repository_sharing_scenarios.sql) proves the twelve cases
-- a reader can name. This file proves the five things that make those cases hold
-- for the right REASON, each of which is a finding from an adversarial review of
-- the design:
--
--   1. EVERY derived column carries its own `visibility is null` guard. Postgres
--      three-valued logic is why: `'organization' = 'organization' and NULL <>
--      'public'` is NULL, not FALSE, so a CASE without a leading guard does not
--      take the ORG branch — it falls through to the ELSE and reads the USER's
--      election for a repository that has no authority at all.
--   2. A STALE capture is a bypass, not a lag. `public → free` fires before every
--      billing term, so a repository that went private upstream would keep
--      syncing free under an authority that no longer applies.
--   3. Entitlement tests for the MISSING billing row before its value, and the
--      period test is HALF-OPEN — `period_end` is a DATE, and `between` denies
--      the whole final day.
--   4. The registry is joined LEFT and coalesced. An untranslated code must
--      surface raw; dropping a repository out of a sync-decision view is worse.
--   5. `last_synced_at` / `metric_rows` are per (repository, MEMBER). The Worker
--      reads this view as service_role, which bypasses the RLS on
--      dojo.repository_metrics — so without the principal predicate, member A's
--      row shows member B's push timestamp and contribution volume.
--      `can_read_repository_metric` names that outcome: "metrics by user visible
--      to every peer is surveillance, not transparency."
--
-- WHAT BREAKS THIS TEST: removing any one of those five guards. Each has an
-- assertion that goes red on its own.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('5ecd0000-0000-0000-0000-00000000a11c', '5ecd0000-0000-0000-0000-00000000a11d', 'ztest Alice (admin)'),
  ('5ecd0000-0000-0000-0000-00000000b0b0', '5ecd0000-0000-0000-0000-00000000b0b1', 'ztest Bob (contributor)'),
  ('5ecd0000-0000-0000-0000-00000000ca01', '5ecd0000-0000-0000-0000-00000000ca02', 'ztest Carol (maintainer)'),
  ('5ecd0000-0000-0000-0000-00000000da7e', '5ecd0000-0000-0000-0000-00000000da7f', 'ztest Dave (lead)');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('5ecd0000-0000-0000-0000-000000000001', 'organization/ztest-v-org', 'organization', 'ztest-v-org',
   'Org', 'dojo.sensei-hq.org/organization/ztest-v-org'),
  ('5ecd0000-0000-0000-0000-000000000002', 'personal/ztest-v-lastday', 'personal', 'ztest-v-lastday',
   'Last day of the period', 'dojo.sensei-hq.org/personal/ztest-v-lastday'),
  ('5ecd0000-0000-0000-0000-000000000003', 'personal/ztest-v-lapsed', 'personal', 'ztest-v-lapsed',
   'Period ended yesterday', 'dojo.sensei-hq.org/personal/ztest-v-lapsed'),
  ('5ecd0000-0000-0000-0000-000000000004', 'personal/ztest-v-noperiod', 'personal', 'ztest-v-noperiod',
   'Active with no period', 'dojo.sensei-hq.org/personal/ztest-v-noperiod'),
  ('5ecd0000-0000-0000-0000-000000000005', 'personal/ztest-v-trial', 'personal', 'ztest-v-trial',
   'Trialing', 'dojo.sensei-hq.org/personal/ztest-v-trial');

insert into dojo.memberships (tenant_id, user_id, kind, authenticated_via, role) values
  ('5ecd0000-0000-0000-0000-000000000001', '5ecd0000-0000-0000-0000-00000000a11c', 'employer', 'github_oauth', 'admin'),
  ('5ecd0000-0000-0000-0000-000000000001', '5ecd0000-0000-0000-0000-00000000b0b0', 'employer', 'github_oauth', 'contributor'),
  ('5ecd0000-0000-0000-0000-000000000001', '5ecd0000-0000-0000-0000-00000000ca01', 'employer', 'github_oauth', 'maintainer'),
  ('5ecd0000-0000-0000-0000-000000000001', '5ecd0000-0000-0000-0000-00000000da7e', 'employer', 'github_oauth', 'lead'),
  ('5ecd0000-0000-0000-0000-000000000002', '5ecd0000-0000-0000-0000-00000000a11c', 'personal', 'github_oauth', 'admin'),
  ('5ecd0000-0000-0000-0000-000000000003', '5ecd0000-0000-0000-0000-00000000a11c', 'personal', 'github_oauth', 'admin'),
  ('5ecd0000-0000-0000-0000-000000000004', '5ecd0000-0000-0000-0000-00000000a11c', 'personal', 'github_oauth', 'admin'),
  ('5ecd0000-0000-0000-0000-000000000005', '5ecd0000-0000-0000-0000-00000000a11c', 'personal', 'github_oauth', 'admin');

insert into dojo.billing_accounts (tenant_id, status, period_start, period_end) values
  ('5ecd0000-0000-0000-0000-000000000001', 'active',   current_date - 10, current_date + 20),
  -- The LAST DAY of the period. `now() between period_start and period_end` casts
  -- the DATE to midnight and denies the whole day, announcing a lapse that has
  -- not happened. Half-open — `now() < period_end + 1` — is what keeps it live.
  ('5ecd0000-0000-0000-0000-000000000002', 'active',   current_date - 10, current_date),
  ('5ecd0000-0000-0000-0000-000000000003', 'active',   current_date - 40, current_date - 1),
  -- Status says active, but there is no period at all: not a lapse, a row that
  -- never carried a subscription.
  ('5ecd0000-0000-0000-0000-000000000004', 'active',   null,              null),
  -- Trialing IS a subscription. Excluding it demos the product with its core
  -- proposition switched off.
  ('5ecd0000-0000-0000-0000-000000000005', 'trialing', current_date - 3,  current_date + 11);

insert into dojo.tenant_share_policy (tenant_id, private_repos_shared, set_at) values
  ('5ecd0000-0000-0000-0000-000000000001', true, timestamptz '2026-02-02 09:00:00+00');

insert into dojo.repositories (id, tenant_id, repo_key, name, provider, visibility, visibility_captured_at) values
  -- never captured
  ('5ecd0000-0000-0000-0000-000000000010', '5ecd0000-0000-0000-0000-000000000001',
   'github.com/ztest-v-org/unknown', 'unknown', 'github', null, null),
  -- captured, but long ago: an old capture is not a capture
  ('5ecd0000-0000-0000-0000-000000000011', '5ecd0000-0000-0000-0000-000000000001',
   'github.com/ztest-v-org/stale', 'stale', 'github', 'public', now() - interval '400 days'),
  -- a value with no capture timestamp — undatable, therefore untrustworthy
  ('5ecd0000-0000-0000-0000-000000000012', '5ecd0000-0000-0000-0000-000000000001',
   'github.com/ztest-v-org/undated', 'undated', 'github', 'private', null),
  -- org-private, freshly captured: the org decides
  ('5ecd0000-0000-0000-0000-000000000013', '5ecd0000-0000-0000-0000-000000000001',
   'github.com/ztest-v-org/private', 'private', 'github', 'private', now()),
  -- org-public, freshly captured: the member decides, whatever their role
  ('5ecd0000-0000-0000-0000-000000000014', '5ecd0000-0000-0000-0000-000000000001',
   'github.com/ztest-v-org/public', 'public', 'github', 'public', now()),
  -- a capture that recorded WHEN but not WHAT. The two columns are written
  -- together today, so this is the shape a half-finished refresh leaves behind —
  -- and it must not read as "captured", or the freshness test would certify an
  -- answer that was never given.
  ('5ecd0000-0000-0000-0000-000000000015', '5ecd0000-0000-0000-0000-000000000001',
   'github.com/ztest-v-org/timestamped', 'timestamped', 'github', null, now()),
  ('5ecd0000-0000-0000-0000-000000000020', '5ecd0000-0000-0000-0000-000000000002',
   'github.com/ztest-v-lastday/r', 'r', 'github', 'private', now()),
  ('5ecd0000-0000-0000-0000-000000000021', '5ecd0000-0000-0000-0000-000000000003',
   'github.com/ztest-v-lapsed/r', 'r', 'github', 'private', now()),
  ('5ecd0000-0000-0000-0000-000000000022', '5ecd0000-0000-0000-0000-000000000004',
   'github.com/ztest-v-noperiod/r', 'r', 'github', 'private', now()),
  ('5ecd0000-0000-0000-0000-000000000023', '5ecd0000-0000-0000-0000-000000000005',
   'github.com/ztest-v-trial/r', 'r', 'github', 'private', now()),
  -- subscribed and never elected: the row the registry join is tested on
  ('5ecd0000-0000-0000-0000-000000000024', '5ecd0000-0000-0000-0000-000000000002',
   'github.com/ztest-v-lastday/unelected', 'unelected', 'github', 'private', now());

insert into dojo.repository_elections (tenant_id, repository_id, authority, principal_id, elected, elected_at) values
  -- Alice elected BOTH the uncaptured and the stale repository. Neither may
  -- apply: there is no authority to apply it under.
  ('5ecd0000-0000-0000-0000-000000000001', '5ecd0000-0000-0000-0000-000000000010', 'user',
   '5ecd0000-0000-0000-0000-00000000a11c', true, now()),
  ('5ecd0000-0000-0000-0000-000000000001', '5ecd0000-0000-0000-0000-000000000011', 'user',
   '5ecd0000-0000-0000-0000-00000000a11c', true, now()),
  ('5ecd0000-0000-0000-0000-000000000001', '5ecd0000-0000-0000-0000-000000000012', 'user',
   '5ecd0000-0000-0000-0000-00000000a11c', true, now()),
  ('5ecd0000-0000-0000-0000-000000000002', '5ecd0000-0000-0000-0000-000000000020', 'user',
   '5ecd0000-0000-0000-0000-00000000a11c', true, timestamptz '2026-03-03 10:00:00+00'),
  ('5ecd0000-0000-0000-0000-000000000003', '5ecd0000-0000-0000-0000-000000000021', 'user',
   '5ecd0000-0000-0000-0000-00000000a11c', true, now()),
  ('5ecd0000-0000-0000-0000-000000000004', '5ecd0000-0000-0000-0000-000000000022', 'user',
   '5ecd0000-0000-0000-0000-00000000a11c', true, now()),
  ('5ecd0000-0000-0000-0000-000000000005', '5ecd0000-0000-0000-0000-000000000023', 'user',
   '5ecd0000-0000-0000-0000-00000000a11c', true, now());

-- Two metric rows on the org-private repository: one describing the REPOSITORY,
-- one describing BOB. Bob's is the more recent, so a missing principal predicate
-- shows up as Alice inheriting his timestamp.
insert into dojo.repository_metrics
       (tenant_id, repository_id, metric_id, scope, principal_id, computed_on, value, pushed_at)
values ('5ecd0000-0000-0000-0000-000000000001', '5ecd0000-0000-0000-0000-000000000013',
        (select id from sensei.metrics order by key limit 1), 'repo', null,
        date '2026-01-01', 1, timestamptz '2026-01-01 00:00:00+00'),
       ('5ecd0000-0000-0000-0000-000000000001', '5ecd0000-0000-0000-0000-000000000013',
        (select id from sensei.metrics order by key limit 1), 'user',
        '5ecd0000-0000-0000-0000-00000000b0b0',
        date '2026-06-01', 2, timestamptz '2026-06-01 00:00:00+00');

-- ---------------------------------------------------------------------------
-- 1. Each derived column guards `visibility is null` ITSELF.
-- ---------------------------------------------------------------------------
do $$
declare
    v record;
begin
    select * into v from dojo.all_my_repositories
     where repo_key = 'github.com/ztest-v-org/unknown'
       and principal_id = '5ecd0000-0000-0000-0000-00000000a11c';

    if v.authority is not null then
        raise exception 'an uncaptured repository must have NO authority, got %.', v.authority;
    end if;
    -- THE MUTATION PROBE. Alice elected this repository. If `elected` lacked its
    -- own leading guard, the ORG branch's `NULL <> ''public''` would evaluate to
    -- NULL, the CASE would fall to the ELSE, and this would read her election as
    -- TRUE for a repository whose authority is NOBODY.
    if v.elected is not false then
        raise exception 'uncaptured repository read an election: elected = % (expected false). The `visibility is null` guard is missing from the elected CASE.',
            coalesce(v.elected::text, 'NULL');
    end if;
    if v.may_share is not false or v.reason_code <> 'forge_visibility_unknown'
       or v.refused_by <> 'entitlement' then
        raise exception 'uncaptured: expected (may_share=false, entitlement, forge_visibility_unknown), got (%, %, %)',
            coalesce(v.may_share::text, 'NULL'), coalesce(v.refused_by, 'NULL'),
            coalesce(v.reason_code, 'NULL');
    end if;
    if v.configurable_by_me is not false then
        raise exception 'there is nothing to configure until the forge answers; configurable_by_me = %.',
            coalesce(v.configurable_by_me::text, 'NULL');
    end if;
    if v.configured_by is not null or v.configured_at is not null then
        raise exception 'an uncaptured repository must report no configuration, got (%, %).',
            coalesce(v.configured_by::text, 'NULL'), coalesce(v.configured_at::text, 'NULL');
    end if;
    if v.forge_visibility is not null then
        raise exception 'forge_visibility must stay NULL when nothing was captured, got %.', v.forge_visibility;
    end if;

    -- A visibility with no capture timestamp is undatable, so it cannot be shown
    -- to be fresh — treated exactly as never captured.
    select * into v from dojo.all_my_repositories
     where repo_key = 'github.com/ztest-v-org/undated'
       and principal_id = '5ecd0000-0000-0000-0000-00000000a11c';
    if v.authority is not null or v.elected is not false
       or v.reason_code <> 'forge_visibility_unknown' then
        raise exception 'an undated capture must fail closed, got (authority=%, elected=%, reason=%)',
            coalesce(v.authority::text, 'NULL'), coalesce(v.elected::text, 'NULL'),
            coalesce(v.reason_code, 'NULL');
    end if;

    -- …and the mirror image: a timestamp with no answer. Fresh by date, empty in
    -- substance. `may_share` must still refuse, and with the code that says we do
    -- not know rather than one implying somebody decided.
    select * into v from dojo.all_my_repositories
     where repo_key = 'github.com/ztest-v-org/timestamped'
       and principal_id = '5ecd0000-0000-0000-0000-00000000a11c';
    if v.may_share is not false or v.reason_code <> 'forge_visibility_unknown'
       or v.refused_by <> 'entitlement' or v.authority is not null then
        raise exception 'a timestamped non-answer must fail closed, got (may_share=%, refused_by=%, reason=%, authority=%)',
            coalesce(v.may_share::text, 'NULL'), coalesce(v.refused_by, 'NULL'),
            coalesce(v.reason_code, 'NULL'), coalesce(v.authority::text, 'NULL');
    end if;
end $$;

-- ---------------------------------------------------------------------------
-- 2. A stale capture is a bypass, not a lag.
-- ---------------------------------------------------------------------------
do $$
declare
    v record;
begin
    select * into v from dojo.all_my_repositories
     where repo_key = 'github.com/ztest-v-org/stale'
       and principal_id = '5ecd0000-0000-0000-0000-00000000a11c';

    -- The repository is recorded `public` and Alice elected it. If the freshness
    -- test is dropped, `public → free` fires ABOVE every billing term and this
    -- syncs for nothing, under a user authority the org may no longer hold.
    if v.may_share is not false or v.reason_code <> 'forge_visibility_stale'
       or v.refused_by <> 'entitlement' then
        raise exception 'a stale capture must refuse: expected (false, entitlement, forge_visibility_stale), got (%, %, %)',
            coalesce(v.may_share::text, 'NULL'), coalesce(v.refused_by, 'NULL'),
            coalesce(v.reason_code, 'NULL');
    end if;
    if v.authority is not null or v.elected is not false or v.sync_enabled is not false then
        raise exception 'a stale capture must fail closed on BOTH axes, got (authority=%, elected=%, sync=%)',
            coalesce(v.authority::text, 'NULL'), coalesce(v.elected::text, 'NULL'),
            coalesce(v.sync_enabled::text, 'NULL');
    end if;
    if v.forge_visibility is not null then
        raise exception 'a stale capture must not be reported as the forge''s current answer, got %.',
            v.forge_visibility;
    end if;
end $$;

-- ---------------------------------------------------------------------------
-- 3. Entitlement order: missing row before value, and a HALF-OPEN period.
-- ---------------------------------------------------------------------------
do $$
declare
    expected record;
    got      record;
begin
    for expected in
        select * from (values
            -- The final day of the period is still INSIDE it.
            ('last day of the period',      'github.com/ztest-v-lastday/r',  true,  null,                   null),
            ('the period ended yesterday',  'github.com/ztest-v-lapsed/r',   false, 'entitlement',          'subscription_expired'),
            -- A row with no period never carried a subscription: `not_subscribed`,
            -- not `subscription_expired`, because nothing lapsed.
            ('active row with no period',   'github.com/ztest-v-noperiod/r', false, 'entitlement',          'not_subscribed'),
            ('trialing is a subscription',  'github.com/ztest-v-trial/r',    true,  null,                   null)
        ) as t(label, repo_key, may_share, refused_by, reason_code)
    loop
        select v.may_share, v.refused_by, v.reason_code into got
          from dojo.all_my_repositories v
         where v.repo_key = expected.repo_key
           and v.principal_id = '5ecd0000-0000-0000-0000-00000000a11c';
        if not found then
            raise exception 'no row for % (%)', expected.label, expected.repo_key;
        end if;
        if got.may_share is distinct from expected.may_share
           or got.refused_by is distinct from expected.refused_by
           or got.reason_code is distinct from expected.reason_code then
            raise exception '%: expected (may_share=%, refused_by=%, reason=%), got (%, %, %)',
                expected.label, expected.may_share, coalesce(expected.refused_by, 'NULL'),
                coalesce(expected.reason_code, 'NULL'),
                coalesce(got.may_share::text, 'NULL'), coalesce(got.refused_by, 'NULL'),
                coalesce(got.reason_code, 'NULL');
        end if;
    end loop;
end $$;

-- ---------------------------------------------------------------------------
-- 4. The registry decorates; it never gates. LEFT JOIN + coalesce.
-- ---------------------------------------------------------------------------
do $$
declare
    v record;
    n int;
begin
    select * into v from dojo.all_my_repositories
     where repo_key = 'github.com/ztest-v-lastday/unelected'
       and principal_id = '5ecd0000-0000-0000-0000-00000000a11c';

    if v.reason_code <> 'not_elected_user' then
        raise exception 'expected not_elected_user, got %.', coalesce(v.reason_code, 'NULL');
    end if;
    if v.reason <> 'You have not turned sharing on for this repository'
       or v.remedy <> 'Turn sharing on for this repository'
       or v.reason_actor::text <> 'user'
       or v.reason_detail is null then
        raise exception 'the registry did not decorate the verdict: (reason=%, remedy=%, actor=%, detail present=%)',
            coalesce(v.reason, 'NULL'), coalesce(v.remedy, 'NULL'),
            coalesce(v.reason_actor::text, 'NULL'), (v.reason_detail is not null);
    end if;

    -- A repository that IS syncing carries no reason at all.
    select count(*) into n from dojo.all_my_repositories
     where repo_key = 'github.com/ztest-v-trial/r'
       and principal_id = '5ecd0000-0000-0000-0000-00000000a11c'
       and sync_enabled
       and reason_code is null and reason is null and refused_by is null
       and remedy is null and reason_actor is null;
    if n <> 1 then
        raise exception 'a syncing repository must carry no refusal decoration.';
    end if;

    -- Untranslated code: the row must survive with the RAW code showing. An
    -- inner join would delete a repository from a sync-decision view because a
    -- prose row was missing, which is far worse than an unreadable string.
    delete from sensei.reason_codes
     where domain = 'repository_sharing' and code = 'not_elected_user';

    select * into v from dojo.all_my_repositories
     where repo_key = 'github.com/ztest-v-lastday/unelected'
       and principal_id = '5ecd0000-0000-0000-0000-00000000a11c';
    if not found then
        raise exception 'a repository vanished from the view because its reason code had no registry row — the join must be LEFT.';
    end if;
    if v.reason <> 'not_elected_user' then
        raise exception 'an untranslated code must surface raw, got %.', coalesce(v.reason, 'NULL');
    end if;
    if v.sync_enabled is not false or v.may_share is not true or v.elected is not false then
        raise exception 'the verdict changed when a registry row was deleted — behaviour is reading the registry.';
    end if;
end $$;

-- ---------------------------------------------------------------------------
-- 5. last_synced_at / metric_rows are per (repository, MEMBER).
-- ---------------------------------------------------------------------------
do $$
declare
    v record;
begin
    -- Alice is an ADMIN and still does not inherit Bob's user-scoped row here.
    -- The view is read by the Worker as service_role, which bypasses the RLS
    -- that would otherwise decide this, so the predicate is the whole boundary.
    select * into v from dojo.all_my_repositories
     where repo_key = 'github.com/ztest-v-org/private'
       and principal_id = '5ecd0000-0000-0000-0000-00000000a11c';
    if v.metric_rows <> 1 then
        raise exception 'Alice must see only the repo-scoped row (1), saw %.', v.metric_rows;
    end if;
    if v.last_synced_at <> timestamptz '2026-01-01 00:00:00+00' then
        raise exception 'Alice inherited another member''s push timestamp: %.', v.last_synced_at;
    end if;

    -- Bob sees the repository's row AND his own.
    select * into v from dojo.all_my_repositories
     where repo_key = 'github.com/ztest-v-org/private'
       and principal_id = '5ecd0000-0000-0000-0000-00000000b0b0';
    if v.metric_rows <> 2 or v.last_synced_at <> timestamptz '2026-06-01 00:00:00+00' then
        raise exception 'Bob must see both his own and the repo-scoped rows, got (rows=%, last=%).',
            v.metric_rows, coalesce(v.last_synced_at::text, 'NULL');
    end if;

    -- A repository nobody has pushed reports zero and NULL — genuinely empty, not
    -- a masked failure.
    select * into v from dojo.all_my_repositories
     where repo_key = 'github.com/ztest-v-org/public'
       and principal_id = '5ecd0000-0000-0000-0000-00000000b0b0';
    if v.metric_rows <> 0 or v.last_synced_at is not null then
        raise exception 'an unpushed repository must report (0, NULL), got (%, %).',
            v.metric_rows, coalesce(v.last_synced_at::text, 'NULL');
    end if;
end $$;

-- ---------------------------------------------------------------------------
-- 6. configurable_by_me is viewer-relative, and ADMIN-only on org-private.
-- ---------------------------------------------------------------------------
do $$
declare
    expected record;
    got      boolean;
begin
    for expected in
        select * from (values
            -- Org-PRIVATE: the org's policy is the thing being changed, and
            -- dojo.member_role assigns policy to `admin`. Every seeded remedy for
            -- an org-authority refusal says "ask an admin" — a lead who could
            -- flip it would make that copy a lie, and a lead guards client
            -- confidentiality rather than running provisioning.
            ('github.com/ztest-v-org/private', '5ecd0000-0000-0000-0000-00000000a11c', true),   -- admin
            ('github.com/ztest-v-org/private', '5ecd0000-0000-0000-0000-00000000da7e', false),  -- lead
            ('github.com/ztest-v-org/private', '5ecd0000-0000-0000-0000-00000000ca01', false),  -- maintainer
            ('github.com/ztest-v-org/private', '5ecd0000-0000-0000-0000-00000000b0b0', false),  -- contributor
            -- Org-PUBLIC: the member's own call, whatever their role.
            ('github.com/ztest-v-org/public',  '5ecd0000-0000-0000-0000-00000000b0b0', true),
            -- Personal: always the member's.
            ('github.com/ztest-v-lastday/r',   '5ecd0000-0000-0000-0000-00000000a11c', true)
        ) as t(repo_key, principal_id, configurable)
    loop
        select v.configurable_by_me into got
          from dojo.all_my_repositories v
         where v.repo_key = expected.repo_key
           and v.principal_id = expected.principal_id::uuid;
        if got is distinct from expected.configurable then
            raise exception 'configurable_by_me for % / %: expected %, got %.',
                expected.repo_key, expected.principal_id, expected.configurable,
                coalesce(got::text, 'NULL');
        end if;
    end loop;
end $$;

-- ---------------------------------------------------------------------------
-- 7. configured_by / configured_at name WHO decided and WHEN.
-- ---------------------------------------------------------------------------
do $$
declare
    v record;
begin
    -- A user's own election.
    select * into v from dojo.all_my_repositories
     where repo_key = 'github.com/ztest-v-lastday/r'
       and principal_id = '5ecd0000-0000-0000-0000-00000000a11c';
    if v.configured_by::text <> 'user' or v.configured_at <> timestamptz '2026-03-03 10:00:00+00' then
        raise exception 'expected the user''s election to be reported, got (%, %).',
            coalesce(v.configured_by::text, 'NULL'), coalesce(v.configured_at::text, 'NULL');
    end if;

    -- An org-private repository covered by the TENANT POLICY and no per-repo row:
    -- the organisation configured it, when the policy was set.
    select * into v from dojo.all_my_repositories
     where repo_key = 'github.com/ztest-v-org/private'
       and principal_id = '5ecd0000-0000-0000-0000-00000000b0b0';
    if v.configured_by::text <> 'organization' or v.configured_at <> timestamptz '2026-02-02 09:00:00+00' then
        raise exception 'expected the tenant policy to be reported as the organisation''s configuration, got (%, %).',
            coalesce(v.configured_by::text, 'NULL'), coalesce(v.configured_at::text, 'NULL');
    end if;

    -- Nobody has decided: no election row, and the user holds authority.
    select * into v from dojo.all_my_repositories
     where repo_key = 'github.com/ztest-v-lastday/unelected'
       and principal_id = '5ecd0000-0000-0000-0000-00000000a11c';
    if v.configured_by is not null or v.configured_at is not null then
        raise exception 'an unconfigured repository must report nobody, got (%, %).',
            coalesce(v.configured_by::text, 'NULL'), coalesce(v.configured_at::text, 'NULL');
    end if;
end $$;

-- ---------------------------------------------------------------------------
-- 8. The Worker's role can actually read it.
--
-- `security_invoker = on` means the CALLER needs privileges on everything the
-- view touches, including the dojo.reason_codes view it joins and the
-- sensei.reason_codes table behind that. dojo.metric_catalogue answered
-- "permission denied" to precisely this read until the BASE table was granted
-- too — a failure no test on the owner connection can see, because the owner has
-- every privilege by definition.
-- ---------------------------------------------------------------------------
set local role service_role;
do $$
declare
    n int;
begin
    select count(*) into n
      from dojo.all_my_repositories
     where principal_id = '5ecd0000-0000-0000-0000-00000000a11c'
       and reason_code is not null;
    if n = 0 then
        raise exception 'service_role read the view but saw no decorated refusals — the registry join is not reachable as the Worker.';
    end if;
end $$;
reset role;

rollback;
