-- Writing metrics and metric activations as the signed-in person (#236 step 3;
-- access matrix "Metrics").
--
-- dojo.upsert_repository_metrics files a batch of repo-scoped metric rows for
-- the caller's daemon. Every row must name a repository the caller may sync
-- (their own verdict row says sync_enabled); the tenant is taken from that row,
-- never from the input; one row that fails refuses the WHOLE batch
-- (insufficient_privilege), because the caller already filtered and a mismatch
-- means the batch was not what this side resolved. Upserts on the seven-column
-- key; within a batch, the later of two rows for one key wins.
--
-- dojo.set_metric_activation switches a catalogue metric off (or on) for one
-- repository — the tenant's cost decision, so only where the caller holds it:
-- configurable_by_me AND (a personal tenant OR a tenant admin). Off is a row;
-- on is its absence.
--
-- Neither table takes a session write.
--
-- WHAT BREAKS THIS TEST: an ingest gate that ignores sync_enabled or the
-- caller; a tenant taken from the input; a partial write of a refused batch; an
-- upsert that duplicates; an activation gate that lets a contributor of an
-- organisation decide; any session write grant on either table.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000131', 'a1000000-0000-4000-8000-000000000131', 'ada'),
  ('a0000000-0000-4000-8000-000000000132', 'a1000000-0000-4000-8000-000000000132', 'ola');

-- T: organisation, subscribed, shares its private repos. P: ola's personal dōjō. U: not ola's.
insert into dojo.tenants (id, key, origin, slug, name, dojo_url, claimed_at) values
  ('b0000000-0000-4000-8000-00000000013a', 'organization/ztest-mw-t', 'organization', 'ztest-mw-t', 'T', 'x', now()),
  ('b0000000-0000-4000-8000-00000000013b', 'personal/ztest-mw-ola',   'personal',     'ztest-mw-ola', 'P', 'x', null),
  ('b0000000-0000-4000-8000-00000000013c', 'organization/ztest-mw-u', 'organization', 'ztest-mw-u', 'U', 'x', now());
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000013a', 'a0000000-0000-4000-8000-000000000131', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000013a', 'a0000000-0000-4000-8000-000000000132', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000013b', 'a0000000-0000-4000-8000-000000000132', 'admin',       'personal', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000013c', 'a0000000-0000-4000-8000-000000000131', 'admin',       'employer', 'github_oauth');
insert into dojo.billing_accounts (tenant_id, plan, status, period_start, period_end) values
  ('b0000000-0000-4000-8000-00000000013a', 'team', 'active', current_date - 1, current_date + 30);
insert into dojo.tenant_share_policy (tenant_id, private_repos_shared) values ('b0000000-0000-4000-8000-00000000013a', true);

-- 1 T private (syncs for ola via the org policy) · 2 T public (ada elected it, ola has not) ·
-- 3 P public (ola's own) · 4 U public (not ola's) · 5 P, visibility never captured
insert into dojo.repositories (id, tenant_id, repo_key, name, visibility, visibility_captured_at, provider) values
  ('91000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-00000000013a', 'github.com/ztest-mw/priv', 'priv', 'private', now(), 'github'),
  ('91000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-00000000013a', 'github.com/ztest-mw/pub',  'pub',  'public',  now(), 'github'),
  ('91000000-0000-4000-8000-000000000003', 'b0000000-0000-4000-8000-00000000013b', 'github.com/ztest-ola/own', 'own',  'public',  now(), 'github'),
  ('91000000-0000-4000-8000-000000000004', 'b0000000-0000-4000-8000-00000000013c', 'github.com/ztest-u/x',     'x',    'public',  now(), 'github'),
  ('91000000-0000-4000-8000-000000000005', 'b0000000-0000-4000-8000-00000000013b', 'github.com/ztest-ola/unk', 'unk',  null,      null,  'github');

-- ada has elected T's public repository: HER verdict syncs it, ola's does not.
insert into dojo.repository_elections (tenant_id, repository_id, authority, principal_id, elected) values
  ('b0000000-0000-4000-8000-00000000013a', '91000000-0000-4000-8000-000000000002', 'user',
   'a0000000-0000-4000-8000-000000000131', true);

create temp table m as select id, key from sensei.metrics order by key limit 1;
grant select on m to authenticated;

set local role authenticated;

