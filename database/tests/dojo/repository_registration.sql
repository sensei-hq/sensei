-- What `registerRepositories` writes, against the real schema.
--
-- `dojo.repositories` had ZERO readers and ZERO writers in the whole app until
-- this slice (spec §VIII.1) — nothing had ever inserted a row, so nothing had
-- ever checked that the columns the code targets exist or that its NOT NULLs are
-- satisfied. The unit tests run against a fake that enforces only the uniques it
-- was told about.
--
-- WHAT BREAKS THIS TEST: a repositories column the registration path does not
-- supply becoming NOT NULL, the (tenant_id, repo_key) unique disappearing (which
-- would make re-registration duplicate instead of converge), or tenant_id
-- becoming nullable (which would silently permit the unmapped-repo row that
-- §II.6 forbids).

begin;

insert into dojo.principals (id, auth_user_id) values
  ('aaaaaaaa-1111-1111-1111-111111111111', '11111111-1111-1111-1111-111111111111');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('cccccccc-3333-3333-3333-333333333333', 'organization/ztest-acme', 'organization', 'acme',
   'acme', 'dojo.sensei-hq.org/organization/ztest-acme');

insert into dojo.tenant_connections
  (tenant_id, provider, external_id, external_slug, connected_by, verified_at)
values ('cccccccc-3333-3333-3333-333333333333', 'github', '11', 'acme',
        'aaaaaaaa-1111-1111-1111-111111111111', now());

-- ── the insert registerRepositories issues ──────────────────────────────────
insert into dojo.repositories (tenant_id, repo_key, remote_url, name, provider)
values ('cccccccc-3333-3333-3333-333333333333', 'github.com/acme/api',
        'git@github.com:acme/api.git', 'api', 'github');

do $$
declare
    got record;
begin
    select tenant_id, repo_key, name, visibility into got
      from dojo.repositories where repo_key = 'github.com/acme/api';
    if got is null then
        raise exception 'the registration insert produced no repositories row.';
    end if;
    -- Phase 1 does not set visibility; the column default must therefore be the
    -- safe one. Defaulting to `public` would mean every registered repo read as
    -- free-to-sync the moment the phase-2 gate starts consulting it.
    if got.visibility <> 'private' then
        raise exception
            'dojo.repositories.visibility defaults to % — phase 1 does not set it, so the default must be private.',
            got.visibility;
    end if;
end $$;

-- ── (tenant_id, repo_key) is what makes re-registration converge ────────────
do $$
begin
    begin
        insert into dojo.repositories (tenant_id, repo_key, name, provider)
        values ('cccccccc-3333-3333-3333-333333333333', 'github.com/acme/api', 'api again', 'github');
        raise exception
            'dojo.repositories (tenant_id, repo_key) is not unique — re-registering would duplicate.';
    exception when unique_violation then
        null;  -- expected
    end;
end $$;

-- ── an unmapped repo has nowhere to live, by design ─────────────────────────
-- §II.6: a repo whose remote matches no connection is unmapped, NOT personal.
-- `tenant_id NOT NULL` is what makes that structural rather than a convention
-- the code has to remember.
do $$
begin
    begin
        insert into dojo.repositories (tenant_id, repo_key, name, provider)
        values (null, 'git.internal.acme.com/acme/api', 'api', 'github');
        raise exception
            'dojo.repositories.tenant_id is nullable — an unmapped repo could be stored with no tenant.';
    exception when not_null_violation then
        null;  -- expected
    end;
end $$;

-- ── provider is REQUIRED, so the registration path must derive and store it ──
-- It is derived from repo_key's host once, at registration. Storing it means the
-- host→provider mapping lives in exactly one place instead of being re-derived
-- in every view and query that needs the forge.
do $$
begin
    begin
        insert into dojo.repositories (tenant_id, repo_key, name)
        values ('cccccccc-3333-3333-3333-333333333333', 'github.com/acme/noprovider', 'x');
        raise exception 'dojo.repositories.provider is nullable — the forge could go unrecorded.';
    exception when not_null_violation then
        null;  -- expected
    end;
end $$;

-- ── the same repo under two tenants is legitimate ───────────────────────────
-- A consultant genuinely has one repository under two clients, and a fork under
-- a personal account alongside the org original. The unique is per TENANT, not
-- global, precisely so that stays expressible.
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('dddddddd-4444-4444-4444-444444444444', 'personal/ztest-alice', 'personal', 'ztest-alice',
   'Alice''s Dōjō', 'dojo.sensei-hq.org/personal/ztest-alice');

insert into dojo.repositories (tenant_id, repo_key, name, provider)
values ('dddddddd-4444-4444-4444-444444444444', 'github.com/acme/api', 'api', 'github');

do $$
declare n int;
begin
    select count(*) into n from dojo.repositories where repo_key = 'github.com/acme/api';
    if n <> 2 then
        raise exception 'one repo_key should be storable under two tenants, found % row(s).', n;
    end if;
end $$;

rollback;
