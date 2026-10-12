-- Knowledge as the signed-in person (#236 step 3; docs/spec/dojo/access-matrix.md
-- "Knowledge — artifacts, triage, rules, policies").
--
-- | data                              | contributor | maintainer | admin |
-- |-----------------------------------|-------------|------------|-------|
-- | artifacts, published              | R           | R          | R     |
-- | artifacts, own submissions        | R           | R          | R     |
-- | artifacts, others' (pre-triage)   | —           | R          | R     |
-- | triage queue, decisions           | —           | R/W        | R/W   |
-- | policies                          | R           | R          | W     |
--
-- All of it within the person's own tenant. A decision is the deciding
-- maintainer's own, by hand, about an artifact of the tenant it is filed in.
--
-- WHAT BREAKS THIS TEST: an artifact policy without the published / own /
-- maintainer clauses, or without is_member; a triage or decision policy at
-- contributor; a policies write below admin; a decision whose maintainer_id is
-- not the caller, that claims to be automated, or that names another tenant's
-- artifact or triage row; an UPDATE grant on a triage row's identity columns.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000071', 'a1000000-0000-4000-8000-000000000071', 'carl'),
  ('a0000000-0000-4000-8000-000000000072', 'a1000000-0000-4000-8000-000000000072', 'mia'),
  ('a0000000-0000-4000-8000-000000000073', 'a1000000-0000-4000-8000-000000000073', 'ada'),
  ('a0000000-0000-4000-8000-000000000074', 'a1000000-0000-4000-8000-000000000074', 'uma');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000007a', 'organization/ztest-know-t', 'organization',
   'ztest-know-t', 'knowledge fixture T', 'dojo.sensei-hq.org/organization/ztest-know-t'),
  ('b0000000-0000-4000-8000-00000000007b', 'organization/ztest-know-u', 'organization',
   'ztest-know-u', 'knowledge fixture U', 'dojo.sensei-hq.org/organization/ztest-know-u');

insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via) values
  ('b0000000-0000-4000-8000-00000000007a', 'a0000000-0000-4000-8000-000000000071', 'contributor', 'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000007a', 'a0000000-0000-4000-8000-000000000072', 'maintainer',  'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000007a', 'a0000000-0000-4000-8000-000000000073', 'admin',       'employer', 'github_oauth'),
  ('b0000000-0000-4000-8000-00000000007b', 'a0000000-0000-4000-8000-000000000074', 'admin',       'employer', 'github_oauth');

-- 1 published in T · 2 carl's submission in T · 3 mia's submission in T · 4 published in U
insert into dojo.artifacts (id, tenant_id, kind, status, title, body, signature, contributed_by) values
  ('f7000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-00000000007a', 'principle', 'published',
   'one', 'x', 'ztest-sig-1', 'a0000000-0000-4000-8000-000000000072'),
  ('f7000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-00000000007a', 'principle', 'submitted',
   'two', 'x', 'ztest-sig-2', 'a0000000-0000-4000-8000-000000000071'),
  ('f7000000-0000-4000-8000-000000000003', 'b0000000-0000-4000-8000-00000000007a', 'principle', 'submitted',
   'three', 'x', 'ztest-sig-3', 'a0000000-0000-4000-8000-000000000072'),
  ('f7000000-0000-4000-8000-000000000004', 'b0000000-0000-4000-8000-00000000007b', 'principle', 'published',
   'four', 'x', 'ztest-sig-4', 'a0000000-0000-4000-8000-000000000074');

insert into dojo.triage_queue (id, tenant_id, artifact_id, signature) values
  ('e7000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-00000000007a',
   'f7000000-0000-4000-8000-000000000002', 'ztest-sig-2'),
  ('e7000000-0000-4000-8000-000000000004', 'b0000000-0000-4000-8000-00000000007b',
   'f7000000-0000-4000-8000-000000000004', 'ztest-sig-4');

insert into dojo.policies (id, tenant_id, scope_key) values
  ('d7000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-00000000007a', 'ztest-all-org'),
  ('d7000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-00000000007b', 'ztest-all-org');

set local role authenticated;

-- ── reads ────────────────────────────────────────────────────────────────────
create temp table expect (who text, sub text, artifacts text, triage text, policies text) on commit drop;
insert into expect values
  ('carl', 'a1000000-0000-4000-8000-000000000071', '1,2',   '',  '1'),
  ('mia',  'a1000000-0000-4000-8000-000000000072', '1,2,3', '2', '1'),
  ('ada',  'a1000000-0000-4000-8000-000000000073', '1,2,3', '2', '1'),
  ('uma',  'a1000000-0000-4000-8000-000000000074', '4',     '4', '2');

do $$
declare
    r record;
    got text;
begin
    for r in select * from expect loop
        perform set_config('request.jwt.claims', json_build_object('sub', r.sub)::text, true);
        select coalesce(string_agg(right(id::text, 1), ',' order by id), '') into got
          from dojo.artifacts where signature like 'ztest-sig-%';
        if got is distinct from r.artifacts then
            raise exception 'artifacts RLS: % should see [%], saw [%]', r.who, r.artifacts, got;
        end if;
        select coalesce(string_agg(right(id::text, 1), ',' order by id), '') into got
          from dojo.triage_queue where signature like 'ztest-sig-%';
        if got is distinct from r.triage then
            raise exception 'triage RLS: % should see [%], saw [%]', r.who, r.triage, got;
        end if;
        select coalesce(string_agg(right(id::text, 1), ',' order by id), '') into got
          from dojo.policies where scope_key like 'ztest-%';
        if got is distinct from r.policies then
            raise exception 'policies RLS: % should see [%], saw [%]', r.who, r.policies, got;
        end if;
    end loop;
