-- `dojo.all_my_repositories` — the worked scenarios of daemon-sync.md §8b.
--
-- Sharing is TWO questions, and this file is the proof that the view answers
-- both: `may_share` (ENTITLEMENT — may it?) and `elected` (ELECTION — did
-- whoever holds authority actually choose it?), with
-- `sync_enabled = may_share AND elected`.
--
-- Each row of §8b's table is one case here, asserted on the five columns a
-- consumer acts on: (authority, may_share, elected, sync_enabled, refused_by,
-- reason_code). "Allowed but nobody chose" and "chosen but not allowed" are
-- different states with different fixes, so a single boolean cannot stand in for
-- any of them.
--
-- WHAT BREAKS THIS TEST:
--   * `sync_enabled` going back to a constant (the shipped phase-1 view);
--   * dropping `origin = 'personal' → ALLOW` back INTO the entitlement CASE —
--     A2 turns green-to-red immediately, which is the point: a personal private
--     repository is subscription-gated like everyone else's;
--   * testing `billing.status <> 'active'` before testing whether a billing ROW
--     exists — H2 goes red, because `NULL <> 'active'` is NULL, not TRUE, and a
--     value test alone falls through to ALLOW;
--   * reading the USER's election for an ORG-authority repository (G), or the
--     ORG's policy for a public one (E);
--   * letting an uncaptured forge visibility resolve to anything at all (I).

begin;

-- ---------------------------------------------------------------------------
-- Fixtures. Every tenant key/slug is `ztest-`-prefixed: these run against a LIVE
-- database and dojo.tenants.key is globally unique, so a plausible name breaks
-- the day someone provisions it for real.
-- ---------------------------------------------------------------------------

insert into dojo.principals (id, auth_user_id, display_name) values
  ('5ecc0000-0000-0000-0000-00000000a11c', '5ecc0000-0000-0000-0000-00000000a11d', 'ztest Alice');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  -- personal, WITH an active subscription
  ('5ecc0000-0000-0000-0000-000000000001', 'personal/ztest-alice-sub', 'personal', 'ztest-alice-sub',
   'Alice (subscribed)', 'dojo.sensei-hq.org/personal/ztest-alice-sub'),
  -- personal, with NO billing row at all — the common case
  ('5ecc0000-0000-0000-0000-000000000002', 'personal/ztest-alice-free', 'personal', 'ztest-alice-free',
   'Alice (no billing)', 'dojo.sensei-hq.org/personal/ztest-alice-free'),
  -- org, subscribed, policy ON
  ('5ecc0000-0000-0000-0000-000000000003', 'organization/ztest-on', 'organization', 'ztest-on',
   'Acme (mandating)', 'dojo.sensei-hq.org/organization/ztest-on'),
  -- org, subscribed, policy OFF
  ('5ecc0000-0000-0000-0000-000000000004', 'organization/ztest-off', 'organization', 'ztest-off',
   'Acme (not mandating)', 'dojo.sensei-hq.org/organization/ztest-off'),
  -- org, billing row EXISTS and is past_due, policy ON
  ('5ecc0000-0000-0000-0000-000000000005', 'organization/ztest-pastdue', 'organization', 'ztest-pastdue',
   'Acme (lapsed)', 'dojo.sensei-hq.org/organization/ztest-pastdue'),
  -- org, NO billing row at all, policy ON
  ('5ecc0000-0000-0000-0000-000000000006', 'organization/ztest-nobill', 'organization', 'ztest-nobill',
   'Acme (never subscribed)', 'dojo.sensei-hq.org/organization/ztest-nobill'),
  -- org, subscribed, and NO tenant_share_policy row at all
  ('5ecc0000-0000-0000-0000-000000000007', 'organization/ztest-nopolicy', 'organization', 'ztest-nopolicy',
   'Acme (undecided)', 'dojo.sensei-hq.org/organization/ztest-nopolicy'),
  -- L: org, subscribed, policy ON, forge answer captured — right in EVERY way
  -- except that NOBODY HAS CLAIMED IT. Isolates the claim term: if the update
  -- below ever claims this one too, L goes green-for-the-wrong-reason and the
  -- gate is satisfied everywhere and therefore tested nowhere.
  ('5ecc0000-0000-0000-0000-000000000008', 'organization/ztest-unowned', 'organization', 'ztest-unowned',
   'Acme (nobody claimed it)', 'dojo.sensei-hq.org/organization/ztest-unowned');

