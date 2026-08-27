# Checkpoint

**Slice:** Dōjō auth & provisioning — phase 1 (`docs/spec/dojo/dojo-auth-provisioning.md`, Parts I–VIII)

## Phase 1 is code-complete — all 8 items, each red-first

| # | what | commit |
|---|---|---|
| 1 | RLS resolves `auth.uid()` → principal. **Three** surfaces, not one: `dojo.projects`' policy, `dojo.owns_membership` (backs all three relay_* policies), `can_read_repository_metric:48`. One new `dojo.current_principal_id()`. | `bb994b6a` |
| 8 | `database/tests/` + `make test-db`. `make test` also gained `test-dojo`, which was in no aggregate target at all. | `bb994b6a` |
| 2a | `resolveCaller` returns the **principal** id as `userId` (+ `authUserId`). New `principal-resolve.ts`. | `dd6917f7` |
| 4 | Every `dojo.identities` path onto `principal_id`; tenant isolation is now an **explicit membership check**. | `5cbe4d4d` |
| 3 | `createDojo` → `organization/{slug}` + `slug`. **Issue #117's own AC.** | `11ebd83e` |
| 2b | **`ensureProvisioned`** — the operation that creates a tenant. `forge-github.ts` + `provisioning.ts` + `fake-dojo-db.ts`. | `2eda4236` |
| 5 | The three callers: `POST /v1/you/provision` (new), `/v1/you/github/sync` (re-exports it), `/v1/auth/cli/token` (provisions on the way through, response byte-for-byte unchanged). `github-sync-data.ts` deleted. | `b344da86` |
| 6·7 | `POST /v1/you/repositories` + `GET /v1/you/sync/plan`, both user-scoped. `repo-mapping.ts` reuses the daemon's `repo_key` rather than re-implementing the normaliser. | `acda527a` |

## Gates (all green as of `acda527a`)

`make test-db` → 5 files · `cd dojo && bun run test` → 129 files / 1407 · `bun run check` → 0 errors, 0 warnings / 1767 files · `dbd diff --scope dojo --exit-code` → in sync

## VERIFIED end-to-end against a real GitHub sign-in (2026-08-27)

The assumption carried since the slice began — does Supabase return
`provider_token`? — is **observed and true**. A real sign-in produced, from an
empty schema: `dojo.principals` ×1 · `dojo.identities` `github_oauth/293381742`
(GitHub's stable user id) · `dojo.tenants` `personal/sensei-hq-org` **and**
`organization/sensei-hq` · memberships `admin` on both · `tenant_connections`
`github/276295035` with `verified_at` set.

Idempotence proven on that real data: two further syncs, identical counts, no
duplicate key/membership/connection. The failure paths stay distinguishable —
`no_forge_token` vs `forge_unreachable` vs success — and both failures still
return the personal dōjō while inventing no org tenant.

**The token reaches the server via kavach's new `onSessionSync` hook**
(jerrythomas/kavach `040d34c`), not the cookie: `setCookieFromSession` keeps only
access/refresh, so `locals.session.provider_token` is structurally always null.
The hook uses the session the browser already POSTs — nothing extra persisted.

### Carry forward

- **`node_modules/kavach` is patched locally.** Publish `1.1.1` and repin before
  this deploys anywhere.
- `dojo.principals.display_name` is null (the hook resolves the principal before
  it has the forge profile). Harmless — names resolve from `dojo.identities` —
  but tidy it when convenient.

## Phase 2 (deliberately not built)

`claim_state` / `claimed_at` / `claimed_by`; `forge_visibility`; `seat_allocations` + `seat_release_reason`; billing; the full `can_sync`; de-provisioning. `ensureProvisioned` documents its three matching omissions in its header: it never removes, never overwrites an admin-overridden role, and every org tenant it creates is implicitly unclaimed.

## Facts worth not re-deriving

- **Postgres validates a `language sql` body at CREATE time** — a function calling a function is order-dependent. `dbd apply --scope dojo --dry-run` prints the real order. A `plpgsql` body is *not* validated.
- A policy that calls a function must live in `policies/`, not the table DDL. Moving the GRANT there too is safe — a bare `dbd apply` leaves `authenticated` with no grant at all, a loud failure rather than a silent leak.
- The Worker uses `service_role` and **bypasses RLS**, so a broken policy is invisible to every app-level test. Only a read as `authenticated` sees it.
- `psql` is at `/opt/homebrew/bin` (keg-only libpq; not always on PATH).
- Azure's SSH remote routes through a `v3` segment, so its org is the **second** path segment — `split('/')[1]` would map every Azure repo to an org called "v3".