end $$;

-- ── writes ───────────────────────────────────────────────────────────────────
do $$
declare
    n int;
    t constant uuid := 'b0000000-0000-4000-8000-00000000007a';
    mia constant uuid := 'a0000000-0000-4000-8000-000000000072';
    ada constant uuid := 'a0000000-0000-4000-8000-000000000073';
    a2 constant uuid := 'f7000000-0000-4000-8000-000000000002';
    a4 constant uuid := 'f7000000-0000-4000-8000-000000000004';
    q2 constant uuid := 'e7000000-0000-4000-8000-000000000002';
    q4 constant uuid := 'e7000000-0000-4000-8000-000000000004';
begin
    -- triage: carl (contributor) cannot decide; mia (maintainer) decides T's row only.
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000071"}', true);
    update dojo.triage_queue set state = 'resolved', updated_at = now() where id = q2;
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'triage: a contributor moved a triage row'; end if;
    begin
        insert into dojo.decisions (tenant_id, artifact_id, triage_id, maintainer_id, status, automated)
        -- No triage row: carl sees his own artifact, so only the role refuses this.
        values (t, a2, null, 'a0000000-0000-4000-8000-000000000071', 'approve', false);
        raise exception 'decisions: a contributor recorded a decision';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'contributor decision refused for the wrong reason: %', sqlerrm; end if;
    end;

    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000072"}', true);
    insert into dojo.decisions (tenant_id, artifact_id, triage_id, maintainer_id, status, automated)
    values (t, a2, q2, mia, 'approve', false);
    update dojo.triage_queue set state = 'resolved', updated_at = now() where id = q2;
    get diagnostics n = row_count;
    if n <> 1 then raise exception 'triage: the maintainer should move T''s row, changed %', n; end if;
    update dojo.triage_queue set state = 'resolved', updated_at = now() where id = q4;
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'triage: a maintainer of T moved U''s row'; end if;
    begin
        update dojo.triage_queue set signature = 'forged' where id = q2;
        raise exception 'triage: a row''s signature is not the decider''s to change';
    exception when insufficient_privilege then
        if sqlerrm like '%row-level security%' then raise exception 'triage: signature must be refused by the column grant'; end if;
    end;

    begin
        insert into dojo.decisions (tenant_id, artifact_id, triage_id, maintainer_id, status, automated)
        values (t, a2, q2, ada, 'approve', false);
        raise exception 'decisions: mia recorded a decision in ada''s name';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'forged decider refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        insert into dojo.decisions (tenant_id, artifact_id, triage_id, maintainer_id, status, automated)
        values (t, a2, q2, mia, 'approve', true);
        raise exception 'decisions: a person''s decision must not claim to be automated';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'automated refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        insert into dojo.decisions (tenant_id, artifact_id, triage_id, maintainer_id, status, automated)
        values (t, a4, null, mia, 'approve', false);
        raise exception 'decisions: a decision in T named U''s artifact';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'cross-tenant artifact refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        insert into dojo.decisions (tenant_id, artifact_id, triage_id, maintainer_id, status, automated)
        values (t, a2, q4, mia, 'approve', false);
        raise exception 'decisions: a decision in T named U''s triage row';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'cross-tenant triage refused for the wrong reason: %', sqlerrm; end if;
    end;

    select count(*) into n from dojo.decisions where artifact_id = a2;
    if n <> 1 then raise exception 'decisions: the maintainer should read T''s trail, saw %', n; end if;

    -- policies: a maintainer reads but does not write; the admin does. (Changing
    -- U's policy is refused by the read policy already; T's is the write policy's.)
    update dojo.policies set retention_days = 7 where id = 'd7000000-0000-4000-8000-000000000001';
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'policies: a maintainer changed a policy'; end if;
    delete from dojo.policies where id = 'd7000000-0000-4000-8000-000000000001';
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'policies: a maintainer deleted a policy'; end if;
    begin
        insert into dojo.policies (tenant_id, scope_key, updated_at) values (t, 'ztest-mia', now());
        raise exception 'policies: a maintainer wrote a policy';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'maintainer policy refused for the wrong reason: %', sqlerrm; end if;
    end;

    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000071"}', true);
    select count(*) into n from dojo.decisions;
    if n <> 0 then raise exception 'decisions: a contributor read the trail (% row)', n; end if;

    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000073"}', true);
    -- The route's upsert, as PostgREST sends it.
    insert into dojo.policies (tenant_id, scope_key, retention_days, updated_at)
    values (t, 'ztest-all-org', 30, now())
    on conflict (tenant_id, scope_key) do update
       set tenant_id = excluded.tenant_id, scope_key = excluded.scope_key,
           retention_days = excluded.retention_days, updated_at = excluded.updated_at;
    if (select retention_days from dojo.policies where id = 'd7000000-0000-4000-8000-000000000001') <> 30 then
        raise exception 'policies: the admin''s upsert did not update T''s policy';
    end if;
    update dojo.policies set retention_days = 7 where id = 'd7000000-0000-4000-8000-000000000002';
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'policies: an admin of T changed U''s policy'; end if;
    delete from dojo.policies where id = 'd7000000-0000-4000-8000-000000000002';
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'policies: an admin of T deleted U''s policy'; end if;
    delete from dojo.policies where id = 'd7000000-0000-4000-8000-000000000001';
    get diagnostics n = row_count;
    if n <> 1 then raise exception 'policies: the admin should delete T''s policy, deleted %', n; end if;
end $$;

rollback;
