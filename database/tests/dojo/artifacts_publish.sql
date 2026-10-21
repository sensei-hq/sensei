-- A person's daemon contributes an artifact, as them (#236 step 4,
-- docs/spec/dojo/access-matrix.md).
--
-- POST artifacts inserts one submission: into a tenant the caller is an active
-- member of, attributed to the caller, awaiting triage. Publishing, approving and
-- the adoption counters are the system's (promoteCluster) and the maintainers'.
--
-- WHAT BREAKS THIS TEST: an insert policy without is_member (a tenant the caller
-- is not in, or a disabled membership); one that accepts another contributor or
-- no contributor; a grant on `status`/`approved_by`/the counters, which would let
-- a contributor publish their own submission past triage; a missing grant on a
-- column the route writes.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000171', 'a1000000-0000-4000-8000-000000000171', 'ola'),
  ('a0000000-0000-4000-8000-000000000172', 'a1000000-0000-4000-8000-000000000172', 'bob');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000017a', 'organization/ztest-ap-t', 'organization', 'ztest-ap-t', 'T', 'x'),
  ('b0000000-0000-4000-8000-00000000017b', 'organization/ztest-ap-u', 'organization', 'ztest-ap-u', 'U', 'x'),
  ('b0000000-0000-4000-8000-00000000017c', 'organization/ztest-ap-v', 'organization', 'ztest-ap-v', 'V', 'x');
insert into dojo.memberships (tenant_id, user_id, role, kind, authenticated_via, disabled_at) values
  ('b0000000-0000-4000-8000-00000000017a', 'a0000000-0000-4000-8000-000000000171', 'contributor', 'employer', 'github_oauth', null),
  ('b0000000-0000-4000-8000-00000000017c', 'a0000000-0000-4000-8000-000000000171', 'contributor', 'employer', 'github_oauth', now());

set local role authenticated;

do $$
declare
    got record;
    ola constant uuid := 'a0000000-0000-4000-8000-000000000171';
    bob constant uuid := 'a0000000-0000-4000-8000-000000000172';
    t   constant uuid := 'b0000000-0000-4000-8000-00000000017a';
    u   constant uuid := 'b0000000-0000-4000-8000-00000000017b';
    v   constant uuid := 'b0000000-0000-4000-8000-00000000017c';
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000171"}', true);

    -- The route's insert, as PostgREST sends it, returning id + seq.
    insert into dojo.artifacts (tenant_id, engagement_id, kind, title, body, payload, scope, signature,
                                attribution, contributed_by)
    values (t, null, 'principle', 'Small changes', 'b', '{"kind":"principle"}', '{}', 'sig-1', '{}', ola)
    returning id, seq, status::text as status into got;
    if got.id is null or got.seq is null or got.status <> 'submitted' then
        raise exception 'artifacts: ola''s contribution should come back submitted with an id and seq, got %', got;
    end if;

    begin
        insert into dojo.artifacts (tenant_id, kind, title, body, signature, contributed_by)
        values (u, 'principle', 'x', 'b', 'sig-u', ola);
        raise exception 'artifacts: ola contributed to a tenant she is not in';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'foreign tenant refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        insert into dojo.artifacts (tenant_id, kind, title, body, signature, contributed_by)
        values (v, 'principle', 'x', 'b', 'sig-v', ola);
        raise exception 'artifacts: ola contributed through a disabled membership';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'disabled tenant refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        insert into dojo.artifacts (tenant_id, kind, title, body, signature, contributed_by)
        values (t, 'principle', 'x', 'b', 'sig-bob', bob);
        raise exception 'artifacts: ola contributed as bob';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'forged contributor refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        insert into dojo.artifacts (tenant_id, kind, title, body, signature, contributed_by)
        values (t, 'principle', 'x', 'b', 'sig-anon', null);
        raise exception 'artifacts: ola contributed anonymously';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'anonymous refused for the wrong reason: %', sqlerrm; end if;
    end;
    -- Past triage is not hers to go.
    begin
        insert into dojo.artifacts (tenant_id, kind, status, title, body, signature, contributed_by)
        values (t, 'principle', 'published', 'x', 'b', 'sig-pub', ola);
        raise exception 'artifacts: ola published her own contribution past triage';
    exception when insufficient_privilege then
        if sqlerrm not like 'permission denied for table artifacts' then raise exception 'self-publish refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        update dojo.artifacts set status = 'published' where contributed_by = ola;
        raise exception 'artifacts: ola promoted her own contribution';
    exception when insufficient_privilege then
        if sqlerrm not like 'permission denied for table artifacts' then raise exception 'self-promote refused for the wrong reason: %', sqlerrm; end if;
    end;
end $$;

rollback;
