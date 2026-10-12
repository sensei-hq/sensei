-- RLS for dojo.relay_segments.
--
-- Lives here, not in ddl/table/dojo/relay_segments.ddl, because this policy CALLS a
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

-- Row-Level Security (P4.1) — see the note on dojo.relay_sessions. relay_segments
-- carries no user_id, so ownership is derived by joining through the owning session:
-- a user reads/subscribes a segment only if it belongs to THEIR run. The Worker's
-- service_role writes bypass RLS. SELECT-only; team-wide visibility is P6.
-- Idempotent (drop-if-exists) because the deploy re-applies this file each time.
alter table dojo.relay_segments enable row level security;

-- Table-level SELECT grant for the `authenticated` read path (RLS filters rows;
-- the grant lets the role touch the table). See the note on dojo.relay_sessions.
grant select on dojo.relay_segments to authenticated;

drop policy if exists relay_segments_select_own on dojo.relay_segments;
create policy relay_segments_select_own
    on dojo.relay_segments
    for select
    to authenticated
    using (
        exists (
            select 1
            from dojo.relay_sessions s
            where s.id = relay_segments.session_id
              and dojo.owns_membership(s.membership_id)
        )
    );

-- ── review as the person (#236 step 3b; access matrix "Relay") ──────────────
-- Only the owner of the run records verdicts on its segments, and only the
-- verdict columns: the outline itself is the agent's.
grant update (response_verdict, response_note, submitted_at, updated_at) on dojo.relay_segments to authenticated;

drop policy if exists relay_segments_review_own on dojo.relay_segments;
create policy relay_segments_review_own
    on dojo.relay_segments
    for update
    to authenticated
    using (
        exists (
            select 1
              from dojo.relay_sessions s
             where s.id = relay_segments.session_id
               and dojo.owns_membership(s.membership_id)
        )
    )
    with check (
        exists (
            select 1
              from dojo.relay_sessions s
             where s.id = relay_segments.session_id
               and dojo.owns_membership(s.membership_id)
        )
    );
