# Checkpoint

**Slice:** Dōjō auth & provisioning — phase 1 (`docs/spec/dojo/dojo-auth-provisioning.md`, ~1400 lines, Parts I–VII)

## The hole being closed

Nothing creates a tenant — **zero inserts into `dojo.tenants` anywhere**. `syncGithubMemberships` joins only pre-existing tenants ("never invents a tenant"), is wired only to `POST /v1/you/github/sync` rather than sign-in, and that endpoint silently no-ops because `session.provider_token` exists only immediately after the OAuth exchange.

## Done — schema, verified live

`dbd diff --scope dojo --exit-code` **CLEAN** on local Supabase (`127.0.0.1:54322`).

- `dojo.forge_provider` — `github | gitlab | bitbucket | azure_devops`
- `dojo.tenant_origin` → `(personal | organization)`, declared as target (pre-release, we reset — no transitional labels)
- `dojo.tenants.org` → **`slug`** (+ staging, `import_tenants`, seed)
- `dojo.tenant_connections` — `external_id` **nullable**, two partial uniques: `(provider, external_id)` where known keeps one proven forge org to one tenant forever; `(provider, lower(external_slug))` where NULL stops unproven slug races
- seed → `organization/global-dojo`; orphaned `dojo.metrics` pruned

Commits `37ca9fab`, `78cd7808`, `47a726fb`, `25d2ba16`.

## Design decisions worth not re-deriving

- **A tenant is an ORGANIZATION**, not a forge org. Same slug across forges is never evidence of same org — linking is an authorized act by one human authenticated on both sides who already administers the tenant.
- **Key is `<origin>/<slug>`** (`personal/jerry`, `organization/sensei-hq`). No `@` sigil — the origin prefix already separates the namespaces and all 33 `/v1/t/[origin]/[org]/` routes keep their shape.
- **Three gates:** intent (`sensei.repo_visibility='shared'`, daemon-local) → cost (forge visibility, dōjō) → entitlement (claim + billing + seat, dōjō).
- **The daemon asks, never caches.** `GET /v1/t/{tenant}/sync/plan` → `{allowed[], denied[{repo_key, reason}]}`. No TTL, offline degrades to no-sync by construction. sensei's schema delta across *all* phases is one display-only column.
- **Seats split**: participation (existing `dojo.seats`, `(user, namespace)`, observed) vs entitlement (new `seat_allocations`, `(tenant, user)`, admin-granted, current+past by row). Dissolves the circularity.
- **Claim decoupled from admin** — losing a claim does not remove admin, or an org gets locked out of its own dōjō.

## Next — phase 1 remainder (TypeScript, not started)

1. `ensureProvisioned(userId, forgeToken, provider)` — idempotent, wired to all three callers (web sign-in callback, `POST /v1/auth/cli/token`, `POST /v1/you/github/sync`)
2. repo→tenant mapping by remote URL — normaliser must handle github/gitlab/bitbucket **and both Azure forms**
3. `GET /v1/t/{tenant}/sync/plan` returning everything shared as `allowed`

Phase 2 (entitlement): `claim_state`, `forge_visibility`, `seat_allocations`, billing, full `can_sync`, de-provisioning.

## Blocked on Jerry

**A dōjō sign-in.** The reset dropped `personal/jerry`, so signing in is both the first end-to-end test and the confirmation that Supabase returns `provider_token` on the PKCE exchange — the last unobserved assumption. The daemon side is verified wired (`store_provider_token` → keychain, `canReadOrgs` in `/api/auth/status`) and has better durability than the web session.

## Lifecycle note

`reconcile` is a **pre-release stopgap**. `dbd release` (once) sets `released: true`, disables reconcile, writes the baseline; then `make bump` snapshots and `dbd deploy` migrates v(n)→v(n+1). Two findings survive the cutover: dbd cannot express a column rename (plans DROP+ADD), and a data migration must move in lockstep with its seed (apply→import gives the seed the last word — observed as two `global-dojo` tenants).

## Environment a new session inherits (verified)

| | state |
|---|---|
| branch | `develop` @ `bf829e6c`, clean, nothing unpushed, 1 worktree |
| daemon | 0.9.1, release build, running |
| dōjō DB | `postgresql://postgres:postgres@127.0.0.1:54322/postgres` — reachable, **1 tenant** (`organization/global-dojo`; `personal/jerry` was dropped by the reset, by design) |
| sensei DB | `localhost:5432/sensei` — reachable, 338 sessions |
| issue | **#117** (F3a self-serve create a dōjō) under epic **#116** |

To re-verify the schema after resuming:

```
cd database
export DATABASE_URL="postgresql://postgres:postgres@127.0.0.1:54322/postgres"
dbd diff --scope dojo --exit-code     # expect CLEAN
```

## Also landed today (unrelated)

- **#125** Zed ingest root-caused and fixed (`b63ac861`) — the retention pruner deleted turns and left watermarks behind, self-sealing. Repaired live; analyzable pool **109/287 → 274/338**.
- **Thematic retrospectives** spec + stage inference shipped, measured on 86 real sessions.
- **History redacted** across all three public repos; single identity; leak guard in pre-commit.
