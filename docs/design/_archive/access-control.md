# Database Access Control Design

## DDL Structure Fix

The current `database/ddl/` layout conflates views and tables under `table/`. Move views:

```
database/ddl/
├── procedure/
│   ├── core/           (get_user_teams — to be removed, replaced by view)
│   ├── sensei/         match_embeddings, rank_bm25, rank_bfs
│   └── staging/        import_* procedures
├── table/
│   ├── core/           8 tables (accounts, profiles, teams, …)
│   ├── sensei/         ~28 tables (repos, sessions, symbols, …)
│   └── staging/        6 staging buffer tables
└── view/               ← NEW — move all VIEW DDLs here
    ├── core/
    │   └── user_teams.ddl        (currently misplaced in table/sensei/)
    ├── platform/
    │   ├── account_stats.ddl     (currently misplaced in table/platform/)
    │   ├── ftr_distribution.ddl  (currently misplaced in table/platform/)
    │   └── tool_usage.ddl        (currently misplaced in table/platform/)
    └── sensei/
        (empty for now)
```

---

## Roles

PostgREST surfaces three Postgres roles:

| Role | Who | Notes |
|------|-----|-------|
| `anon` | Unauthenticated requests | Very limited — public repos only |
| `authenticated` | Logged-in users (JWT from Supabase) | Standard user access, RLS enforces row filtering |
| `service_role` | Server-side code (kavach /data, /rpc, indexer) | Bypasses RLS — never expose to browser |

App-level role (from `app_metadata.role`) maps into Postgres using a custom JWT claim check:
- `platform_admin` — access to platform schema views; all-tenant read on sensei

---

## Helper Functions (add to `public` schema)

```sql
-- Current user's Supabase auth UUID
CREATE OR REPLACE FUNCTION public.auth_uid() RETURNS uuid
  LANGUAGE sql STABLE AS $$ SELECT auth.uid() $$;

-- Current user's app-level role from JWT app_metadata
CREATE OR REPLACE FUNCTION public.auth_app_role() RETURNS text
  LANGUAGE sql STABLE AS $$
    SELECT coalesce(
      (auth.jwt()->'app_metadata'->>'role'),
      'member'
    )
  $$;

-- TRUE if current user is member of the given account
CREATE OR REPLACE FUNCTION public.is_account_member(p_account_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER AS $$
    SELECT EXISTS (
      SELECT 1 FROM core.profile_accounts
      WHERE user_id = auth.uid() AND account_id = p_account_id
    )
  $$;

-- TRUE if current user has at least the given role in an account
CREATE OR REPLACE FUNCTION public.has_account_role(p_account_id uuid, p_role text) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER AS $$
    SELECT EXISTS (
      SELECT 1 FROM core.profile_accounts
      WHERE user_id = auth.uid()
        AND account_id = p_account_id
        AND CASE p_role
              WHEN 'member' THEN role IN ('member','admin','owner')
              WHEN 'admin'  THEN role IN ('admin','owner')
              WHEN 'owner'  THEN role = 'owner'
            END
    )
  $$;

-- TRUE if current user has access to a repo (owns it or it is public)
CREATE OR REPLACE FUNCTION public.can_access_repo(p_repo_id uuid) RETURNS boolean
  LANGUAGE sql STABLE SECURITY DEFINER AS $$
    SELECT EXISTS (
      SELECT 1 FROM sensei.repos
      WHERE id = p_repo_id
        AND (owner_id = auth.uid() OR is_public = true)
    )
  $$;
```

---

## Schema-Level Grants

```sql
-- anon: no direct schema access (PostgREST blocks at JWT level)
GRANT USAGE ON SCHEMA public    TO anon;

-- authenticated: read-only on catalog schemas; read+write on their own data
GRANT USAGE ON SCHEMA public    TO authenticated;
GRANT USAGE ON SCHEMA core      TO authenticated;
GRANT USAGE ON SCHEMA sensei    TO authenticated;
-- platform schema: granted per-object only for platform_admin (see below)

-- service_role: full access (indexer, kavach backend)
GRANT USAGE ON SCHEMA core, sensei, platform, staging TO service_role;
GRANT ALL   ON ALL TABLES    IN SCHEMA core, sensei, platform, staging TO service_role;
GRANT ALL   ON ALL SEQUENCES IN SCHEMA core, sensei, platform, staging TO service_role;
GRANT ALL   ON ALL ROUTINES  IN SCHEMA core, sensei, platform, staging TO service_role;
```

