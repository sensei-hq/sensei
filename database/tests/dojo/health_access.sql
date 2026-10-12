-- The tenant's live relay sessions, as a COUNT for its admin (#236 step 3;
-- docs/spec/dojo/access-matrix.md "Relay": sessions are own-only content, and an
-- admin gets "count only (health)").
--
-- dojo.tenant_live_session_count answers a tenant admin with how many of the
-- tenant's sessions have heartbeated since a moment — a number, never a row —
-- and refuses anyone else, loudly: a refusal that returned 0 would read as "no
-- live sessions".
--
-- WHAT BREAKS THIS TEST: a gate below admin; returning 0 instead of refusing; a
-- count that ignores the heartbeat window, a disabled membership's sessions, or
-- the tenant.

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-0000000000d1', 'a1000000-0000-4000-8000-0000000000d1', 'ada'),
  ('a0000000-0000-4000-8000-0000000000d2', 'a1000000-0000-4000-8000-0000000000d2', 'lea'),
  ('a0000000-0000-4000-8000-0000000000d3', 'a1000000-0000-4000-8000-0000000000d3', 'ola'),
  ('a0000000-0000-4000-8000-0000000000d4', 'a1000000-0000-4000-8000-0000000000d4', 'uma'),
  ('a0000000-0000-4000-8000-0000000000d5', 'a1000000-0000-4000-8000-0000000000d5', 'dis');

insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-0000000000da', 'organization/ztest-hl-t', 'organization',
   'ztest-hl-t', 'health fixture T', 'dojo.sensei-hq.org/organization/ztest-hl-t'),
  ('b0000000-0000-4000-8000-0000000000db', 'organization/ztest-hl-u', 'organization',
   'ztest-hl-u', 'health fixture U', 'dojo.sensei-hq.org/organization/ztest-hl-u');

insert into dojo.memberships (id, tenant_id, user_id, role, kind, authenticated_via, disabled_at) values
  ('dd000000-0000-4000-8000-000000000001', 'b0000000-0000-4000-8000-0000000000da', 'a0000000-0000-4000-8000-0000000000d1', 'admin',       'employer', 'github_oauth', null),
  ('dd000000-0000-4000-8000-000000000002', 'b0000000-0000-4000-8000-0000000000da', 'a0000000-0000-4000-8000-0000000000d2', 'lead',        'employer', 'github_oauth', null),
  ('dd000000-0000-4000-8000-000000000003', 'b0000000-0000-4000-8000-0000000000da', 'a0000000-0000-4000-8000-0000000000d3', 'contributor', 'employer', 'github_oauth', null),
  ('dd000000-0000-4000-8000-000000000004', 'b0000000-0000-4000-8000-0000000000db', 'a0000000-0000-4000-8000-0000000000d4', 'admin',       'employer', 'github_oauth', null),
  ('dd000000-0000-4000-8000-000000000005', 'b0000000-0000-4000-8000-0000000000da', 'a0000000-0000-4000-8000-0000000000d5', 'contributor', 'employer', 'github_oauth', now());

-- T: ola live · ola stale (an hour ago) · ada live · dis live but DISABLED.  U: uma live.
insert into dojo.relay_sessions (membership_id, run_id, heartbeat_at) values
  ('dd000000-0000-4000-8000-000000000003', 'c3000000-0000-4000-8000-000000000001', now()),
  ('dd000000-0000-4000-8000-000000000003', 'c3000000-0000-4000-8000-000000000002', now() - interval '1 hour'),
  ('dd000000-0000-4000-8000-000000000001', 'c3000000-0000-4000-8000-000000000003', now()),
  ('dd000000-0000-4000-8000-000000000005', 'c3000000-0000-4000-8000-000000000005', now()),
  ('dd000000-0000-4000-8000-000000000004', 'c3000000-0000-4000-8000-000000000004', now());

set local role authenticated;

do $$
declare
    n int;
    t constant uuid := 'b0000000-0000-4000-8000-0000000000da';
    u constant uuid := 'b0000000-0000-4000-8000-0000000000db';
    since constant timestamptz := now() - interval '5 minutes';
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-0000000000d1"}', true);
    n := dojo.tenant_live_session_count(t, since);
    if n <> 2 then raise exception 'health: ada should count T''s 2 live sessions, got %', n; end if;
    begin
        n := dojo.tenant_live_session_count(u, since);
        raise exception 'health: an admin of T counted U''s sessions (%)', n;
    exception when insufficient_privilege then null;
    end;

    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-0000000000d2"}', true);
    begin
        n := dojo.tenant_live_session_count(t, since);
        raise exception 'health: a lead counted the tenant''s sessions (%)', n;
    exception when insufficient_privilege then null;
    end;

    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-0000000000d4"}', true);
    n := dojo.tenant_live_session_count(u, since);
    if n <> 1 then raise exception 'health: uma should count U''s 1 live session, got %', n; end if;
end $$;

rollback;
