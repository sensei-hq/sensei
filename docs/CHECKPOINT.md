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

## NOT yet verified — the honest status

**Nothing has run against a real sign-in.** Every gate above is unit tests, type checks and SQL assertions. The end-to-end path — GitHub OAuth → `provider_token` → `ensureProvisioned` → a real `personal/jerry` row — has never executed. In particular:

- whether Supabase returns `provider_token` on the PKCE exchange is **still the one unobserved assumption in the design**. The code handles its absence (`no_forge_token`, visible, never silent), so a miss is loud rather than a lie — but it has not been seen either way.
- the dōjō DB still holds **1 tenant** (`organization/global-dojo`) and 0 principals / identities / memberships / connections / repositories.

Next command: sign in to the dōjō, then

```
psql -h 127.0.0.1 -p 54322 -U postgres -d postgres \
  -c "select key, origin from dojo.tenants"
```

— expect `personal/{login}` plus one tenant per GitHub org.

## Phase 2 (deliberately not built)

`claim_state` / `claimed_at` / `claimed_by`; `forge_visibility`; `seat_allocations` + `seat_release_reason`; billing; the full `can_sync`; de-provisioning. `ensureProvisioned` documents its three matching omissions in its header: it never removes, never overwrites an admin-overridden role, and every org tenant it creates is implicitly unclaimed.

## Facts worth not re-deriving

- **Postgres validates a `language sql` body at CREATE time** — a function calling a function is order-dependent. `dbd apply --scope dojo --dry-run` prints the real order. A `plpgsql` body is *not* validated.
- A policy that calls a function must live in `policies/`, not the table DDL. Moving the GRANT there too is safe — a bare `dbd apply` leaves `authenticated` with no grant at all, a loud failure rather than a silent leak.
- The Worker uses `service_role` and **bypasses RLS**, so a broken policy is invisible to every app-level test. Only a read as `authenticated` sees it.
- `psql` is at `/opt/homebrew/bin` (keg-only libpq; not always on PATH).
- Azure's SSH remote routes through a `v3` segment, so its org is the **second** path segment — `split('/')[1]` would map every Azure repo to an org called "v3".
