-- elected/configured_by/configured_at follow the org-authority branch of the
-- view's CASE, and configurable_by_me is true for the admin caller unless
-- visibility is null (org-repositories r2, test 5; L2: null-visibility reads
-- false even for admins, and flipping one 409s — the switch reads the wire).
--
-- WHAT BREAKS THIS TEST: a missing dojo.org_repositories(); an org row not
-- honored either way; the policy fallback misordered (explicit row must beat
-- policy, policy must beat the default); the private/public defaults swapped;
-- NULL visibility electing anything; the provenance pointing at the wrong
-- source (policy_set_at vs elected_at) or naming a decider where nobody
-- decided.
--
-- Phase A: red — dojo.org_repositories(uuid) does not exist yet.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000151', 'a1000000-0000-4000-8000-000000000151', 'ada'),
  ('a0000000-0000-4000-8000-000000000152', 'a1000000-0000-4000-8000-000000000152', 'ola');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-000000000151', 'organization/ztest-orrepos-elec', 'organization',
   'ztest-orrepos-elec', 'election fixture', 'dojo.sensei-hq.org/organization/ztest-orrepos-elec', now()),
  ('b0000000-0000-4000-8000-000000000152', 'organization/ztest-orrepos-pol', 'organization',
   'ztest-orrepos-pol', 'policy fixture', 'dojo.sensei-hq.org/organization/ztest-orrepos-pol', now()),
  ('b0000000-0000-4000-8000-000000000153', 'organization/ztest-orrepos-nopol', 'organization',
   'ztest-orrepos-nopol', 'nopolicy fixture', 'dojo.sensei-hq.org/organization/ztest-orrepos-nopol', now());

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-000000000151', 'a0000000-0000-4000-8000-000000000151', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-000000000151', 'a0000000-0000-4000-8000-000000000152', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-000000000152', 'a0000000-0000-4000-8000-000000000151', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-000000000152', 'a0000000-0000-4000-8000-000000000152', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-000000000153', 'a0000000-0000-4000-8000-000000000151', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-000000000153', 'a0000000-0000-4000-8000-000000000152', 'contributor', 'employer', 'github_oauth');

insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b0000000-0000-4000-8000-000000000151', 'pro', 'active', current_date - 1, current_date + 30),
  ('b0000000-0000-4000-8000-000000000152', 'pro', 'active', current_date - 1, current_date + 30),
  ('b0000000-0000-4000-8000-000000000153', 'pro', 'active', current_date - 1, current_date + 30);

-- Policy true on the policy tenant; false on main (a private default that an
-- explicit row must beat); no row at all on nopol.
insert into dojo.tenant_share_policy (tenant_id, private_repos_shared, set_at) values
  ('b0000000-0000-4000-8000-000000000151', false, '2026-09-01T10:00:00Z'),
  ('b0000000-0000-4000-8000-000000000152', true,  '2026-09-02T10:00:00Z');

insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider) values
  ('90000000-0000-4000-8000-000000000151', 'b0000000-0000-4000-8000-000000000151',
   'github.com/ztest-orrepos-elec/orgtrue', 'orgtrue', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000152', 'b0000000-0000-4000-8000-000000000151',
   'github.com/ztest-orrepos-elec/orgfalse', 'orgfalse', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000153', 'b0000000-0000-4000-8000-000000000151',
   'github.com/ztest-orrepos-elec/puboff', 'puboff', 'public', now(), 'github'),
  ('90000000-0000-4000-8000-000000000154', 'b0000000-0000-4000-8000-000000000151',
   'github.com/ztest-orrepos-elec/pubdef', 'pubdef', 'public', now(), 'github'),
  ('90000000-0000-4000-8000-000000000155', 'b0000000-0000-4000-8000-000000000151',
   'github.com/ztest-orrepos-elec/nulvis', 'nulvis', null, null, 'github'),
  ('90000000-0000-4000-8000-000000000156', 'b0000000-0000-4000-8000-000000000152',
   'github.com/ztest-orrepos-pol/poltrue', 'poltrue', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000157', 'b0000000-0000-4000-8000-000000000153',
   'github.com/ztest-orrepos-nopol/defpriv', 'defpriv', 'private', now(), 'github');