-- CLAIM every ORG fixture (§II.4). Each scenario below tests a gate BELOW the
-- claim — billing, policy, election — and an unclaimed tenant refuses ABOVE all
-- of them with `unclaimed`, which would mask every one of those assertions with
-- the same reason. Personal tenants are never claimed: they ARE the person.
--
-- Scenario J covers the unclaimed case itself, so the term is not merely
-- satisfied everywhere and thereby untested.
update dojo.tenants
   set claimed_at = timestamptz '2026-01-01 09:00:00+00',
       claimed_by = '5ecc0000-0000-0000-0000-00000000a11c'
 where origin = 'organization'
   and key like 'organization/ztest-%'
   -- EXCEPT the one scenario L exists to test.
   and key <> 'organization/ztest-unowned';

insert into dojo.memberships (tenant_id, user_id, kind, authenticated_via, role) values
  ('5ecc0000-0000-0000-0000-000000000001', '5ecc0000-0000-0000-0000-00000000a11c', 'personal', 'github_oauth', 'admin'),
  ('5ecc0000-0000-0000-0000-000000000002', '5ecc0000-0000-0000-0000-00000000a11c', 'personal', 'github_oauth', 'admin'),
  ('5ecc0000-0000-0000-0000-000000000003', '5ecc0000-0000-0000-0000-00000000a11c', 'employer', 'github_oauth', 'contributor'),
  ('5ecc0000-0000-0000-0000-000000000004', '5ecc0000-0000-0000-0000-00000000a11c', 'employer', 'github_oauth', 'contributor'),
  ('5ecc0000-0000-0000-0000-000000000005', '5ecc0000-0000-0000-0000-00000000a11c', 'employer', 'github_oauth', 'contributor'),
  ('5ecc0000-0000-0000-0000-000000000006', '5ecc0000-0000-0000-0000-00000000a11c', 'employer', 'github_oauth', 'contributor'),
  ('5ecc0000-0000-0000-0000-000000000007', '5ecc0000-0000-0000-0000-00000000a11c', 'employer', 'github_oauth', 'contributor'),
  -- L: Alice is an admin here, so `configurable_by_me` is not what refuses.
  ('5ecc0000-0000-0000-0000-000000000008', '5ecc0000-0000-0000-0000-00000000a11c', 'employer', 'github_oauth', 'admin');

-- Subscriptions. `period_end` is a DATE and the window is HALF-OPEN, so a
-- current period ends in the future here; the boundary itself has its own
-- assertions in repository_sharing_view.sql.
insert into dojo.billing_accounts (tenant_id, status, period_start, period_end) values
  ('5ecc0000-0000-0000-0000-000000000001', 'active',   current_date - 10, current_date + 20),
  ('5ecc0000-0000-0000-0000-000000000003', 'active',   current_date - 10, current_date + 20),
  ('5ecc0000-0000-0000-0000-000000000004', 'active',   current_date - 10, current_date + 20),
  ('5ecc0000-0000-0000-0000-000000000005', 'past_due', current_date - 10, current_date + 20),
  ('5ecc0000-0000-0000-0000-000000000007', 'active',   current_date - 10, current_date + 20),
  -- L: a live subscription, so `unclaimed` is reached on its own merits and
  -- not because billing happened to refuse first.
  ('5ecc0000-0000-0000-0000-000000000008', 'active', current_date - 10, current_date + 20);
-- tenants 2 and 6 deliberately have NO row: absence is the case H2/A2 exist for.