---

## Core Schema — Table Policies

### `core.profiles`
```sql
ALTER TABLE core.profiles ENABLE ROW LEVEL SECURITY;

-- Anyone authenticated can read any profile (username, avatar — not sensitive)
CREATE POLICY profiles_read   ON core.profiles FOR SELECT TO authenticated USING (true);
-- Users can only update their own profile
CREATE POLICY profiles_write  ON core.profiles FOR UPDATE TO authenticated USING (user_id = auth.uid());
```

### `core.accounts`
```sql
ALTER TABLE core.accounts ENABLE ROW LEVEL SECURITY;

-- Users can only see accounts they belong to
CREATE POLICY accounts_read ON core.accounts FOR SELECT TO authenticated
  USING (is_account_member(id));
-- Only service_role inserts/updates accounts (done via kavach backend)
```

### `core.profile_accounts`
```sql
ALTER TABLE core.profile_accounts ENABLE ROW LEVEL SECURITY;

-- Users see only their own memberships
CREATE POLICY pa_read ON core.profile_accounts FOR SELECT TO authenticated
  USING (user_id = auth.uid());
-- Owners and admins can see all members in their account
CREATE POLICY pa_read_account ON core.profile_accounts FOR SELECT TO authenticated
  USING (has_account_role(account_id, 'admin'));
```

### `core.teams`
```sql
ALTER TABLE core.teams ENABLE ROW LEVEL SECURITY;

-- Members of the account can see its teams
CREATE POLICY teams_read ON core.teams FOR SELECT TO authenticated
  USING (is_account_member(account_id));
-- Admins/owners can create and modify teams
CREATE POLICY teams_write ON core.teams FOR ALL TO authenticated
  USING (has_account_role(account_id, 'admin'))
  WITH CHECK (has_account_role(account_id, 'admin'));
```

### `core.team_members`
```sql
ALTER TABLE core.team_members ENABLE ROW LEVEL SECURITY;

-- Members of the account can view team rosters
CREATE POLICY tm_read ON core.team_members FOR SELECT TO authenticated
  USING (
    EXISTS (
      SELECT 1 FROM core.teams t WHERE t.id = team_id AND is_account_member(t.account_id)
    )
  );
-- Maintainers and above can add/remove members
CREATE POLICY tm_write ON core.team_members FOR ALL TO authenticated
  USING (role IN ('maintainer') OR has_account_role(
    (SELECT account_id FROM core.teams WHERE id = team_id), 'admin'
  ));
```

### `core.api_keys`
```sql
ALTER TABLE core.api_keys ENABLE ROW LEVEL SECURITY;

-- Only account admins+ can view key metadata (never expose encrypted_value via API)
CREATE POLICY api_keys_read ON core.api_keys FOR SELECT TO authenticated
  USING (has_account_role(account_id, 'admin'));
CREATE POLICY api_keys_write ON core.api_keys FOR ALL TO authenticated
  USING (has_account_role(account_id, 'owner'))
  WITH CHECK (has_account_role(account_id, 'owner'));
```
> **Note:** `encrypted_value` should be excluded from all `SELECT` via column-level security or by never including it in `:select` params via the `/data` endpoint.

### `core.account_keys`
```sql
ALTER TABLE core.account_keys ENABLE ROW LEVEL SECURITY;

-- No authenticated access — service_role only. DEKs must never leave the server.
-- No policies needed: enabling RLS with no policies = deny all for non-service_role
ALTER TABLE core.account_keys ENABLE ROW LEVEL SECURITY;
```

### `core.invitations`
```sql
ALTER TABLE core.invitations ENABLE ROW LEVEL SECURITY;

-- Invitees can see their own pending invite
CREATE POLICY inv_own ON core.invitations FOR SELECT TO authenticated
  USING (email = (SELECT email FROM auth.users WHERE id = auth.uid()));
-- Account admins can see all invites for their account
CREATE POLICY inv_admin ON core.invitations FOR SELECT TO authenticated
  USING (has_account_role(account_id, 'admin'));
CREATE POLICY inv_write ON core.invitations FOR ALL TO authenticated
  USING (has_account_role(account_id, 'admin'))
  WITH CHECK (has_account_role(account_id, 'admin'));
```

