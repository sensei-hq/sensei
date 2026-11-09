-- last_synced_at is the max pushed_at over ALL scopes for the repo: an admin
-- reads user-scoped rows too (org-repositories r2, test 4; the management
-- branch of can_read_repository_metric — no principal predicate, unlike the
-- member read's (scope = 'repo' or principal_id = me)).
--
-- WHAT BREAKS THIS TEST: a missing dojo.org_repositories(); a member-rule
-- predicate leaking into the admin read (a newer user row not moving the
-- max); NULL-vs-value confusion on repos with no rows (D83: never synced
-- reads NULL, distinct from hidden).
--
-- Phase A: red — dojo.org_repositories(uuid) does not exist yet.

begin;

insert into sensei.metrics
       (key, name, description, family, type, direction, purpose, how_to_read, formula, task_name) values
  ('ztest-orrepos-sync', 'ztest sync', 'ztest', 'quality', 'count', 'neutral', 'ztest', 'ztest', 'ztest', 'ztest');

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000141', 'a1000000-0000-4000-8000-000000000141', 'ada'),
  ('a0000000-0000-4000-8000-000000000142', 'a1000000-0000-4000-8000-000000000142', 'ola');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-000000000141', 'organization/ztest-orrepos-sync', 'organization',
   'ztest-orrepos-sync', 'sync fixture', 'dojo.sensei-hq.org/organization/ztest-orrepos-sync', now());

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-000000000141', 'a0000000-0000-4000-8000-000000000141', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-000000000141', 'a0000000-0000-4000-8000-000000000142', 'contributor', 'employer', 'github_oauth');

insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b0000000-0000-4000-8000-000000000141', 'pro', 'active', current_date - 1, current_date + 30);

insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider) values
  ('90000000-0000-4000-8000-000000000141', 'b0000000-0000-4000-8000-000000000141',
   'github.com/ztest-orrepos-sync/both', 'both', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000142', 'b0000000-0000-4000-8000-000000000141',
   'github.com/ztest-orrepos-sync/repoonly', 'repoonly', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000143', 'b0000000-0000-4000-8000-000000000141',
   'github.com/ztest-orrepos-sync/never', 'never', 'private', now(), 'github');

-- Every repo needs a standing role or it is unlisted (test 3's rule):
-- counts are not this file's subject, presence is.
insert into dojo.repository_roles (repository_id, principal_id, forge_role) values
  ('90000000-0000-4000-8000-000000000141', 'a0000000-0000-4000-8000-000000000142', 'contributor'),
  ('90000000-0000-4000-8000-000000000142', 'a0000000-0000-4000-8000-000000000142', 'contributor'),
  ('90000000-0000-4000-8000-000000000143', 'a0000000-0000-4000-8000-000000000142', 'contributor');

-- both: a repo row at T1 and a user row (another principal) at T2 > T1.
-- repoonly: a repo row only. never: no rows.
insert into dojo.repository_metrics
       (tenant_id, repository_id, metric_id, scope, principal_id, computed_on, value, pushed_at) values
  ('b0000000-0000-4000-8000-000000000141', '90000000-0000-4000-8000-000000000141',
   (select id from sensei.metrics where key = 'ztest-orrepos-sync'),
   'repo', null, current_date - 2, 7, '2026-10-01T10:00:00Z'),
  ('b0000000-0000-4000-8000-000000000141', '90000000-0000-4000-8000-000000000141',
   (select id from sensei.metrics where key = 'ztest-orrepos-sync'),
   'user', 'a0000000-0000-4000-8000-000000000142', current_date - 1, 3, '2026-10-02T10:00:00Z'),
  ('b0000000-0000-4000-8000-000000000141', '90000000-0000-4000-8000-000000000142',
   (select id from sensei.metrics where key = 'ztest-orrepos-sync'),
   'repo', null, current_date - 3, 5, '2026-10-03T10:00:00Z');

set local role authenticated;

do $$
declare
    got timestamptz;
begin
    perform set_config('request.jwt.claims',
        json_build_object('sub', 'a1000000-0000-4000-8000-000000000141')::text, true);

    -- The user row (another principal's) moves the admin's max: this is the
    -- pin that separates the admin rule from the member rule.
    select last_synced_at into got from dojo.org_repositories('b0000000-0000-4000-8000-000000000141')
     where name = 'both';
    if got is distinct from '2026-10-02T10:00:00Z'::timestamptz then
        raise exception 'last_synced_at(both): a newer user row must move the admin max to 2026-10-02, got %', got;
    end if;

    select last_synced_at into got from dojo.org_repositories('b0000000-0000-4000-8000-000000000141')
     where name = 'repoonly';
    if got is distinct from '2026-10-03T10:00:00Z'::timestamptz then
        raise exception 'last_synced_at(repoonly): expected 2026-10-03, got %', got;
    end if;

    select last_synced_at into got from dojo.org_repositories('b0000000-0000-4000-8000-000000000141')
     where name = 'never';
    if got is not null then
        raise exception 'last_synced_at(never): no rows must read NULL (never synced), got %', got;
    end if;
end $$;

rollback;
