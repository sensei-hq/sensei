# Daemon ↔ dōjō sync — the plan consumer

> **Status:** plan. Written because a depth review of the daemon work against
> `dojo-auth-provisioning.md` returned NOT-READY: six areas were underspecified
> or contradicted, and several were schema gaps rather than missing prose.
>
> **Relationship to the parent spec.** `dojo-auth-provisioning.md` is authoritative
> for the dōjō side. Where it describes the DAEMON it is partly stale — Part VIII
> corrected the plan endpoint to a user-scoped shape but did not edit §II.8/§V.4
> in place. This document is authoritative for the daemon, and §0 lists every
> statement it overrides.

## 0. What this overrides in the parent spec

| parent | says | actually |
|---|---|---|
| §V.4 code block | `GET /v1/t/{tenant}/sync/plan` | `GET /v1/you/sync/plan` — user-scoped (§VIII.1 F9) |
| §VIII.1 illustration | `unmapped: ["gitlab.com/acme/x"]` | `unmapped: [{repo_key, reason}]`, four reasons — the shipped, tested shape in `dojo/src/lib/server/repositories.ts` |
| §II.8 gherkin | one combined round trip on connect, mirroring into `sensei.dojo_memberships` | two separate calls; no membership mirroring in this slice |
| §V.5 | "sensei: consume the plan… No schema change at all" | one rename + one enum value (§4, §6). The claim was true of the *plan* mechanism, not of identity storage |

## 1. What the daemon already has

- `PgStore::shared_repositories(limit)` → `SharedRepo { repo_key, remote_url, name }`,
  filtering `visibility = 'shared' AND repo_key IS NOT NULL`. **Gate 1 (intent)** —
  the only gate the daemon owns. Landed `d363f720`.
- `PgStore::unpushed_metric_rows(limit)` — the one production push path, already
  filtered on gate 1. It does **not** yet consult the dōjō's `allowed[]`.
- `dojo_client/session.rs` — per-persona Keychain session slots, `needs_refresh`.
- `dojo_client/dojo_auth.rs::refresh()` — `POST /v1/auth/cli/refresh`.
- `dojo/client.rs` — the **tenant-plane** client (per-membership device token).
  Untouched by this slice; artifacts are genuinely tenant-addressed.

## 2. Decisions taken

| # | decision | why |
|---|---|---|
| D1 | **Sync runs for EVERY signed-in persona**, not just `default` | `session.rs` was deliberately built for concurrent personas; syncing one silently strands the others. Requires a registry (§3) — nothing enumerates Keychain entries today. |
| D2 | **`sensei.repositories.dojo_id` → renamed `tenant_id`, holds `dojo.tenants.id`** | A uuid fits the column and its documented meaning. `projects.dojo_id` holds a MEMBERSHIP id, so keeping the name would give one name two meanings. Requires `tenant_id` in the API responses (§5). |
| D3 | **Per-repo governance pull is IN scope** | §V.4 claims the daemon "pulls governance for allowed only". No per-repo pull exists — `resolved_pack_rules` is tenant/namespace-scoped. Building it makes the claim true rather than leaving it silently unimplemented. |
| D4 | **New `dojo_sync_scheduler`, config-driven interval, default hourly** | Matches `metrics_scheduler` (3600s, `metrics.interval_secs`). Daily would leave hourly-computed metrics unshared for up to a day. Key: `dojo.sync_interval_secs`. |
| D5 | **Plan every tick; register only when the shared set changed** | The plan must never be cached (§V.4) — that is the whole design. Repository identity rarely moves, so re-registering every tick is wasted work. |
| D6 | **Gate on `repo_key ∈ allowed`; no denial-reason handling yet** | Phase-1 `denied` is provably always empty: `all_my_repositories` hardcodes `sync_enabled = true`, `denied_reason = null`. Decoding a non-empty array must not crash, but building reason UX now would be speculative. |
| D7 | **A failed plan fetch is log-and-skip, recorded in `sensei.sync_state`** | Needs a new `sensei.sync_entity` value (§6) — none of the five existing values names a whole-cycle fetch. Without it there is no schema-legal `(entity, key)` to record against, and the failure would be invisible. |