-- Standing roles: unlisted repos assert nothing (test 3's rule).
insert into dojo.repository_roles (repository_id, principal_id, forge_role)
select r, 'a0000000-0000-4000-8000-000000000152', 'contributor'
  from (values ('90000000-0000-4000-8000-000000000151'::uuid),
               ('90000000-0000-4000-8000-000000000152'::uuid),
               ('90000000-0000-4000-8000-000000000153'::uuid),
               ('90000000-0000-4000-8000-000000000154'::uuid),
               ('90000000-0000-4000-8000-000000000155'::uuid),
               ('90000000-0000-4000-8000-000000000156'::uuid),
               ('90000000-0000-4000-8000-000000000157')) v(r);

-- Explicit org rows beat the tenant policy either way.
insert into dojo.repository_elections (tenant_id, repository_id, authority, elected, elected_at) values
  ('b0000000-0000-4000-8000-000000000151', '90000000-0000-4000-8000-000000000151',
   'organization', true,  '2026-10-10T10:00:00Z'),
  ('b0000000-0000-4000-8000-000000000151', '90000000-0000-4000-8000-000000000152',
   'organization', false, '2026-10-11T10:00:00Z'),
  ('b0000000-0000-4000-8000-000000000151', '90000000-0000-4000-8000-000000000153',
   'organization', false, '2026-10-12T10:00:00Z'),
  ('b0000000-0000-4000-8000-000000000151', '90000000-0000-4000-8000-000000000155',
   'organization', true,  '2026-10-13T10:00:00Z');

set local role authenticated;

do $$
declare
    r record;
begin
    perform set_config('request.jwt.claims',
        json_build_object('sub', 'a1000000-0000-4000-8000-000000000151')::text, true);

    -- Explicit org row true: elected, by the organization, at the row's stamp.
    select elected, configured_by::text as who, configured_at as at, configurable_by_me as cfg
      into r from dojo.org_repositories('b0000000-0000-4000-8000-000000000151') where name = 'orgtrue';
    if (r.elected, r.by, r.at, r.cfg) is distinct from
       (true, 'organization', '2026-10-10T10:00:00Z'::timestamptz, true) then
        raise exception 'election(orgtrue): expected (t,organization,2026-10-10,t), got (%,%,%,%)',
            r.elected, r.by, r.at, r.cfg;
    end if;

    -- Explicit org row false beats policy-false alike: not elected, still decided.
    select elected, configured_by::text as who, configured_at as at, configurable_by_me as cfg
      into r from dojo.org_repositories('b0000000-0000-4000-8000-000000000151') where name = 'orgfalse';
    if (r.elected, r.by, r.at, r.cfg) is distinct from
       (false, 'organization', '2026-10-11T10:00:00Z'::timestamptz, true) then
        raise exception 'election(orgfalse): expected (f,organization,2026-10-11,t), got (%,%,%,%)',
            r.elected, r.by, r.at, r.cfg;
    end if;

    -- Public with an explicit false: honored (the default only fills absence).
    select elected, configured_by::text as who
      into r from dojo.org_repositories('b0000000-0000-4000-8000-000000000151') where name = 'puboff';
    if (r.elected, r.by) is distinct from (false, 'organization') then
        raise exception 'election(puboff): explicit false on public must hold, got (%,%)', r.elected, r.by;
    end if;

    -- Public with no row: elected by default, decided by nobody (a private
    -- policy never configures a public repo, so provenance stays null).
    select elected, configured_by as who, configured_at as at
      into r from dojo.org_repositories('b0000000-0000-4000-8000-000000000151') where name = 'pubdef';
    if r.elected is distinct from true or r.by is not null or r.at is not null then
        raise exception 'election(pubdef): expected (t,null,null), got (%,%,%)', r.elected, r.by, r.at;
    end if;

    -- NULL visibility: elected false AND not configurable, even with an org
    -- row saying true and even for the admin caller (L2).
    select elected, configurable_by_me as cfg
      into r from dojo.org_repositories('b0000000-0000-4000-8000-000000000151') where name = 'nulvis';
    if (r.elected, r.cfg) is distinct from (false, false) then
        raise exception 'election(nulvis): null visibility must read (f,f), got (%,%)', r.elected, r.cfg;
    end if;

    -- No org row + policy true: the policy elects, provenance is the policy stamp.
    select elected, configured_by::text as who, configured_at as at
      into r from dojo.org_repositories('b0000000-0000-4000-8000-000000000152') where name = 'poltrue';
    if (r.elected, r.by, r.at) is distinct from
       (true, 'organization', '2026-09-02T10:00:00Z'::timestamptz) then
        raise exception 'election(poltrue): expected (t,organization,2026-09-02), got (%,%,%)',
            r.elected, r.by, r.at;
    end if;

    -- No row + no policy on private: not elected, decided by nobody.
    select elected, configured_by as who, configured_at as at
      into r from dojo.org_repositories('b0000000-0000-4000-8000-000000000153') where name = 'defpriv';
    if r.elected is distinct from false or r.by is not null or r.at is not null then
        raise exception 'election(defpriv): expected (f,null,null), got (%,%,%)', r.elected, r.by, r.at;
    end if;
end $$;

rollback;
