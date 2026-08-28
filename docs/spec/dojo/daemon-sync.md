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
- ~~`PgStore::unpushed_metric_rows(limit)` — the one production push path~~
  **Wrong.** It has no production caller at all — only tests — and there is no
  dōjō endpoint receiving metrics. There is therefore no existing push to gate on
  `plan.allowed`: building the push is a slice of its own, not a filter to add.
  This is why `tasks/dojo_sync.rs` establishes identity and entitlement and says
  so in its log, rather than appearing to sync and moving nothing.
- `dojo_client/session.rs` — per-persona Keychain session slots, `needs_refresh`.
- `dojo_client/dojo_auth.rs::refresh()` — `POST /v1/auth/cli/refresh`.
- `dojo/client.rs` — the **tenant-plane** client (per-membership device token).
  Untouched by this slice; artifacts are genuinely tenant-addressed.

## 2. Decisions taken

| # | decision | why |
|---|---|---|
| D1 | **Sync runs for EVERY signed-in persona**, not just `default` | `session.rs` was deliberately built for concurrent personas; syncing one silently strands the others. The registry already exists — see §3. |
| D2 | **`sensei.repositories.dojo_id` → renamed `tenant_id`, holds `dojo.tenants.id`** | A uuid fits the column and its documented meaning. `projects.dojo_id` holds a MEMBERSHIP id, so keeping the name would give one name two meanings. Requires `tenant_id` in the API responses (§5). |
| D3 | **Per-repo governance pull is IN scope** | §V.4 claims the daemon "pulls governance for allowed only". No per-repo pull exists — `resolved_pack_rules` is tenant/namespace-scoped. Building it makes the claim true rather than leaving it silently unimplemented. |
| D4 | **New `dojo_sync_scheduler`, config-driven interval, default hourly** | Matches `metrics_scheduler` (3600s, `metrics.interval_secs`). Daily would leave hourly-computed metrics unshared for up to a day. Key: `dojo.sync_interval_secs`. |
| D5 | **Plan every tick; register only when the shared set changed** | The plan must never be cached (§V.4) — that is the whole design. Repository identity rarely moves, so re-registering every tick is wasted work. |
| D6 | **Gate on `repo_key ∈ allowed`; no denial-reason handling yet** | Phase-1 `denied` is provably always empty: `all_my_repositories` hardcodes `sync_enabled = true`, `denied_reason = null`. Decoding a non-empty array must not crash, but building reason UX now would be speculative. |
| D7 | **A failed plan fetch is log-and-skip, recorded in `sensei.sync_state`** | Needs a new `sensei.sync_entity` value (§6) — none of the five existing values names a whole-cycle fetch. Without it there is no schema-legal `(entity, key)` to record against, and the failure would be invisible. |

## 3. Persona registry (D1) — WITHDRAWN, it already exists

> This section originally proposed a new `sensei.dojo_personas` table on the
> premise that *"sign-in state lives only in the Keychain and nothing can list
> it."* **That premise was false.** Checked against the code before building it;
> every field the table wanted already exists, so it was never created.

| §3 wanted | already is | evidence |
|---|---|---|
| `persona` (Keychain slot) | `sensei.personas.label` | `session.rs::account_for` formats `refresh_token.{persona}`; `auth.rs` passes that same string to `link_persona_identity`, which resolves it to a `personas` row |
| `dojo_url` | a **global** setting | `settings::dojo_url()` — env `DOJO_URL`, then local settings, then a default. Not per-persona |
| `signed_in_at` | `personas.verified_at` | `link_persona_identity` sets `verified_at = now()` on every completed OAuth callback |
| `last_sync_at` | `sync_state.synced_at` | keyed `(entity='dojo_sync_plan', entity_key=label)` — the entity value **D7 already adds** |

So the registry is a query, not a table:

```sql
select label from sensei.personas where verified_at is not null;
```

then a Keychain probe for a live token, because a row proves a sign-in
*happened*, not that its token is still good.

**Why not `principal_id is not null`**, which reads more precisely as "has a dōjō
login": nothing sets it yet — the column is documented "NULL until the user links
this persona (Phase 6)". Using it today would enumerate zero personas and sync
nothing, silently. `verified_at` is the predicate that is actually written.

