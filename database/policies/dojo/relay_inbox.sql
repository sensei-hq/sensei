-- RLS for dojo.relay_inbox.
--
-- Lives here, not in ddl/table/dojo/relay_inbox.ddl, because this policy CALLS a
-- function and `dbd apply` creates every table before any function. Inline, it
-- failed the deploy outright:
--   function dojo.owns_membership(...) does not exist
--
-- `dbd policies` runs after all entities exist, which is the only ordering in
-- which a function-dependent policy can be created. Policies that call no
-- function (auth.uid() alone) stay with their table — see policies/README.md.
--
-- Plain SQL, no scope guard: since dbd 0.12.0 `policies` honours --scope and
-- skips a file whose table is out of scope ("Skipped … outside scope 'default'")
-- rather than failing it. Earlier versions applied every file regardless, which
-- forced each statement through a `do $$ … execute … $$` block just to no-op on
-- the daemon plane.

-- Row-Level Security (P4.1) — see the note on dojo.relay_sessions. Own-rows-only:
-- a user reads/subscribes only their own inbox rows — ownership derived from
-- membership_id via dojo.owns_membership() (no stale user_id copy — WS-0 Rule A).
-- The Worker's service_role writes bypass RLS. SELECT-only; team-wide visibility is P6.
-- Idempotent (drop-if-exists) because the deploy re-applies this file each time.
alter table dojo.relay_inbox enable row level security;

-- Table-level SELECT grant for the `authenticated` read path (RLS filters rows;
-- the grant lets the role touch the table). See the note on dojo.relay_sessions.
grant select on dojo.relay_inbox to authenticated;

drop policy if exists relay_inbox_select_own on dojo.relay_inbox;
create policy relay_inbox_select_own
    on dojo.relay_inbox
    for select
    to authenticated
    using (dojo.owns_membership(membership_id));

-- ── writes as the person (#236 step 3b; access matrix "Relay") ──────────────
-- Only the owner answers a gate or steers their agent. Column grants limit each
-- write to what its action changes: a reply sets reply / status / answered_at,
-- never the agent's payload; a nudge supplies the steer and nothing else.
-- `seq` is drawn from relay_inbox_seq on insert and by the bump trigger on
-- update, so the role needs usage on it.
grant usage on sequence dojo.relay_inbox_seq to authenticated;
grant update (reply, status, answered_at) on dojo.relay_inbox to authenticated;
grant insert (session_id, membership_id, kind, direction, payload) on dojo.relay_inbox to authenticated;

-- Reply: the owner's PENDING gate, raised by the agent (agent → human).
drop policy if exists relay_inbox_reply_own on dojo.relay_inbox;
create policy relay_inbox_reply_own
    on dojo.relay_inbox
    for update
    to authenticated
    using (dojo.owns_membership(membership_id)
           and status = 'pending'
           and direction = 'agent_to_human')
    with check (dojo.owns_membership(membership_id));

-- Nudge: a human → agent message, as the owner, into the owner's own session.
drop policy if exists relay_inbox_nudge_own on dojo.relay_inbox;
create policy relay_inbox_nudge_own
    on dojo.relay_inbox
    for insert
    to authenticated
    with check (
        dojo.owns_membership(membership_id)
        and direction = 'human_to_agent'
        and exists (
            select 1
              from dojo.relay_sessions s
             where s.id = relay_inbox.session_id
               and s.membership_id = relay_inbox.membership_id
        )
    );