insert into dojo.tenant_share_policy (tenant_id, private_repos_shared) values
  ('5ecc0000-0000-0000-0000-000000000003', true),
  ('5ecc0000-0000-0000-0000-000000000004', false),
  ('5ecc0000-0000-0000-0000-000000000005', true),
  ('5ecc0000-0000-0000-0000-000000000006', true),
  -- L: the org mandates sharing, so the election is not what refuses.
  ('5ecc0000-0000-0000-0000-000000000008', true);

insert into dojo.repositories (id, tenant_id, repo_key, name, provider, visibility, visibility_captured_at) values
  -- A  personal · private · subscribed · user elected
  ('5ecc0000-0000-0000-0000-000000000010', '5ecc0000-0000-0000-0000-000000000001',
   'github.com/ztest-alice-sub/a', 'a', 'github', 'private', now()),
  -- A2 personal · private · NOT subscribed · user elected
  ('5ecc0000-0000-0000-0000-000000000011', '5ecc0000-0000-0000-0000-000000000002',
   'github.com/ztest-alice-free/a2', 'a2', 'github', 'private', now()),
  -- B  personal · private · subscribed · NOT elected
  ('5ecc0000-0000-0000-0000-000000000012', '5ecc0000-0000-0000-0000-000000000001',
   'github.com/ztest-alice-sub/b', 'b', 'github', 'private', now()),
  -- C  personal · public · NOT elected
  ('5ecc0000-0000-0000-0000-000000000013', '5ecc0000-0000-0000-0000-000000000002',
   'github.com/ztest-alice-free/c', 'c', 'github', 'public', now()),
  -- D  org · public · user elected
  ('5ecc0000-0000-0000-0000-000000000014', '5ecc0000-0000-0000-0000-000000000003',
   'github.com/ztest-on/d', 'd', 'github', 'public', now()),
  -- E  org · public · policy on · NOT elected by the user
  ('5ecc0000-0000-0000-0000-000000000015', '5ecc0000-0000-0000-0000-000000000003',
   'github.com/ztest-on/e', 'e', 'github', 'public', now()),
  -- F  org · private · subscribed · policy on · no user election
  ('5ecc0000-0000-0000-0000-000000000016', '5ecc0000-0000-0000-0000-000000000003',
   'github.com/ztest-on/f', 'f', 'github', 'private', now()),
  -- G  org · private · subscribed · policy OFF · user elected anyway
  ('5ecc0000-0000-0000-0000-000000000017', '5ecc0000-0000-0000-0000-000000000004',
   'github.com/ztest-off/g', 'g', 'github', 'private', now()),
  -- H  org · private · billing row exists and is past_due · policy on
  ('5ecc0000-0000-0000-0000-000000000018', '5ecc0000-0000-0000-0000-000000000005',
   'github.com/ztest-pastdue/h', 'h', 'github', 'private', now()),
  -- H2 org · private · NO billing row · policy on
  ('5ecc0000-0000-0000-0000-000000000019', '5ecc0000-0000-0000-0000-000000000006',
   'github.com/ztest-nobill/h2', 'h2', 'github', 'private', now()),
  -- I  org · visibility NEVER CAPTURED · subscribed · policy on
  ('5ecc0000-0000-0000-0000-00000000001a', '5ecc0000-0000-0000-0000-000000000003',
   'github.com/ztest-on/i', 'i', 'github', null, null),
  -- J  org · private · subscribed · policy OFF but this repo mandated
  ('5ecc0000-0000-0000-0000-00000000001b', '5ecc0000-0000-0000-0000-000000000004',
   'github.com/ztest-off/j', 'j', 'github', 'private', now()),
  -- K  org · private · subscribed · policy on · this member has no seat
  ('5ecc0000-0000-0000-0000-00000000001c', '5ecc0000-0000-0000-0000-000000000003',
   'github.com/ztest-on/k', 'k', 'github', 'private', now()),
  -- N  org · private · subscribed · NO tenant_share_policy row at all. Not in
  --    §8b, and it is the commonest state of every live tenant — the same
  --    absence-vs-value defect H2 exists for, one table over. An org that has
  --    not decided has not mandated, so `elected` must be FALSE, never NULL:
  --    §8b's own sketch coalesces the per-repo election to the policy and stops,
  --    which leaves a three-valued verdict for a consumer that has to decide.
  ('5ecc0000-0000-0000-0000-00000000001d', '5ecc0000-0000-0000-0000-000000000007',
   'github.com/ztest-nopolicy/n', 'n', 'github', 'private', now()),
  -- L  org · private · subscribed · mandated · but the tenant is UNCLAIMED
  ('5ecc0000-0000-0000-0000-00000000001e', '5ecc0000-0000-0000-0000-000000000008',
   'github.com/ztest-unowned/l', 'l', 'github', 'private', now());

