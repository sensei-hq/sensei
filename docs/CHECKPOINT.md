# Checkpoint

**Slice:** Dōjō auth & provisioning — phase 1 (`docs/spec/dojo/dojo-auth-provisioning.md`, Parts I–VIII)

## Done — the four prerequisites, each red-first

| # | what | commit |
|---|---|---|
| 1 | RLS resolves `auth.uid()` → principal. **Three** surfaces, not one: `dojo.projects`' policy, `dojo.owns_membership` (backs all three relay_* policies), `can_read_repository_metric:48`. One new `dojo.current_principal_id()`; the projects policy moved to `policies/dojo/projects.sql` (it calls a function now). | `bb994b6a` |
| 8 | `database/tests/` + `make test-db` — SQL assertions against a real Postgres. `make test` also gained `test-dojo`, which was in no aggregate target at all. | `bb994b6a` |
| 2a | `resolveCaller` returns the **principal** id as `userId` (+ `authUserId`). New `principal-resolve.ts`; creates the principal on first sight, survives the concurrent-sign-in 23505 by re-reading. | `dd6917f7` |
| 4 | Every `dojo.identities` path onto `principal_id`. Tenant isolation is now an **explicit membership check** — the dropped `tenant_id` filter was providing it incidentally. | `5cbe4d4d` |
| 3 | `createDojo` → `organization/{slug}` + `slug` column. **Issue #117's own AC.** | `11ebd83e` |

## Next — the remaining four

2b. `ensureProvisioned(userId, forgeToken, provider)` — identities → personal tenant → org tenants + `tenant_connections` → memberships, idempotent
5. `POST /v1/you/provision`, and `/v1/auth/cli/token` provisioning **without reshaping its response**
6. `POST /v1/you/repositories` — `repo_key → (provider, org)` → `tenant_connections` → tenant; reuse the Rust `normalize_repo_key`, do **not** re-implement it
7. `GET /v1/you/sync/plan` — everything registered `allowed` in phase 1

Next command: `cd dojo && bun run test` to confirm the gate, then 2b red-first.

## Gates (all green as of `11ebd83e`)

`make test-db` → 3 files · `cd dojo && bun run test` → 124 files / 1344 · `bun run check` → 0 errors, 0 warnings / 1752 files · `dbd diff --scope dojo --exit-code` → in sync

## Facts worth not re-deriving

- **Postgres validates a `language sql` body at CREATE time** — a function calling a function is order-dependent. `dbd apply --scope dojo --dry-run` prints the real order and reports issues. A `plpgsql` body is *not* validated.
- A policy that calls a function must live in `policies/`, not the table DDL (`dbd apply` creates every table before any function). Moving the GRANT there too is safe — a bare `dbd apply` then leaves `authenticated` with no grant at all, a loud failure rather than a silent leak.
- `psql` is at `/opt/homebrew/bin` (keg-only libpq; not always on PATH).
- The Worker uses `service_role` and **bypasses RLS**, so a broken policy is invisible to every app-level test. Only a read as `authenticated` sees it.

## Open

- Unobserved: whether Supabase returns `provider_token` on the PKCE exchange. A dōjō sign-in confirms it and is the first end-to-end test (the reset dropped `personal/jerry`). Blocks nothing before item 5.