`personas` also holds `is_self = false` rows for contributors who are not the
user. They are excluded in practice — only a completed OAuth callback sets
`verified_at` — but not filtered on explicitly, and deliberately so: if identity
resolution ever does match a sign-in to such a row, then someone *did* sign in as
it and there *is* a token in the Keychain. Syncing it is right; the wrong field
is `is_self`, and that is a correction to make there rather than a persona to
silently skip here.

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

Two, both in `sensei` — the third (§3's persona table) was withdrawn:

```
sensei.repositories  ~ dojo_id uuid  →  tenant_id uuid   -- D2
                       comment: the dojo.tenants.id this repository is
                       enrolled with. NULL = not federated.

sensei.sync_entity   + 'dojo_sync_plan'                  -- D7
```

The rename is safe to do wholesale: `repositories.dojo_id` has **three**
references in the tree — the column, its own comment, and a cross-reference in
`personas.ddl` — and no Rust or TypeScript reads or writes it. (The 1400-odd
other `dojo_id` hits are `projects.dojo_id`, which holds a MEMBERSHIP id and
deliberately keeps its name — that collision is the whole reason for D2.)

`sync_entity` gains one value rather than a table: a failed plan fetch is a sync
event like any other, and `sync_state` already carries `last_error`,
`attempted_at` and `synced_at` per `(entity, key, direction)`.

## 7. The cycle

```
for each persona in (select label from sensei.personas
                      where verified_at is not null):     § 3
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

## 9a. Claims (verified 2026-08-28, against the live `sensei` DB and the tree)

Every assertion this spec makes about what already exists, with the check that would disprove it.
Re-run before build — a claim verified three weeks ago is a claim about three weeks ago.

| # | claim | check | expect | actual | verdict |
|---|---|---|---|---|---|
| C1 | `unpushed_metric_rows` is the production push path | `rg -l 'unpushed_metric_rows' crates/ -g '*.rs'` minus tests/definition | ≥1 | **0** | **FALSE** (§1, already corrected) |
| C2 | `personas.principal_id` is unset, so user-scoped rows cannot be attributed | `select count(principal_id) from sensei.personas` | 0 | **0 of 3** | CONFIRMED |
| C3 | some repository has opted into sharing | `select count(*) from sensei.repositories where visibility='shared'` | ≥1 | **0 of 67** | **FALSE** |
| C4 | the daemon's push query already carries `scope`/`grain`/`props` | read the SELECT in `sync.rs` | present | absent — only `id, repo_key, key, computed_on, value` | **FALSE** |
| C5 | `dojo.repository_metrics` can absorb a re-push idempotently | read the unique index in its DDL | present | `unique (metric_id, repository_id, scope, principal_id, commit_sha, computed_on, grain)` | CONFIRMED |

### C3 is the one that would have cost a slice

**No repository is `shared`** — all 67 are `private`. `visibility='shared'` is gate 1, the local
intent gate, and nothing has ever set it. So:

- `shared_repositories()` returns empty → `dojo_sync` logs "nothing shared" and returns.
- `unpushed_metric_rows()` returns empty → the push moves nothing.
- **Every test still passes**, because each one supplies its own fixture.

Built without noticing, the push would have shipped, run green, pushed zero rows, and been
indistinguishable from working. This is not a bug in the default — `repositories.visibility` is
documented as private-by-default precisely so that signing in does not silently start sharing. It
is a missing *prerequisite*: there is no way for a user to mark a repository shared.

**Consequences for the slice:**

1. **Marking a repository shared is a prerequisite, not a follow-up.** Whatever surface does it —
   an API route, a CLI flag, the app — has to exist before the push can be verified at all.
2. **The done gate must observe a row land in the dōjō**, not merely that the push code ran. With
   gate 1 empty, "the cycle completed without error" is true of a cycle that did nothing.
3. **C4 means `unpushed_metric_rows` needs widening** before it can feed the ingest endpoint, which
   requires `scope`, `grain`, `props`, `commit_sha` and `source`.
4. **C2 confirms the user-scope deferral is real**, not cautious: 0 of 3 personas have a
   `principal_id`, so there is genuinely nothing to attribute a user-scoped row to. The ingest
   endpoint rejects `scope='user'` for exactly this reason.

## 9. Not in this slice

De-provisioning, claim, seats, billing, `forge_visibility` — all phase 2 of the
parent spec. `denied[]` reason UX (D6). Mirroring into `sensei.dojo_memberships`
(§II.8's older combined-round-trip scenario).