---

## Sensei Schema — Table Policies

The key predicate for sensei is **repo ownership**. A user can access a repo if:
- `repos.owner_id = auth.uid()` — they own it directly, OR
- `repos.is_public = true` — it is public

### `sensei.repos`
```sql
ALTER TABLE sensei.repos ENABLE ROW LEVEL SECURITY;

CREATE POLICY repos_read ON sensei.repos FOR SELECT TO authenticated
  USING (owner_id = auth.uid() OR is_public = true);
CREATE POLICY repos_write ON sensei.repos FOR ALL TO authenticated
  USING (owner_id = auth.uid())
  WITH CHECK (owner_id = auth.uid());
```

### Indexing tables (service_role write, authenticated read-by-repo)
Applies to: `symbols`, `call_edges`, `imports`, `scan_state`, `symbol_map`,
`chunks`, `embeddings`, `docs`, `doc_sections`

```sql
-- Template (repeat for each table, adjusting table name):
ALTER TABLE sensei.symbols ENABLE ROW LEVEL SECURITY;
CREATE POLICY symbols_read ON sensei.symbols FOR SELECT TO authenticated
  USING (can_access_repo(repo_id));
-- Writes handled by service_role only (indexer) — no authenticated write policy
```

### `sensei.sessions` / `sensei.task_sessions` / `sensei.task_turns` / `sensei.snapshots` / `sensei.context_packs`
```sql
-- Users see only sessions for their repos
CREATE POLICY sessions_read ON sensei.sessions FOR SELECT TO authenticated
  USING (can_access_repo(repo_id));
CREATE POLICY sessions_write ON sensei.sessions FOR ALL TO authenticated
  USING (can_access_repo(repo_id))
  WITH CHECK (can_access_repo(repo_id));
-- Same pattern for task_sessions, task_turns, snapshots, context_packs
```

### `sensei.memory_items`
```sql
ALTER TABLE sensei.memory_items ENABLE ROW LEVEL SECURITY;
CREATE POLICY mem_read  ON sensei.memory_items FOR SELECT TO authenticated
  USING (can_access_repo(repo_id));
CREATE POLICY mem_write ON sensei.memory_items FOR ALL TO authenticated
  USING (can_access_repo(repo_id))
  WITH CHECK (can_access_repo(repo_id));
```

### `sensei.events`
```sql
ALTER TABLE sensei.events ENABLE ROW LEVEL SECURITY;
-- Users see only their own events
CREATE POLICY events_read ON sensei.events FOR SELECT TO authenticated
  USING (user_uuid = auth.uid());
-- Writes via service_role (collector) only
```

### `sensei.api_requests`
```sql
ALTER TABLE sensei.api_requests ENABLE ROW LEVEL SECURITY;
-- Users see cost data for their repos
CREATE POLICY api_requests_read ON sensei.api_requests FOR SELECT TO authenticated
  USING (can_access_repo(repo_id));
```

### Global catalog (read-only for authenticated, write via service_role)
Applies to: `libraries`, `references`, `shared_libs`, `shared_lib_sections`, `repo_libraries`

```sql
ALTER TABLE sensei.libraries ENABLE ROW LEVEL SECURITY;
CREATE POLICY libraries_read ON sensei.libraries FOR SELECT TO authenticated USING (true);
-- Same for references, shared_libs, shared_lib_sections
```

### Per-repo libs (`repo_libs`, `lib_doc_sections`)
```sql
ALTER TABLE sensei.repo_libs ENABLE ROW LEVEL SECURITY;
CREATE POLICY repo_libs_read ON sensei.repo_libs FOR SELECT TO authenticated
  USING (can_access_repo(repo_id));
CREATE POLICY repo_libs_write ON sensei.repo_libs FOR ALL TO authenticated
  USING (can_access_repo(repo_id))
  WITH CHECK (can_access_repo(repo_id));
-- Same for lib_doc_sections
```

### `sensei.sync_sessions`
```sql
ALTER TABLE sensei.sync_sessions ENABLE ROW LEVEL SECURITY;
-- Account members can see their account's sync data (PII-scrubbed)
CREATE POLICY sync_read ON sensei.sync_sessions FOR SELECT TO authenticated
  USING (is_account_member(account_id));
-- Writes via service_role (collector) only
```

