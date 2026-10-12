-- Push subscriptions and notification prefs as the signed-in person (#236 step
-- 3b; access matrix "Relay": R/W own, requires membership).
--
-- A person registers their own devices and sets their own opt-ins. Nobody else
-- reads or changes either — a push endpoint is where the dōjō sends their
-- gates, so pointing someone else's at your device is reading their relay.
--
-- WHAT BREAKS THIS TEST: a policy on user_id alone for push_subscriptions (a
-- subscription filed under another person's membership); a missing with-check
-- (a row re-pointed at another person after the fact); an UPDATE grant on
-- push_subscriptions.user_id; a read policy wider than own; leaving the old
-- deny-all as the only policy (every write refused).

begin;

insert into dojo.principals (id, auth_user_id, display_name) values
  ('a0000000-0000-4000-8000-000000000051', 'a1000000-0000-4000-8000-000000000051', 'alice'),
  ('a0000000-0000-4000-8000-000000000052', 'a1000000-0000-4000-8000-000000000052', 'bob');
insert into dojo.tenants (id, key, origin, slug, name, dojo_url) values
  ('b0000000-0000-4000-8000-00000000005a', 'organization/ztest-push', 'organization',
   'ztest-push', 'push fixture', 'dojo.sensei-hq.org/organization/ztest-push');
insert into dojo.memberships (id, tenant_id, user_id, role, kind, authenticated_via) values
  ('d1000000-0000-4000-8000-000000000051', 'b0000000-0000-4000-8000-00000000005a',
   'a0000000-0000-4000-8000-000000000051', 'contributor', 'employer', 'github_oauth'),
  ('d1000000-0000-4000-8000-000000000052', 'b0000000-0000-4000-8000-00000000005a',
   'a0000000-0000-4000-8000-000000000052', 'contributor', 'employer', 'github_oauth');
-- Bob's own rows, made by the system, for alice to fail to see.
insert into dojo.notification_prefs (user_id, events) values
  ('a0000000-0000-4000-8000-000000000052', '{"approvals":true}');
insert into dojo.push_subscriptions (id, user_id, membership_id, platform, web_push) values
  ('e2000000-0000-4000-8000-000000000052', 'a0000000-0000-4000-8000-000000000052',
   'd1000000-0000-4000-8000-000000000052', 'web', '{"endpoint":"https://push.example/bob"}');

set local role authenticated;

do $$
declare
    n int;
    alice constant uuid := 'a0000000-0000-4000-8000-000000000051';
    bob   constant uuid := 'a0000000-0000-4000-8000-000000000052';
    alice_mem constant uuid := 'd1000000-0000-4000-8000-000000000051';
    bob_mem   constant uuid := 'd1000000-0000-4000-8000-000000000052';
begin
    perform set_config('request.jwt.claims', '{"sub":"a1000000-0000-4000-8000-000000000051"}', true);

    -- ── notification prefs ───────────────────────────────────────────────────
    select count(*) into n from dojo.notification_prefs;
    if n <> 0 then raise exception 'prefs: alice saw % row(s) of someone else''s prefs', n; end if;

    -- The route's upsert, as PostgREST sends it.
    insert into dojo.notification_prefs (user_id, events, updated_at) values (alice, '{"stalls":true}', now())
    on conflict (user_id) do update set user_id = excluded.user_id, events = excluded.events,
                                        updated_at = excluded.updated_at;
    insert into dojo.notification_prefs (user_id, events, updated_at) values (alice, '{"stalls":false}', now())
    on conflict (user_id) do update set user_id = excluded.user_id, events = excluded.events,
                                        updated_at = excluded.updated_at;
    if (select events from dojo.notification_prefs where user_id = alice) <> '{"stalls":false}' then
        raise exception 'prefs: alice''s second upsert did not update her row';
    end if;

    begin
        insert into dojo.notification_prefs (user_id, events) values (bob, '{"stalls":false}');
        raise exception 'prefs: alice wrote prefs for bob';
    exception
        when insufficient_privilege then
            if sqlerrm not like '%row-level security%' then raise exception 'prefs refused for the wrong reason: %', sqlerrm; end if;
        when unique_violation then raise exception 'prefs: alice reached bob''s row';
    end;

    begin
        update dojo.notification_prefs set user_id = bob where user_id = alice;
        raise exception 'prefs: alice handed her row to bob';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'prefs re-point refused for the wrong reason: %', sqlerrm; end if;
    end;

    -- ── push subscriptions ───────────────────────────────────────────────────
    select count(*) into n from dojo.push_subscriptions;
    if n <> 0 then raise exception 'push: alice saw % of bob''s subscription(s)', n; end if;

    insert into dojo.push_subscriptions (user_id, membership_id, platform, web_push, enabled, last_seen)
    values (alice, alice_mem, 'web', '{"endpoint":"https://push.example/alice"}', true, now());
    update dojo.push_subscriptions set enabled = true, last_seen = now(), updated_at = now(),
                                       web_push = '{"endpoint":"https://push.example/alice"}',
                                       platform = 'web', membership_id = alice_mem
     where user_id = alice and web_push->>'endpoint' = 'https://push.example/alice';
    get diagnostics n = row_count;
    if n <> 1 then raise exception 'push: alice should refresh her own subscription, changed %', n; end if;

    begin
        insert into dojo.push_subscriptions (user_id, membership_id, platform, web_push)
        values (alice, bob_mem, 'web', '{"endpoint":"https://push.example/x"}');
        raise exception 'push: alice filed a subscription under bob''s membership';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'push refused for the wrong reason: %', sqlerrm; end if;
    end;

    begin
        insert into dojo.push_subscriptions (user_id, membership_id, platform, web_push)
        values (bob, alice_mem, 'web', '{"endpoint":"https://push.example/y"}');
        raise exception 'push: alice filed a subscription as bob';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'push-as-bob refused for the wrong reason: %', sqlerrm; end if;
    end;

    begin
        insert into dojo.push_subscriptions (user_id, membership_id, platform, web_push)
        values (alice, null, 'web', '{"endpoint":"https://push.example/z"}');
        raise exception 'push: a subscription must name one of the person''s memberships';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'push-no-membership refused for the wrong reason: %', sqlerrm; end if;
    end;

    begin
        update dojo.push_subscriptions set membership_id = bob_mem where user_id = alice;
        raise exception 'push: alice re-pointed her subscription at bob''s membership';
    exception when insufficient_privilege then
        if sqlerrm not like '%row-level security%' then raise exception 'push re-point refused for the wrong reason: %', sqlerrm; end if;
    end;

    begin
        update dojo.push_subscriptions set user_id = bob where user_id = alice;
        raise exception 'push: user_id is not the person''s to change';
    exception when insufficient_privilege then
        if sqlerrm like '%row-level security%' then raise exception 'push: user_id must be refused by the column grant, not only the policy'; end if;
    end;

    update dojo.push_subscriptions set enabled = false where id = 'e2000000-0000-4000-8000-000000000052';
    get diagnostics n = row_count;
    if n <> 0 then raise exception 'push: alice disabled bob''s subscription (% row)', n; end if;
end $$;

rollback;
