# Design: replace denormalized `relay_sessions.user_id` with membership-derived RLS

**Status:** proposed · **Scope:** dōjō schema (`dojo.relay_*`) + `/v1` relay write handlers ·
**Non-goals:** `tenant_id` removal, `dojo_url` dedup, personal-tenant first-classing (§7).

## 0. Principle — sensei is user/membership-primary

Sensei is a **user's** tool; the primary access axis is the **user (via their membership)**, not
the tenant. A user spans many tenants; personal work is reached through the membership. So:

- **User/membership-primary** (personal zone `/you/*`): `relay_sessions` / `relay_inbox` /
  `relay_segments`, projects, contributions. The inbox is *"your work across every dōjō"* → it
  aggregates across **all** the user's memberships. Primary read filter = the user (via
  `membership_id`).
- **Tenant-primary** (governance + org only): rules, ladder/scopes, rule-packs, org console.
  `tenant_id` is the **write-identity** + **isolation boundary**, but secondary for personal reads.

This spec makes the `relay_*` read path user/membership-primary. See
[[reference_sensei_user_primary_model]].

## 1. Problem

`dojo.relay_sessions` (and `relay_inbox`) store `user_id` denormalized alongside
`membership_id`, even though `membership_id → memberships(user_id, tenant_id)` already
determines it. The redundancy exists only to make the own-rows RLS policy a bare column
compare (`user_id = auth.uid()`). Costs:

- **Stale on re-ownership (real bug).** Changing `memberships.user_id` (re-pairing, ownership
  transfer, the F6 verify re-own) does NOT update existing `relay_sessions.user_id` → those
  rows silently point at the old owner and mis-scope RLS.
- **Duplication** of a value the FK already implies, on every session + inbox row.

## 2. Decision

Derive ownership from `membership_id` via a `SECURITY DEFINER STABLE` helper and **drop the
`user_id` column** from `relay_sessions` and `relay_inbox`. `relay_segments` (already
join-based) repoints its join through the membership. `tenant_id` stays (§6 — it's structural,
not RLS-only). Keep `membership_id` — it becomes the single ownership reference.