do $$
declare
    mid uuid := (select id from m);
    mkey text := (select key from m);
    n int;
    r record;
    row1 jsonb;
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000132"}', true);
    row1 := jsonb_build_object('repository_id', '91000000-0000-4000-8000-000000000001', 'metric_id', mid,
                               'commit_sha', null, 'computed_on', current_date, 'grain', 'daily', 'value', 1);

    -- ── ingest ───────────────────────────────────────────────────────────────
    n := dojo.upsert_repository_metrics(jsonb_build_array(row1));
    if n <> 1 then raise exception 'ingest: ola''s permitted row should be written, wrote %', n; end if;
    n := dojo.upsert_repository_metrics(jsonb_build_array(row1 || '{"value": 2}', row1 || '{"value": 3}'));
    if n <> 1 then raise exception 'ingest: two rows for one key in a batch should merge, wrote %', n; end if;

    begin
        perform dojo.upsert_repository_metrics(jsonb_build_array(
            row1 || '{"value": 9}',
            row1 || '{"repository_id": "91000000-0000-4000-8000-000000000002"}'));
        raise exception 'ingest: a row for a repository ola may not sync was accepted (another member''s verdict is not hers)';
    exception when insufficient_privilege then null;
    end;
    begin
        perform dojo.upsert_repository_metrics(jsonb_build_array(
            row1 || '{"repository_id": "91000000-0000-4000-8000-000000000004"}'));
        raise exception 'ingest: a row for another tenant''s repository was accepted';
    exception when insufficient_privilege then null;
    end;
    begin
        perform dojo.upsert_repository_metrics(jsonb_build_array(
            row1 || '{"tenant_id": "b0000000-0000-4000-8000-00000000013c", "commit_sha": "abc"}'));
    exception when others then
        raise exception 'ingest: an extra tenant_id in the input must be ignored, not fail: %', sqlerrm;
    end;
    begin
        insert into dojo.repository_metrics (tenant_id, repository_id, metric_id, scope, computed_on, value)
        values ('b0000000-0000-4000-8000-00000000013a', '91000000-0000-4000-8000-000000000001', mid, 'repo', current_date - 10, 5);
        raise exception 'repository_metrics: a session wrote the table directly';
    exception when insufficient_privilege then null;
    end;

    -- ── activation ───────────────────────────────────────────────────────────
    begin
        perform dojo.set_metric_activation('github.com/ztest-mw/priv', mkey, false);
        raise exception 'activation: a contributor made the organisation''s cost decision';
    exception when insufficient_privilege then null;
    end;
    -- A public repository is configurable by anyone; the cost decision is still the admin's.
    begin
        perform dojo.set_metric_activation('github.com/ztest-mw/pub', mkey, false);
        raise exception 'activation: a contributor switched a metric on the organisation''s public repo';
    exception when insufficient_privilege then null;
    end;
    -- Her own tenant, but no authority yet (visibility never captured).
    begin
        perform dojo.set_metric_activation('github.com/ztest-ola/unk', mkey, false);
        raise exception 'activation: a repository nobody may configure yet was switched';
    exception when insufficient_privilege then null;
    end;
    select * into r from dojo.set_metric_activation('github.com/ztest-ola/own', mkey, false);
    if r.enabled is distinct from false then raise exception 'activation: ola''s own repo should switch off, got %', r; end if;
    begin
        perform dojo.set_metric_activation('github.com/ztest-ola/own', 'ztest-no-such-metric', false);
        raise exception 'activation: an unknown metric was switched';
    exception when no_data_found then null;
    end;
    begin
        perform dojo.set_metric_activation('github.com/ztest-u/x', mkey, false);
        raise exception 'activation: a repository not ola''s was switched';
    exception when no_data_found then null;
    end;
    begin
        insert into dojo.metric_activations (tenant_id, repository_id, metric_id, enabled)
        values ('b0000000-0000-4000-8000-00000000013b', '91000000-0000-4000-8000-000000000005', mid, false);
        raise exception 'metric_activations: a session wrote the table directly';
    exception when insufficient_privilege then null;
    end;

    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000131"}', true);
    perform dojo.set_metric_activation('github.com/ztest-mw/priv', mkey, false);
    perform dojo.set_metric_activation('github.com/ztest-mw/priv', mkey, false);
    select * into r from dojo.set_metric_activation('github.com/ztest-mw/pub', mkey, false);
    select * into r from dojo.set_metric_activation('github.com/ztest-mw/pub', mkey, true);
    if r.enabled is distinct from true then raise exception 'activation: the admin should switch it back on, got %', r; end if;
end $$;

reset role;
do $$
declare
    got text;
begin
    select string_agg(r.repo_key || '=' || rm.value || coalesce('@' || rm.commit_sha, ''), ', ' order by r.repo_key, rm.commit_sha nulls first)
      into got
      from dojo.repository_metrics rm join dojo.repositories r on r.id = rm.repository_id
     where r.repo_key like 'github.com/ztest-%';
    if got is distinct from 'github.com/ztest-mw/priv=3, github.com/ztest-mw/priv=1@abc' then
        raise exception 'ingest stored: %', got;
    end if;
    if (select rm.tenant_id from dojo.repository_metrics rm join dojo.repositories r on r.id = rm.repository_id
         where r.repo_key = 'github.com/ztest-mw/priv' and rm.commit_sha = 'abc')
       <> 'b0000000-0000-4000-8000-00000000013a' then
        raise exception 'ingest: the tenant must come from the caller''s repository, not the input';
    end if;
    select string_agg(r.repo_key, ', ' order by r.repo_key) into got
      from dojo.metric_activations a join dojo.repositories r on r.id = a.repository_id
     where r.repo_key like 'github.com/ztest-%';
    if got is distinct from 'github.com/ztest-mw/priv, github.com/ztest-ola/own' then
        raise exception 'activations stored: %', got;
    end if;
end $$;

rollback;
