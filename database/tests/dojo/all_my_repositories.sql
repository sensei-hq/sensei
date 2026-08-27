-- `dojo.all_my_repositories` — one read that answers "which repositories are
-- mine, and whose dōjō does each belong to".
--
-- The daemon is a USER-plane client: a person belongs to several tenants, each
-- tenant owns repositories, and every repository belongs to exactly one tenant.
-- So the natural question is never "what does tenant X hold" but "what is mine",
-- with the owning tenant carried on each row. This view is that question, and it
-- backs both `GET /v1/you/sync/plan` and the console's repository list, so the
-- two can never disagree about what a user has.
--
-- WHAT BREAKS THIS TEST: the view losing its membership filter (which would
-- expose other tenants' repositories), dropping the disabled-membership filter,
-- or `provider` disappearing from dojo.repositories.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('aaaaaaaa-1111-1111-1111-111111111111', '11111111-1111-1111-1111-111111111111', 'Alice'),
  ('bbbbbbbb-2222-2222-2222-222222222222', '22222222-2222-2222-2222-222222222222', 'Bob');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('cccccccc-3333-3333-3333-333333333333', 'organization/ztest-acme', 'organization', 'ztest-acme',
   'Acme', 'dojo.sensei-hq.org/organization/ztest-acme'),
  ('dddddddd-4444-4444-4444-444444444444', 'personal/ztest-alice', 'personal', 'ztest-alice',
   'Alice''s Dōjō', 'dojo.sensei-hq.org/personal/ztest-alice'),
  ('eeeeeeee-5555-5555-5555-555555555555', 'organization/ztest-other', 'organization', 'ztest-other',
   'Other', 'dojo.sensei-hq.org/organization/ztest-other');

-- Alice is in acme + her personal dōjō. Bob is in `other`, which Alice must
-- never see. Alice also has a DISABLED membership in `other` — a former
-- employer — which must not resurrect its repositories for her.
insert into dojo.memberships (tenant_id, user_id, kind, authenticated_via, role, disabled_at) values
  ('cccccccc-3333-3333-3333-333333333333', 'aaaaaaaa-1111-1111-1111-111111111111',
   'employer', 'github_oauth', 'admin', null),
  ('dddddddd-4444-4444-4444-444444444444', 'aaaaaaaa-1111-1111-1111-111111111111',
   'personal', 'github_oauth', 'admin', null),
  ('eeeeeeee-5555-5555-5555-555555555555', 'bbbbbbbb-2222-2222-2222-222222222222',
   'employer', 'github_oauth', 'contributor', null),
  ('eeeeeeee-5555-5555-5555-555555555555', 'aaaaaaaa-1111-1111-1111-111111111111',
   'employer', 'github_oauth', 'contributor', now());

insert into dojo.repositories (tenant_id, repo_key, remote_url, name, provider) values
  ('cccccccc-3333-3333-3333-333333333333', 'github.com/acme/api',
   'git@github.com:acme/api.git', 'api', 'github'),
  ('dddddddd-4444-4444-4444-444444444444', 'github.com/alice/notes',
   'git@github.com:alice/notes.git', 'notes', 'github'),
  ('eeeeeeee-5555-5555-5555-555555555555', 'github.com/other/secret',
   'git@github.com:other/secret.git', 'secret', 'github');

do $$
declare
    mine text;
    n int;
begin
    -- 1. Alice sees exactly her two, each carrying its owning tenant.
    select string_agg(repo_key || '@' || tenant, ', ' order by repo_key) into mine
      from dojo.all_my_repositories
     where principal_id = 'aaaaaaaa-1111-1111-1111-111111111111';
    if mine is distinct from
       'github.com/acme/api@organization/ztest-acme, github.com/alice/notes@personal/ztest-alice' then
        raise exception 'all_my_repositories for Alice returned: %', coalesce(mine, 'NOTHING');
    end if;

    -- 2. Bob's repository is not among them. The view is an allow-list the sync
    --    plan acts on directly, so a leak here is not a display bug — it is the
    --    daemon syncing someone else's code.
    select count(*) into n
      from dojo.all_my_repositories
     where principal_id = 'aaaaaaaa-1111-1111-1111-111111111111'
       and repo_key = 'github.com/other/secret';
    if n <> 0 then
        raise exception 'Alice can see a repository from a tenant she only has a DISABLED membership in.';
    end if;

    -- 3. Bob sees his own.
    select count(*) into n
      from dojo.all_my_repositories
     where principal_id = 'bbbbbbbb-2222-2222-2222-222222222222';
    if n <> 1 then
        raise exception 'Bob should see exactly his own repository, saw %.', n;
    end if;

    -- 4. The columns the UI and the sync plan read.
    select count(*) into n
      from information_schema.columns
     where table_schema = 'dojo' and table_name = 'all_my_repositories'
       and column_name in ('repository_id', 'repo_key', 'name', 'provider', 'remote_url',
                           'tenant', 'tenant_id', 'owning_org', 'origin',
                           'principal_id', 'role', 'sync_enabled', 'denied_reason');
    if n <> 13 then
        raise exception 'all_my_repositories is missing columns; matched % of 13.', n;
    end if;
end $$;

rollback;
