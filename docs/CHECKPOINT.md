# Checkpoint

**Slice:** Dōjō auth & provisioning — phase 1 (`docs/spec/dojo/dojo-auth-provisioning.md`, Parts I–VIII)

## Done

- **Schema** — `dbd diff --scope dojo --exit-code` CLEAN on `127.0.0.1:54322`. `forge_provider`; `tenant_origin` → `personal|organization`; `tenants.org` → `slug`; `tenant_connections` (nullable `external_id`, two partial uniques). Commits `37ca9fab`, `78cd7808`, `47a726fb`, `25d2ba16`.
- **Part VIII** — deep review against the running code + live DB. Four decisions taken (below); F9–F12 recorded.

## Decisions (Part VIII — supersede earlier parts)

1. **Principal is the grain everywhere.** `resolveCaller` maps `sub → principals.auth_user_id → principal id`; `memberships.user_id` / `projects.user_id` hold principal ids. Chokepoint, so only 3 client-visible surfaces change.
2. **`POST /v1/you/provision`** — kavach owns the callback, so there is no web sign-in seam; the client calls this once after sign-in.
3. **Plan is user-scoped, in two calls** — `POST /v1/you/repositories` (register + map, reports `unmapped`) then `GET /v1/you/sync/plan` (entitlement filter). The tenant-scoped plan contradicted itself: `unmapped` cannot be reported per tenant.
4. **Fix the two dead paths inside this slice**, red-first, plus a live-Postgres test.

## Known broken (verified by running the statements)

| path | error | since |
|---|---|---|
| `createDojo` → `POST /v1/you/dojos` — **issue #117's own AC** | `column "org" of relation "tenants" does not exist` | `37ca9fab` (this slice) |
| every `dojo.identities` path (4 routes + members screen + incidents) | `column "user_id" does not exist` | `75565304` (pre-existing) |

`bun run test` is **exit 0, 1328 tests** over both. The specs stub the Supabase client and assert the payload the code *sends* — no dōjō test touches Postgres, so none can fail on schema drift.

## Next — phase 1 work list (Part VIII.7), in order

1. RLS fix on `dojo.projects` (principal-resolving policy — currently `user_id = auth.uid()`, which would silently match nothing)
2. `resolveCaller` sub→principal; `ensureProvisioned`
3. Repair `createDojo`; 4. Repair the `dojo.identities` paths
5. `POST /v1/you/provision` + `/v1/auth/cli/token` (no response reshaping)
6. `POST /v1/you/repositories` (`repo_key → provider/org`; reuse the Rust `normalize_repo_key`, do **not** re-implement it)
7. `GET /v1/you/sync/plan`; 8. a live-Postgres test

Next command: start item 1 red-first.

## Open

- Unobserved: whether Supabase returns `provider_token` on the PKCE exchange. A dōjō sign-in confirms it and is the first end-to-end test (the reset dropped `personal/jerry`).

## Environment (verified 2026-08-27)

`develop` clean · dōjō DB `127.0.0.1:54322` reachable, 1 tenant (`organization/global-dojo`), 0 principals/identities/memberships/connections/repositories, 10 `auth.users` · kavach 1.1.0 (double-resolve bug fixed, `/v1` POST bodies intact)