## 3. Persona registry (D1)

Sign-in state lives only in the Keychain and nothing can list it. Add a
**`sensei.dojo_personas`** table: one row per persona the daemon has signed in,
written on a successful `/v1/auth/cli/token` exchange and cleared on sign-out.

It stores no secret — the tokens stay in the Keychain. It exists purely so an
unattended task can answer "who is signed in?", which is currently unanswerable.

```
sensei.dojo_personas
  persona       text primary key   -- the Keychain slot name
  dojo_url      text not null
  signed_in_at  timestamptz not null default now()
  last_sync_at  timestamptz
```

## 4. A live access token

Extract the refresh/rotate logic currently inline in
`api/handlers/auth.rs::status` (lines ~234–320) into

```rust
pub async fn live_access_token(persona: &str) -> Result<String, AuthError>
```

`status` then calls it too, so there is one implementation rather than two that
drift. On a 401/403 the stored session is cleared and the persona is reported
signed-out — an expired persona must not silently stall the whole cycle.

## 5. The user-plane client

New `dojo_client/user_plane.rs`. Bearer = the Supabase access token from §4, not
a device token.

```
POST /v1/you/repositories   { repos: [{ repo_key, remote_url, name }] }
  → { mapped:   [{ repo_key, tenant, tenant_id, repo_id }],
      unmapped: [{ repo_key, reason }] }        reason ∈ unknown_host
                                                       | no_connection
                                                       | ambiguous
                                                       | not_a_member
GET  /v1/you/sync/plan
  → { allowed: [{ repo_key, tenant, tenant_id, repo_id }],
      denied:  [{ repo_key, tenant, reason }] }
```

**`tenant_id` is added to both responses by this slice** (the
`all_my_repositories` view already carries it) so the daemon can satisfy D2.

## 6. Schema changes

Two, both in `sensei`:

```
sensei.repositories  ~ dojo_id uuid  →  tenant_id uuid   -- D2
                       comment: the dojo.tenants.id this repository is
                       enrolled with. NULL = not federated.

sensei.sync_entity   + 'dojo_sync_plan'                  -- D7
```

`sync_entity` gains one value rather than a table: a failed plan fetch is a sync
event like any other, and `sync_state` already carries `last_error`,
`attempted_at` and `synced_at` per `(entity, key, direction)`.

## 7. The cycle

```
for each persona in sensei.dojo_personas:
    token = live_access_token(persona)          § 4 — skip persona on failure
    shared = shared_repositories()               gate 1, local
    if shared changed since last register:       D5
        POST /v1/you/repositories
        store tenant_id per repo                 D2
        log unmapped[]                           D6
    plan = GET /v1/you/sync/plan                 every tick, never cached
        on failure → mark_sync_error(dojo_sync_plan, persona) and SKIP    D7
    push unpushed_metric_rows WHERE repo_key ∈ plan.allowed
    pull governance for plan.allowed             D3
```

## 8. Done gate

- [ ] a persona with no shared repos syncs nothing and errors nothing
- [ ] a shared, mapped repo's metrics reach the dōjō; an `unmapped` one's do not
- [ ] a repo whose `visibility` is flipped to `private` stops syncing on the next tick
- [ ] a failed plan fetch leaves `sync_state` with `state = 'error'` and pushes nothing
- [ ] two signed-in personas both sync (D1) — the case a single-persona design silently drops
- [ ] an expired persona is reported signed-out and does not stall the others

## 9. Not in this slice

De-provisioning, claim, seats, billing, `forge_visibility` — all phase 2 of the
parent spec. `denied[]` reason UX (D6). Mirroring into `sensei.dojo_memberships`
(§II.8's older combined-round-trip scenario).