Why this is safe with Realtime: the subscription (`relay-realtime.ts`) attaches
`postgres_changes` with **no column filter** and relies purely on RLS to scope the stream. So
nothing hard-requires a `user_id` column; a function-based policy is authorized by Realtime the
same way. (A client-side `filter: user_id=eq.…` *would* require the column — we don't use one.)

## 3. Design

### 3.1 Ownership helper (one function, reused by all three tables)
```sql
create or replace function dojo.owns_membership(mid uuid)
  returns boolean
  language sql stable security definer
  set search_path = dojo
as $$
  select exists (
    select 1 from dojo.memberships m
    where m.id = mid and m.user_id = (select auth.uid())
  );
$$;
revoke all on function dojo.owns_membership(uuid) from public;
grant execute on function dojo.owns_membership(uuid) to authenticated;
```
- `STABLE` → planner evaluates once per statement (initplan), not per row.
- `SECURITY DEFINER` → reads `memberships` regardless of the caller's grants; `search_path`
  pinned to avoid injection.
- `(select auth.uid())` → the Supabase-recommended form so `auth.uid()` is an initplan constant.

### 3.2 New policies
```sql
-- relay_sessions: was `using (user_id = auth.uid())`
create policy relay_sessions_select_own on dojo.relay_sessions
  for select to authenticated using (dojo.owns_membership(membership_id));

-- relay_inbox: same swap
create policy relay_inbox_select_own on dojo.relay_inbox
  for select to authenticated using (dojo.owns_membership(membership_id));

-- relay_segments: was a join to s.user_id; repoint to the session's membership
create policy relay_segments_select_own on dojo.relay_segments
  for select to authenticated using (
    exists (select 1 from dojo.relay_sessions s
            where s.id = relay_segments.session_id
              and dojo.owns_membership(s.membership_id))
  );
```

### 3.3 Column + index changes
- `relay_sessions`: drop `user_id`; drop `relay_sessions_user_idx (user_id, started_at desc)`.
  The client-direct read (RLS own-rows, ordered by `started_at`) now filters via the function;
  add `relay_sessions_membership_idx (membership_id, started_at desc)` to support it. (Worker
  reads still use `relay_sessions_tenant_idx` — unchanged.)
- `relay_inbox`: drop `user_id`; replace `relay_inbox_user_pending_idx (user_id) where pending`
  with `relay_inbox_membership_pending_idx (membership_id) where status='pending'`.
- `relay_segments`: no column change (already user_id-free); policy body updated.

### 3.4 Write path
- `POST /v1/.../relay/session`: stop writing `user_id` (the row no longer has it). It currently
  sets `user_id: caller.userId` — remove that key from the upsert.
- Wherever `relay_inbox` rows are inserted (gate-raise path): same removal.
- **Daemon is unaffected.** `user_id` is derived server-side from the device token
  (`resolveApiKeyAccess`), never sent by the daemon. No protocol/daemon change.

### 3.5 Push path (audit before merge)
`relay-push-send.ts` targets a user's push subscriptions. The crash-push in the session POST
already uses `caller.userId` (from the token, not the row) → fine. **Audit** any push/nudge path
that reads a *stored* `relay_sessions.user_id` to identify the run's owner; repoint those to
resolve the owner via `membership_id → memberships.user_id` (server-side, service_role).

## 4. Blast radius (files)
- DDL: `table/dojo/relay_sessions.ddl`, `relay_inbox.ddl`, `relay_segments.ddl` + a new
  `function/dojo/owns_membership.ddl` (or inline).
- Handlers: `relay/session/+server.ts` (drop `user_id` write), the `relay_inbox` insert path.
- Push: `server/relay-push-send.ts` (audit §3.5).
- Tests: RLS behaviour (§5), plus the POST-shape specs that assert `user_id` in the upsert.
- **Not touched:** the daemon/protocol, `tenant_id` isolation, the Worker tenant filters, the
  Realtime channel spec, the GET column list (never selected `user_id`).

## 5. Testing (security-critical — a wrong policy = cross-user leak)
- Own-rows: user A sees A's runs/inbox/segments; user B's rows are invisible to A (direct
  Supabase read as each JWT).
- **Re-ownership:** flip `memberships.user_id` A→B; A immediately loses visibility, B gains it,
  with **no `relay_sessions` update** (proves the bug is fixed).
- Realtime: an authed channel for A receives A's changes only; B's changes never arrive.
- Perf smoke: EXPLAIN the own-rows read uses the new `membership_idx`; confirm the function is
  an initplan, not per-row.
- Keep tenant-isolation tests green (unchanged).

## 6. `tenant_id` — REVISED: drop it too; relay tables key on `membership_id` only

**Revised 2026-07-28 (supersedes the earlier "keep tenant_id").** Drop BOTH `user_id` and
`tenant_id` from the relay tables; the sole key is `membership_id`. Rationale: `membership_id` is a
*reference*; `user_id`/`tenant_id` are *copies* that go stale on re-ownership (the bug). One
membership → exactly one tenant (`memberships` is unique on `(tenant_id, user_id)`), so both derive
live via the join. The three uses I cited for keeping `tenant_id` all collapse:

- **Write identity:** `unique(tenant_id, run_id)` → `unique(membership_id, run_id)`; `onConflict`
  → `membership_id,run_id` (equally unique, more precise, no stale copy).
- **Reads (both modes resolve through memberships anyway under user-primary):** personal inbox =
  the user's memberships (user-wide, across dōjōs); org console = the tenant's memberships. So
  `tenant_id` only saved a join — not load-bearing. Add a `relay_sessions(membership_id)` index; the
  org read filters `membership_id IN (memberships of tenant X)`.
- **"Every dojo.* row carries tenant_id":** the tenant-primary pattern — wrong for **user-work**
  tables (`entity-access-model.md` §3).

**The line:** user-work tables (`relay_sessions`, `relay_inbox`; `relay_segments` already keys on
`session_id`) → **`membership_id` only, no `user_id`/`tenant_id`**. Governance/org tables (rules,
ladder, policies, incidents, engagements, identities, billing, audit_events) → **keep `tenant_id`**
(genuinely tenant-primary). Personal inbox read goes user-wide (was `listRuns(tenantKey)`); org
console read is membership-of-tenant scoped. RLS everywhere = `owns_membership(membership_id)`; no
denormalized copies remain to go stale.

## 7. Deferred (captured so they aren't lost)
- **`memberships.dojo_url` is redundant** with `tenants.dojo_url` (the URL is a property of the
  dōjō instance). Drop it on the dōjō side and read from the tenant (the daemon-side mirror
  legitimately keeps its own). Separate change.
- **Personal tenant is ad-hoc:** `key="personal/jerry"` uses "personal" as a pseudo-origin not
  in the `tenant_origin` enum (`github|org`). Consider a first-class personal `scope`/`origin`
  so personal dōjōs stop bolting onto `origin='org'`.

## 8. Migration note (pre-release)
DDL is declaratively re-applied (dbd `Current`/`Fresh`). A **column drop** is destructive — the
edited `.ddl` drops `user_id`, but confirm the deploy path performs the drop (dbd reconcile is
additive-only; a drop may need an explicit step). Policies use `drop policy if exists` first
(already the file convention). Sequence: create function → replace policies → drop indexes →
drop columns, so no policy ever references a dropped column mid-deploy.