insert into dojo.repository_elections (tenant_id, repository_id, authority, principal_id, elected) values
  -- L: the ORG elected it (mandate). Everything allows except the claim.
  ('5ecc0000-0000-0000-0000-000000000008', '5ecc0000-0000-0000-0000-00000000001e', 'organization', null, true),
  -- A, A2, D: the user elected, and the user holds authority.
  ('5ecc0000-0000-0000-0000-000000000001', '5ecc0000-0000-0000-0000-000000000010', 'user', '5ecc0000-0000-0000-0000-00000000a11c', true),
  ('5ecc0000-0000-0000-0000-000000000002', '5ecc0000-0000-0000-0000-000000000011', 'user', '5ecc0000-0000-0000-0000-00000000a11c', true),
  ('5ecc0000-0000-0000-0000-000000000003', '5ecc0000-0000-0000-0000-000000000014', 'user', '5ecc0000-0000-0000-0000-00000000a11c', true),
  -- G: the user elected and does NOT hold authority. A mandate cuts both ways —
  -- an individual may not publish the company's private code on their own say-so.
  ('5ecc0000-0000-0000-0000-000000000004', '5ecc0000-0000-0000-0000-000000000017', 'user', '5ecc0000-0000-0000-0000-00000000a11c', true),
  -- I: an election the user made before the forge answered. It must NOT apply —
  -- there is no authority to apply it under.
  ('5ecc0000-0000-0000-0000-000000000003', '5ecc0000-0000-0000-0000-00000000001a', 'user', '5ecc0000-0000-0000-0000-00000000a11c', true),
  -- J: the per-repo exception a tenant-wide flag cannot express.
  ('5ecc0000-0000-0000-0000-000000000004', '5ecc0000-0000-0000-0000-00000000001b', 'organization', null, true);

-- ---------------------------------------------------------------------------
-- The table of §8b, asserted row by row.
-- ---------------------------------------------------------------------------
do $$
declare
    expected record;
    got      record;
