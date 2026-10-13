-- A person's daemon files their run's project, as them (#236 step 4,
-- docs/plan/2026-10-12-dojo-device-plane-as-person.md).
--
-- The relay session POST upserts the run's project on (user_id, slug): the owner
-- is the caller, and the project is personal (no tenant) or homed in a tenant
-- they belong to. Nobody files a project as someone else, or into a tenant they
-- are not a member of.
--
-- WHAT BREAKS THIS TEST: an owner check that accepts another principal; a
-- tenant check that accepts a tenant the caller is not in (or a disabled
-- membership); an update with-check that lets a project change owner; a missing
-- grant on a column the route writes. The plain hand-over is also refused by the
-- READ policy (ola cannot see a bob-owned row); the admin case is what isolates
-- the update's with-check.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000161', 'a1000000-0000-4000-8000-000000000161', 'ola'),
  ('a0000000-0000-4000-8000-000000000162', 'a1000000-0000-4000-8000-000000000162', 'bob');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000016a', 'organization/ztest-pw-t', 'organization', 'ztest-pw-t', 'T', 'x'),
  ('b0000000-0000-4000-8000-00000000016b', 'organization/ztest-pw-u', 'organization', 'ztest-pw-u', 'U', 'x'),
  ('b0000000-0000-4000-8000-00000000016c', 'organization/ztest-pw-v', 'organization', 'ztest-pw-v', 'V', 'x'),
  ('b0000000-0000-4000-8000-00000000016d', 'organization/ztest-pw-w', 'organization', 'ztest-pw-w', 'W', 'x');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via, disabled_at) values
  ('b0000000-0000-4000-8000-00000000016a', 'a0000000-0000-4000-8000-000000000161', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000016c', 'a0000000-0000-4000-8000-000000000161', 'contributor', 'employer', 'github_oauth', now()),
  ('b0000000-0000-4000-8000-00000000016d', 'a0000000-0000-4000-8000-000000000161', 'admin', 'employer', 'github_oauth', null);

set local role authenticated;

do $$
declare
    ola constant uuid := 'a0000000-0000-4000-8000-000000000161';
    bob constant uuid := 'a0000000-0000-4000-8000-000000000162';
    t   constant uuid := 'b0000000-0000-4000-8000-00000000016a';
    u   constant uuid := 'b0000000-0000-4000-8000-00000000016b';
    v   constant uuid := 'b0000000-0000-4000-8000-00000000016c';
    w   constant uuid := 'b0000000-0000-4000-8000-00000000016d';
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000161"}', true);

    -- The route's upsert, as PostgREST sends it: a personal project, then the same
    -- slug re-filed under ola's tenant (the project's binding moved).
    insert into dojo.projects (user_id, tenant_id, slug, name, classification, last_run_at, updated_at, constitution)
    values (ola, null, 'ztest-pw-api', 'api', 'personal', now(), now(), '{"rules":[]}')
    on conflict (user_id, slug) do update set user_id = excluded.user_id, tenant_id = excluded.tenant_id,
        slug = excluded.slug, name = excluded.name, classification = excluded.classification,
        last_run_at = excluded.last_run_at, updated_at = excluded.updated_at, constitution = excluded.constitution;
    insert into dojo.projects (user_id, tenant_id, slug, name, classification, last_run_at, updated_at)
    values (ola, t, 'ztest-pw-api', 'api', 'company', now(), now())
    on conflict (user_id, slug) do update set user_id = excluded.user_id, tenant_id = excluded.tenant_id,
        slug = excluded.slug, name = excluded.name, classification = excluded.classification,
        last_run_at = excluded.last_run_at, updated_at = excluded.updated_at;
    if (select tenant_id from dojo.projects where user_id = ola and slug = 'ztest-pw-api') is distinct from t then
        raise exception 'projects: ola''s re-filed project should be homed in T';
    end if;

    begin
        insert into dojo.projects (user_id, tenant_id, slug, name, classification)
        values (bob, null, 'ztest-pw-forged', 'x', 'personal');
        raise exception 'projects: ola filed a project as bob';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'foreign owner refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        insert into dojo.projects (user_id, tenant_id, slug, name, classification)
        values (ola, u, 'ztest-pw-elsewhere', 'x', 'company');
        raise exception 'projects: ola filed a project into a tenant she is not in';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'foreign tenant refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        insert into dojo.projects (user_id, tenant_id, slug, name, classification)
        values (ola, v, 'ztest-pw-disabled', 'x', 'company');
        raise exception 'projects: ola filed a project into a tenant whose membership is disabled';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'disabled tenant refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        update dojo.projects set tenant_id = u where user_id = ola and slug = 'ztest-pw-api';
        raise exception 'projects: ola re-homed her project into a tenant she is not in';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 're-home refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        update dojo.projects set user_id = bob where user_id = ola and slug = 'ztest-pw-api';
        raise exception 'projects: ola handed her project to bob';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'hand-over refused for the wrong reason: %', sqlerrm; end if;
    end;
    -- As W's admin, ola may READ a bob-owned project homed in W — so only the
    -- update's own with-check stands between her and handing one over.
    insert into dojo.projects (user_id, tenant_id, slug, name, classification)
    values (ola, w, 'ztest-pw-admin', 'x', 'company');
    begin
        update dojo.projects set user_id = bob where user_id = ola and slug = 'ztest-pw-admin';
        raise exception 'projects: ola, an admin, handed her project to bob';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'admin hand-over refused for the wrong reason: %', sqlerrm; end if;
    end;
end $$;

rollback;
