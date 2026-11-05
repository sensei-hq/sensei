-- WP8: a personal tenant's private repository is entitled, free (D46).
--
-- WHAT BREAKS THIS TEST (today): the view keys entitlement on visibility, so a
-- personal private repository with no billing row refuses `not_subscribed`.
-- WP8's ENTITLEMENT change (`when mr.origin = 'personal' then null` after the
-- capture term) entitles it. Red until then.
--
-- The capture cases below are green before and after: they pin the personal
-- ALLOW's POSITION — below the capture term, so a stale or unknown capture
-- refuses whoever owns the tenant. A personal ALLOW placed above the capture
-- term turns them red.
--
-- Run through --apply (the view DDL is passed in, not \ir'd): the file seeds,
-- asserts, rolls back. Nothing persists.
begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a1111111-1111-1111-1111-111111111111', 'aa111111-1111-1111-1111-111111111111', 'Alice'),
  ('e1111111-1111-1111-1111-111111111111', 'ee111111-1111-1111-1111-111111111111', 'Bob');

-- Personal tenants are never claimed: they ARE the person (claimed_at NULL).
insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b1111111-1111-1111-1111-111111111111', 'personal/ztest-wp8-alice', 'personal', 'ztest-wp8-alice',
   'Alice''s Dōjō', 'dojo.sensei-hq.org/personal/ztest-wp8-alice', null),
  ('f1111111-1111-1111-1111-111111111111', 'personal/ztest-wp8-bob', 'personal', 'ztest-wp8-bob',
   'Bob''s Dōjō', 'dojo.sensei-hq.org/personal/ztest-wp8-bob', null);

insert into dojo.memberships (tenant_id, user_id, kind, authenticated_via, role, disabled_at) values
  ('b1111111-1111-1111-1111-111111111111', 'a1111111-1111-1111-1111-111111111111',
   'personal', 'github_oauth', 'admin', null),
  ('f1111111-1111-1111-1111-111111111111', 'e1111111-1111-1111-1111-111111111111',
   'personal', 'github_oauth', 'admin', null);

-- Bob's tenant HAS a billing row, and it is past due. A personal dōjō is
-- free (D46): the row's state must not refuse his private repository.
insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('f1111111-1111-1111-1111-111111111111', 'team', 'past_due', current_date - 40, current_date - 1);

insert into dojo.repositories (tenant_id, repo_key, remote_url, name, provider, visibility, visibility_captured_at) values
  ('b1111111-1111-1111-1111-111111111111', 'github.com/ztest-wp8-alice/notes',
   'git@github.com:ztest-wp8-alice/notes.git', 'notes', 'github', 'private', now()),
  -- Captured 31 days ago: the TTL (30 days) has lapsed.
  ('b1111111-1111-1111-1111-111111111111', 'github.com/ztest-wp8-alice/stale',
   'git@github.com:ztest-wp8-alice/stale.git', 'stale', 'github', 'private', now() - interval '31 days'),
  -- Never captured at all.
  ('b1111111-1111-1111-1111-111111111111', 'github.com/ztest-wp8-alice/unknown',
   'git@github.com:ztest-wp8-alice/unknown.git', 'unknown', 'github', null, null),
  -- Bob's: past-due billing row above, still entitled.
  ('f1111111-1111-1111-1111-111111111111', 'github.com/ztest-wp8-bob/notes',
   'git@github.com:ztest-wp8-bob/notes.git', 'notes', 'github', 'private', now());

-- No billing row: a personal tenant carries none, and must not need one.

do $$
declare
    r record;
begin
    select may_share, entitlement_refusal, reason_code, refused_by, elected
      into r
      from dojo.all_my_repositories
     where principal_id = 'a1111111-1111-1111-1111-111111111111'
       and repo_key = 'github.com/ztest-wp8-alice/notes';
    if r is null then
        raise exception 'personal private row missing entirely.';
    end if;
    if r.may_share is not true then
        raise exception 'personal private repo not entitled: may_share = %.', r.may_share;
    end if;
    if r.entitlement_refusal is not null then
        raise exception 'personal private repo refused: %.', r.entitlement_refusal;
    end if;
    if r.reason_code is not null then
        raise exception 'personal private repo carries reason: %.', r.reason_code;
    end if;
    if r.refused_by is not null then
        raise exception 'personal private repo refused by: %.', r.refused_by;
    end if;
    if r.elected is not true then
        raise exception 'personal repo with no decision is not elected.';
    end if;
end $$;

-- A personal billing row in a bad state refuses nothing: the dōjō is free.
do $$
declare
    r record;
begin
    select may_share, entitlement_refusal, reason_code, refused_by
      into r
      from dojo.all_my_repositories
     where principal_id = 'e1111111-1111-1111-1111-111111111111'
       and repo_key = 'github.com/ztest-wp8-bob/notes';
    if r is null then
        raise exception 'past-due personal row missing entirely.';
    end if;
    if r.may_share is not true then
        raise exception 'past-due personal private repo not entitled: %.',
            coalesce(r.entitlement_refusal, 'REFUSED-SILENTLY');
    end if;
    if r.entitlement_refusal is not null
       or r.reason_code is not null
       or r.refused_by is not null then
        raise exception 'past-due personal private repo carries a refusal: %.',
            coalesce(r.entitlement_refusal, r.reason_code, 'UNKNOWN');
    end if;
end $$;

-- Capture before personal-ALLOW: a stale or unknown capture refuses, with no
-- authority and no sync, on a personal tenant like any other.
do $$
declare
    expected record;
    got      record;
begin
    for expected in
        select * from (values
            ('github.com/ztest-wp8-alice/stale',   'forge_visibility_stale'),
            ('github.com/ztest-wp8-alice/unknown', 'forge_visibility_unknown')
        ) as t(repo_key, refusal)
    loop
        select v.entitlement_refusal, v.may_share, v.authority::text as authority, v.sync_enabled
          into got
          from dojo.all_my_repositories v
         where v.principal_id = 'a1111111-1111-1111-1111-111111111111'
           and v.repo_key = expected.repo_key;
        if not found then
            raise exception 'no row for %.', expected.repo_key;
        end if;
        if got.entitlement_refusal is distinct from expected.refusal
           or got.may_share is not false
           or got.authority is not null
           or got.sync_enabled is not false then
            raise exception '%: expected (refusal=%, may=false, authority=NULL, sync=false), got (refusal=%, may=%, authority=%, sync=%).',
                expected.repo_key, expected.refusal,
                coalesce(got.entitlement_refusal, 'ENTITLED'), got.may_share,
                coalesce(got.authority, 'NULL'), got.sync_enabled;
        end if;
    end loop;
end $$;

rollback;