begin
    for expected in
        select *
          from (values
            --  §8b   repo_key                          authority   may_share elected sync   refused_by     reason_code
            ('A',  'github.com/ztest-alice-sub/a',   'user',         true,  true,  true,  null,          null),
            ('A2', 'github.com/ztest-alice-free/a2', 'user',         false, true,  false, 'entitlement', 'not_subscribed'),
            ('B',  'github.com/ztest-alice-sub/b',   'user',         true,  false, false, 'election',    'not_elected_user'),
            ('C',  'github.com/ztest-alice-free/c',  'user',         true,  false, false, 'election',    'not_elected_user'),
            ('D',  'github.com/ztest-on/d',          'user',         true,  true,  true,  null,          null),
            ('E',  'github.com/ztest-on/e',          'user',         true,  false, false, 'election',    'not_elected_user'),
            ('F',  'github.com/ztest-on/f',          'organization', true,  true,  true,  null,          null),
            ('G',  'github.com/ztest-off/g',         'organization', true,  false, false, 'election',    'not_elected_org'),
            ('H',  'github.com/ztest-pastdue/h',     'organization', false, true,  false, 'entitlement', 'not_subscribed'),
            ('H2', 'github.com/ztest-nobill/h2',     'organization', false, true,  false, 'entitlement', 'not_subscribed'),
            ('I',  'github.com/ztest-on/i',          null,           false, false, false, 'entitlement', 'forge_visibility_unknown'),
            ('J',  'github.com/ztest-off/j',         'organization', true,  true,  true,  null,          null),
            -- L: subscribed, mandated, captured — refused ONLY because no forge
            -- owner has ever claimed the tenant. An unclaimed org cannot hold a
            -- subscription, so this is tested ABOVE the billing terms; were it
            -- tested below, the answer would be `not_subscribed` and would tell
            -- an admin to buy what the service will not sell them yet.
            ('L',  'github.com/ztest-unowned/l',     'organization', false, true,  false, 'entitlement', 'unclaimed'),
            -- K is PHASE 1. §8b's stated verdict is
            --     may_share ❌ · entitlement · `no_seat`
            -- and it is NOT REACHABLE: `dojo.seat_allocations` does not exist, so
            -- the `no_seat` term sits COMMENTED at its precedence position in the
            -- view rather than pretended. Phase 1's honest answer is that a
            -- subscribed, mandated repository shares — asserted here as what the
            -- SQL actually does, with the schema absence asserted below so this
            -- goes red the day the table arrives and the term is uncommented.
            ('K',  'github.com/ztest-on/k',          'organization', true,  true,  true,  null,          null),
            ('N',  'github.com/ztest-nopolicy/n',    'organization', true,  false, false, 'election',    'not_elected_org')
          ) as t(scenario, repo_key, authority, may_share, elected, sync_enabled, refused_by, reason_code)
    loop
        select v.authority::text  as authority
             , v.may_share
             , v.elected
             , v.sync_enabled
             , v.refused_by
             , v.reason_code
          into got
          from dojo.all_my_repositories v
         where v.repo_key     = expected.repo_key
           and v.principal_id = '5ecc0000-0000-0000-0000-00000000a11c';

        if not found then
            raise exception 'scenario %: no row in all_my_repositories for %',
                expected.scenario, expected.repo_key;
        end if;

        if got.authority    is distinct from expected.authority
           or got.may_share    is distinct from expected.may_share
           or got.elected      is distinct from expected.elected
           or got.sync_enabled is distinct from expected.sync_enabled
           or got.refused_by   is distinct from expected.refused_by
           or got.reason_code  is distinct from expected.reason_code
        then
            raise exception
                'scenario % (%): expected (authority=%, may_share=%, elected=%, sync_enabled=%, refused_by=%, reason_code=%) but got (authority=%, may_share=%, elected=%, sync_enabled=%, refused_by=%, reason_code=%)',
                expected.scenario, expected.repo_key,
                coalesce(expected.authority, 'NULL'), expected.may_share, expected.elected,
                expected.sync_enabled, coalesce(expected.refused_by, 'NULL'),
                coalesce(expected.reason_code, 'NULL'),
                coalesce(got.authority, 'NULL'), got.may_share, got.elected,
                got.sync_enabled, coalesce(got.refused_by, 'NULL'),
                coalesce(got.reason_code, 'NULL');
        end if;
    end loop;
end $$;

do $$
declare
    n int;
begin
    -- K's guard. `no_seat` is unreachable until this table exists; when it does,
    -- the commented term in the entitlement CASE must be uncommented and K's
    -- expectation above changed to §8b's stated verdict.
    select count(*) into n
      from information_schema.tables
     where table_schema = 'dojo' and table_name = 'seat_allocations';
    if n <> 0 then
        raise exception
            'dojo.seat_allocations now exists — uncomment the `no_seat` term in all_my_repositories and restore scenario K to (may_share=false, entitlement, no_seat).';
    end if;

    -- Every scenario resolved to exactly one row for Alice: the view is one row
    -- per (repository, member), and a LATERAL that fanned out would multiply it.
    select count(*) into n
      from dojo.all_my_repositories
     where principal_id = '5ecc0000-0000-0000-0000-00000000a11c'
       and repo_key like 'github.com/ztest-%';
    if n <> 15 then
        raise exception 'expected 15 fixture rows for Alice (one per repository), got %.', n;
    end if;
end $$;

rollback;