### `sensei.benchmark_runs` / `sensei.benchmark_reports`
```sql
ALTER TABLE sensei.benchmark_runs ENABLE ROW LEVEL SECURITY;
CREATE POLICY bench_runs_read ON sensei.benchmark_runs FOR SELECT TO authenticated
  USING (can_access_repo(repo_id));
-- Writes via service_role only
```

### `sensei.user_teams` (VIEW)
```sql
-- RLS on the view itself is not needed — the underlying tables (core.team_members,
-- core.teams, core.accounts) all have RLS. However since this view is accessed via
-- service_role in the /data endpoint, filter by user_id is applied in the query:
-- GET /data?entity=user_teams&user_id=eq.<auth_uid>
-- Long term: switch /data to use the authenticated JWT so RLS applies automatically.
GRANT SELECT ON sensei.user_teams TO authenticated;
```

---

## Platform Schema — View Policies

Platform views are read-only aggregates. Access is restricted to `platform_admin` only.
Since views don't support `FOR authenticated USING (...)` with JWT-based role checks
directly, the cleanest approach is a Postgres row security policy on the underlying view
using the `auth_app_role()` helper, or (simpler) grant SELECT only to a dedicated
Postgres role `platform_admin` mapped from the JWT claim.

```sql
-- Option A: grant to a Postgres role matched from JWT (requires custom JWT hook)
CREATE ROLE platform_admin;
GRANT SELECT ON platform.account_stats   TO platform_admin;
GRANT SELECT ON platform.ftr_distribution TO platform_admin;
GRANT SELECT ON platform.tool_usage      TO platform_admin;

-- Option B (simpler, no new Postgres role): check via a security barrier view
CREATE OR REPLACE VIEW platform.account_stats_safe
  WITH (security_barrier = true) AS
  SELECT * FROM platform.account_stats
  WHERE auth_app_role() = 'platform_admin';
-- Then expose account_stats_safe instead of account_stats
```

---

## `/data` and `/rpc` Endpoint Strategy

Kavach's `/data` and `/rpc` routes currently use `service_role` which bypasses all RLS.
The correct long-term architecture:

1. **Authenticated user requests** → `/data` uses user's JWT (Pass `Authorization: Bearer <access_token>`)
   → PostgREST applies RLS automatically
   → No extra filtering needed in the endpoint

2. **Server-side / indexer requests** → direct Supabase client with `service_role`
   → RLS bypassed (correct for indexer, staging imports, etc.)

Until kavach supports forwarding the user JWT to PostgREST, the current service_role
approach is acceptable **only if** the `/data` endpoint validates that the requested
`user_id` filter matches `locals.session.user.id` for user-scoped entities.

---

## Summary Table

| Schema | Entity | anon | authenticated | platform_admin | service_role |
|--------|--------|------|---------------|----------------|--------------|
| core | profiles | — | read-all, write-own | — | full |
| core | accounts | — | read-own-accounts | — | full |
| core | profile_accounts | — | read-own + admin sees all | — | full |
| core | teams | — | read-if-member, write-if-admin | — | full |
| core | team_members | — | read-if-member, write-if-maintainer | — | full |
| core | api_keys | — | read/write-if-owner | — | full |
| core | account_keys | — | **none** | — | full |
| core | invitations | — | read-own + admin, write-if-admin | — | full |
| sensei | repos | — | read-own+public, write-own | — | full |
| sensei | symbols/chunks/embeddings/etc. | — | read-if-repo-access | — | full |
| sensei | sessions/task_sessions/snapshots | — | read+write-if-repo-access | — | full |
| sensei | events | — | read-own-uuid | — | full |
| sensei | api_requests | — | read-if-repo-access | — | full |
| sensei | libraries/references/shared_libs | — | read-all | — | full |
| sensei | sync_sessions | — | read-if-account-member | — | full |
| sensei | benchmark_runs/reports | — | read-if-repo-access | — | full |
| sensei | user_teams (view) | — | read-own (filtered by user_id) | — | full |
| platform | account_stats | — | **none** | read | full |
| platform | ftr_distribution | — | **none** | read | full |
| platform | tool_usage | — | **none** | read | full |
| staging | all | — | **none** | — | full |
