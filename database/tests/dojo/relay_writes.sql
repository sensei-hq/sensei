-- Relay WRITES as the signed-in person (#236 step 3b; access matrix "Relay").
--
-- Relay content is the person's own machine: only they answer their gates,
-- review their run's segments and steer their agent. These are the three writes
-- the phone makes — reply (relay_inbox UPDATE), review (relay_segments UPDATE)
-- and nudge (relay_inbox INSERT) — and each may change only what that action
-- changes, through column grants as well as row policies.
--
-- WHAT BREAKS THIS TEST: a write policy without owns_membership; a reply policy
-- that admits a row already answered or one flowing human → agent; a nudge policy
-- that admits another person's membership or the agent → human direction; a
-- table-wide UPDATE grant instead of the action's columns; a missing usage grant
-- on relay_inbox_seq (every reply and nudge would be refused).
--
-- NOT separable here, on purpose: a write policy's OWNERSHIP check. Postgres
-- lets an UPDATE reach only rows the SELECT policy shows, and relay reads are
-- own-only, so bob cannot reach alice's rows whatever the write policy says.
-- The check stays as defence in depth: it becomes load-bearing the day relay
-- reads widen (the peer-status view, #252).

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000041', 'a1000000-0000-4000-8000-000000000041', 'alice'),
  ('a0000000-0000-4000-8000-000000000042', 'a1000000-0000-4000-8000-000000000042', 'bob');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000004a', 'organization/ztest-relay-w', 'organization',
   'ztest-relay-w', 'relay writes fixture', 'dojo.sensei-hq.org/organization/ztest-relay-w');
insert into dojo.memberships (id, tenant_id, user_id, role, kind, authenticated_via) values
  ('d1000000-0000-4000-8000-000000000041', 'b0000000-0000-4000-8000-00000000004a',
   'a0000000-0000-4000-8000-000000000041', 'contributor', 'employer', 'github_oauth'),
  ('d1000000-0000-4000-8000-000000000042', 'b0000000-0000-4000-8000-00000000004a',
   'a0000000-0000-4000-8000-000000000042', 'contributor', 'employer', 'github_oauth');
insert into dojo.relay_sessions (id, membership_id, run_id) values
  ('c1000000-0000-4000-8000-000000000041', 'd1000000-0000-4000-8000-000000000041',
   'c2000000-0000-4000-8000-000000000041');
insert into dojo.relay_inbox (id, session_id, membership_id, kind, direction, status) values
  ('e1000000-0000-4000-8000-000000000001', 'c1000000-0000-4000-8000-000000000041',
   'd1000000-0000-4000-8000-000000000041', 'approval', 'agent_to_human', 'pending'),
  ('e1000000-0000-4000-8000-000000000002', 'c1000000-0000-4000-8000-000000000041',
   'd1000000-0000-4000-8000-000000000041', 'approval', 'agent_to_human', 'answered'),
  ('e1000000-0000-4000-8000-000000000003', 'c1000000-0000-4000-8000-000000000041',
   'd1000000-0000-4000-8000-000000000041', 'nudge', 'human_to_agent', 'pending');
insert into dojo.relay_segments (id, session_id, seq, title) values
  ('f1000000-0000-4000-8000-000000000001', 'c1000000-0000-4000-8000-000000000041', 0, 'Phase 1');

set local role authenticated;

do $$
declare
    n int;
    as_alice constant text := '{"sub":"a1000000-0000-4000-8000-000000000041"}';
    as_bob   constant text := '{"sub":"a1000000-0000-4000-8000-000000000042"}';
begin
    -- ── reply ────────────────────────────────────────────────────────────────
    perform set_config('request.jwt.claims', as_bob, true);
    update dojo.relay_inbox set reply = '{"verdict":"approve"}', status = 'answered', answered_at = now()
     where id = 'e1000000-0000-4000-8000-000000000001';
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'reply: bob answered alice''s gate (% row)', n; end if;

    perform set_config('request.jwt.claims', as_alice, true);
    update dojo.relay_inbox set reply = '{"verdict":"approve"}', status = 'answered', answered_at = now()
     where id in ('e1000000-0000-4000-8000-000000000002', 'e1000000-0000-4000-8000-000000000003');
    get diagnostics n = row_count;
    if n <> 0 then
        raise exception 'reply: only a PENDING agent → human gate may be answered, changed %', n;
    end if;

    update dojo.relay_inbox set reply = '{"verdict":"approve"}', status = 'answered', answered_at = now()
     where id = 'e1000000-0000-4000-8000-000000000001';
    get diagnostics n = row_count;
    if n <> 1 then raise exception 'reply: alice should answer her own pending gate, changed %', n; end if;

    begin
        update dojo.relay_inbox set payload = '{"forged":true}' where id = 'e1000000-0000-4000-8000-000000000001';
        raise exception 'reply: the payload is the agent''s, not the replier''s to change';
    exception when insufficient_privilege then null;
    end;

    -- ── review ───────────────────────────────────────────────────────────────
    perform set_config('request.jwt.claims', as_bob, true);
    update dojo.relay_segments set response_verdict = 'approve', submitted_at = now(), updated_at = now()
     where id = 'f1000000-0000-4000-8000-000000000001';
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'review: bob reviewed alice''s segment (% row)', n; end if;

    perform set_config('request.jwt.claims', as_alice, true);
    update dojo.relay_segments set response_verdict = 'approve', response_note = 'ok',
                                   submitted_at = now(), updated_at = now()
     where id = 'f1000000-0000-4000-8000-000000000001';
    get diagnostics n = row_count;
    if n <> 1 then raise exception 'review: alice should review her own segment, changed %', n; end if;

    begin
        update dojo.relay_segments set title = 'renamed' where id = 'f1000000-0000-4000-8000-000000000001';
        raise exception 'review: the outline is the agent''s, not the reviewer''s to rename';
    exception when insufficient_privilege then null;
    end;

    -- ── nudge ────────────────────────────────────────────────────────────────
    insert into dojo.relay_inbox (session_id, membership_id, kind, direction, payload)
    values ('c1000000-0000-4000-8000-000000000041', 'd1000000-0000-4000-8000-000000000041',
            'nudge', 'human_to_agent', '{"text":"focus"}');

    begin
        insert into dojo.relay_inbox (session_id, membership_id, kind, direction, payload)
        values ('c1000000-0000-4000-8000-000000000041', 'd1000000-0000-4000-8000-000000000042',
                'nudge', 'human_to_agent', '{"text":"as bob"}');
        raise exception 'nudge: alice must not write as another membership';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'nudge refused for the wrong reason: %', sqlerrm; end if;
    end;

    begin
        insert into dojo.relay_inbox (session_id, membership_id, kind, direction, payload)
        values ('c1000000-0000-4000-8000-000000000041', 'd1000000-0000-4000-8000-000000000041',
                'approval', 'agent_to_human', '{}');
        raise exception 'nudge: the phone must not raise a gate as if it were the agent';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'gate refused for the wrong reason: %', sqlerrm; end if;
    end;

    perform set_config('request.jwt.claims', as_bob, true);
    begin
        insert into dojo.relay_inbox (session_id, membership_id, kind, direction, payload)
        values ('c1000000-0000-4000-8000-000000000041', 'd1000000-0000-4000-8000-000000000042',
                'nudge', 'human_to_agent', '{"text":"steer alice"}');
        raise exception 'nudge: bob must not steer alice''s session';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'steer refused for the wrong reason: %', sqlerrm; end if;
    end;
end $$;

rollback;
