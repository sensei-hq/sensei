-- The daemon's relay writes, as the signed-in person (#236 step 4,
-- docs/plan/2026-10-12-dojo-device-plane-as-person.md).
--
-- A person's daemon presents THEIR session: it publishes their run (a
-- relay_sessions upsert on the (membership, run) key), the run's outline (a
-- relay_segments upsert on (session, seq), then parent links), and raises their
-- agent's gates (relay_inbox, agent → human). Each only on a membership the
-- person owns and a session of that membership.
--
-- The phone and the daemon are the same person on the same session, so nothing
-- here can — or should — tell them apart: raising a gate on your OWN session is
-- your agent's to do. What no one may do is write into another person's run.
--
-- WHAT BREAKS THIS TEST: a session policy without owns_membership, or without a
-- with-check (a run re-pointed at someone else's membership); a segment or gate
-- policy that accepts another person's session, or a session of a different
-- membership than the row names; a missing grant on a column the daemon writes.
--
-- NOT separable here, on purpose: the session update's with-check, and the gate
-- policy's owns_membership. Relay READS are own-only, and Postgres checks an
-- updated row against the SELECT policy and runs a policy's subquery under the
-- caller's RLS — so another person's run is refused either way. Both clauses
-- stay as defence in depth; they become load-bearing the day relay reads widen
-- (the peer-status view, #252).

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000151', 'a1000000-0000-4000-8000-000000000151', 'alice'),
  ('a0000000-0000-4000-8000-000000000152', 'a1000000-0000-4000-8000-000000000152', 'bob');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000015a', 'organization/ztest-relay-d', 'organization',
   'ztest-relay-d', 'relay daemon fixture', 'dojo.sensei-hq.org/organization/ztest-relay-d');
insert into dojo.memberships (id, tenant_id, user_id, role, kind, authenticated_via) values
  ('d1000000-0000-4000-8000-000000000151', 'b0000000-0000-4000-8000-00000000015a',
   'a0000000-0000-4000-8000-000000000151', 'contributor', 'employer', 'github_oauth'),
  ('d1000000-0000-4000-8000-000000000152', 'b0000000-0000-4000-8000-00000000015a',
   'a0000000-0000-4000-8000-000000000152', 'contributor', 'employer', 'github_oauth');
-- bob's run, made by the system, for alice to fail to touch.
insert into dojo.relay_sessions (id, membership_id, run_id) values
  ('c1000000-0000-4000-8000-000000000152', 'd1000000-0000-4000-8000-000000000152',
   'c2000000-0000-4000-8000-000000000152');

set local role authenticated;

do $$
declare
    n int;
    sid uuid;
    seg uuid;
    alice_mem constant uuid := 'd1000000-0000-4000-8000-000000000151';
    bob_mem   constant uuid := 'd1000000-0000-4000-8000-000000000152';
    bob_sess  constant uuid := 'c1000000-0000-4000-8000-000000000152';
    run       constant uuid := 'c2000000-0000-4000-8000-000000000151';
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000151"}', true);

    -- ── the run ──────────────────────────────────────────────────────────────
    -- The route's upsert, as PostgREST sends it (every posted column on conflict).
    insert into dojo.relay_sessions (membership_id, run_id, title, goal, status, progress_done, progress_total,
                                     current_phase, current_feature, last_event_at, paused_until, pause_reason,
                                     heartbeat_at, updated_at)
    values (alice_mem, run, 'Run', 'goal', 'running', 0, 2, 'P1', 'f', now(), null, null, now(), now())
    on conflict (membership_id, run_id) do update set
        membership_id = excluded.membership_id, run_id = excluded.run_id, title = excluded.title,
        goal = excluded.goal, status = excluded.status, progress_done = excluded.progress_done,
        progress_total = excluded.progress_total, current_phase = excluded.current_phase,
        current_feature = excluded.current_feature, last_event_at = excluded.last_event_at,
        paused_until = excluded.paused_until, pause_reason = excluded.pause_reason,
        heartbeat_at = excluded.heartbeat_at, updated_at = excluded.updated_at;
    insert into dojo.relay_sessions (membership_id, run_id, title, status, progress_done, progress_total, updated_at)
    values (alice_mem, run, 'Run', 'done', 2, 2, now())
    on conflict (membership_id, run_id) do update set
        membership_id = excluded.membership_id, run_id = excluded.run_id, title = excluded.title,
        status = excluded.status, progress_done = excluded.progress_done,
        progress_total = excluded.progress_total, updated_at = excluded.updated_at;
    select id into sid from dojo.relay_sessions where membership_id = alice_mem and run_id = run;
    if (select status from dojo.relay_sessions where id = sid) <> 'done' then
        raise exception 'session: alice''s second publish should update her run';
    end if;

    begin
        insert into dojo.relay_sessions (membership_id, run_id, title) values (bob_mem, run, 'forged');
        raise exception 'session: alice published a run under bob''s membership';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'foreign run refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        update dojo.relay_sessions set membership_id = bob_mem where id = sid;
        raise exception 'session: alice re-pointed her run at bob''s membership';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 're-point refused for the wrong reason: %', sqlerrm; end if;
    end;
    update dojo.relay_sessions set status = 'done' where id = bob_sess;
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'session: alice changed bob''s run'; end if;

    -- ── the outline ──────────────────────────────────────────────────────────
    insert into dojo.relay_segments (session_id, seq, title, summary, detail, agent, model, spec_ref, state,
                                     is_gate, gate_severity, updated_at)
    values (sid, 0, 'Phase 1', 's', 'd', 'claude', 'm', 'spec', 'active', false, null, now()),
           (sid, 1, 'Step', null, null, null, null, null, 'pending', true, 'blocking', now())
    on conflict (session_id, seq) do update set
        session_id = excluded.session_id, seq = excluded.seq, title = excluded.title,
        summary = excluded.summary, detail = excluded.detail, agent = excluded.agent,
        model = excluded.model, spec_ref = excluded.spec_ref, state = excluded.state,
        is_gate = excluded.is_gate, gate_severity = excluded.gate_severity, updated_at = excluded.updated_at;
    select id into seg from dojo.relay_segments where session_id = sid and seq = 0;
    update dojo.relay_segments set parent_id = seg where session_id = sid and seq = 1;
    get diagnostics n = row_count;
    if n <> 1 then raise exception 'segments: alice should nest her own outline, changed %', n; end if;

    begin
        insert into dojo.relay_segments (session_id, seq, title) values (bob_sess, 0, 'forged');
        raise exception 'segments: alice wrote into bob''s run';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'foreign segment refused for the wrong reason: %', sqlerrm; end if;
    end;

    -- ── the gate ─────────────────────────────────────────────────────────────
    insert into dojo.relay_inbox (session_id, segment_id, membership_id, kind, direction, payload)
    values (sid, seg, alice_mem, 'approval', 'agent_to_human', '{"prompt":"run tests?"}');
    begin
        insert into dojo.relay_inbox (session_id, membership_id, kind, direction, payload)
        values (bob_sess, alice_mem, 'approval', 'agent_to_human', '{}');
        raise exception 'inbox: alice raised a gate on bob''s run';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'foreign gate refused for the wrong reason: %', sqlerrm; end if;
    end;
    begin
        insert into dojo.relay_inbox (session_id, membership_id, kind, direction, payload)
        values (bob_sess, bob_mem, 'approval', 'agent_to_human', '{}');
        raise exception 'inbox: alice raised a gate as bob';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'gate-as-bob refused for the wrong reason: %', sqlerrm; end if;
    end;
end $$;

rollback;
