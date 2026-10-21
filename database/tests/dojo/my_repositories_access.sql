-- A person's repositories and sync verdicts, as them (#236 step 3; access matrix
-- "Workspaces and repositories"; entity model §3).
--
-- dojo.all_my_repositories computes each (repository, member) verdict from facts
-- a plain member may not read — the tenant's billing account, its share policy,
-- its claim. Read through a session it would see fewer facts and answer wrong
-- ("not subscribed" to every non-admin). So a session reads it through
-- dojo.my_repositories(), which sees every fact and returns only the CALLER's
-- rows; the view itself is not granted to a session.
--
-- Metric activations (what the tenant switched off) are read by its members, as
-- the sync plan needs.
--
-- WHAT BREAKS THIS TEST: my_repositories() without the principal pin; running
-- it as the invoker (the contributor's verdict turns "not subscribed"); a SELECT
-- grant on the view to authenticated; an activations read wider than members.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000101', 'a1000000-0000-4000-8000-000000000101', 'ada'),
  ('a0000000-0000-4000-8000-000000000102', 'a1000000-0000-4000-8000-000000000102', 'ola'),
  ('a0000000-0000-4000-8000-000000000103', 'a1000000-0000-4000-8000-000000000103', 'uma');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-00000000010a', 'organization/ztest-repo-t', 'organization',
   'ztest-repo-t', 'repos fixture T', 'dojo.sensei-hq.org/organization/ztest-repo-t', now()),
  ('b0000000-0000-4000-8000-00000000010b', 'organization/ztest-repo-u', 'organization',
   'ztest-repo-u', 'repos fixture U', 'dojo.sensei-hq.org/organization/ztest-repo-u', now());

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000010a', 'a0000000-0000-4000-8000-000000000101', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000010a', 'a0000000-0000-4000-8000-000000000102', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000010b', 'a0000000-0000-4000-8000-000000000103', 'contributor', 'employer', 'github_oauth');

-- T subscribes and shares its private repositories; the repository is private, captured now.
insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b0000000-0000-4000-8000-00000000010a', 'team', 'active', current_date - 1, current_date + 30);
insert into dojo.tenant_share_policy (tenant_id, private_repos_shared) values
  ('b0000000-0000-4000-8000-00000000010a', true);
insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider) values
  ('90000000-0000-4000-8000-000000000101', 'b0000000-0000-4000-8000-00000000010a',
   'github.com/ztest-repo-t/one', 'one', 'private', now(), 'github'),
  ('90000000-0000-4000-8000-000000000102', 'b0000000-0000-4000-8000-00000000010b',
   'github.com/ztest-repo-u/two', 'two', 'public', now(), 'github');

insert into dojo.metric_activations (tenant_id, repository_id, metric_id, enabled)
select 'b0000000-0000-4000-8000-00000000010a', '90000000-0000-4000-8000-000000000101', m.id, false
  from (select id from sensei.metrics order by key limit 1) m;
insert into dojo.metric_activations (tenant_id, repository_id, metric_id, enabled)
select 'b0000000-0000-4000-8000-00000000010b', '90000000-0000-4000-8000-000000000102', m.id, false
  from (select id from sensei.metrics order by key limit 1) m;

set local role authenticated;

do $$
declare
    n int;
    got text;
    ok boolean;
begin
    -- ── ola (contributor of T) ───────────────────────────────────────────────
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000102"}', true);
    select coalesce(string_agg(repo_key, ',' order by repo_key), '') into got
      from dojo.my_repositories() where repo_key like 'github.com/ztest-repo-%';
    if got <> 'github.com/ztest-repo-t/one' then
        raise exception 'my_repositories: ola should get T''s repository only, got [%]', got;
    end if;
    select sync_enabled into ok from dojo.my_repositories() where repo_key = 'github.com/ztest-repo-t/one';
    if ok is distinct from true then
        raise exception 'my_repositories: a contributor''s verdict must see T''s subscription and policy (sync_enabled = %)', ok;
    end if;
    select count(distinct principal_id) into n from dojo.my_repositories();
    if n <> 1 then raise exception 'my_repositories: returned % principals'' rows, not only the caller''s', n; end if;

    begin
        select count(*) into n from dojo.all_my_repositories;
        raise exception 'all_my_repositories: a session read the view directly';
    exception when insufficient_privilege then
        -- Refused AT THE VIEW, not one table deeper: a refusal on a base table
        -- holds only until a session is granted that table.
        if sqlerrm not like '%view all_my_repositories%' then
            raise exception 'all_my_repositories: refused for the wrong reason: %', sqlerrm;
        end if;
    end;

    select count(*) into n from dojo.metric_activations
     where repository_id in ('90000000-0000-4000-8000-000000000101', '90000000-0000-4000-8000-000000000102');
    if n <> 1 then raise exception 'metric_activations: ola should read T''s one row, read %', n; end if;

    -- ── uma (contributor of U) ───────────────────────────────────────────────
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000103"}', true);
    select coalesce(string_agg(repo_key, ',' order by repo_key), '') into got
      from dojo.my_repositories() where repo_key like 'github.com/ztest-repo-%';
    if got <> 'github.com/ztest-repo-u/two' then
        raise exception 'my_repositories: uma should get U''s repository only, got [%]', got;
    end if;
end $$;

rollback;
