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
- `PgStore::unpushed_metric_rows(scopes, limit)` — the push queue. **When this
  spec was written it had NO production caller** (only tests) and no dōjō endpoint
  received metrics, which is why the first version of this bullet called it "the
  one production push path" and was wrong. Both halves now exist:
  `tasks/dojo_sync.rs::push_allowed` is the caller and
  `POST /v1/you/metrics` is the endpoint (§5).
- `dojo_client/session.rs` — per-persona Keychain session slots, `needs_refresh`.
- `dojo_client/dojo_auth.rs::refresh()` — `POST /v1/auth/cli/refresh`.
- `dojo/client.rs` — the **tenant-plane** client (per-membership device token).
  Untouched by this slice; artifacts are genuinely tenant-addressed.

## 2. Decisions taken

| # | decision | why |
|---|---|---|
| D1 | **Sync runs for EVERY signed-in persona**, not just `default` | `session.rs` was deliberately built for concurrent personas; syncing one silently strands the others. The registry already exists — see §3. |
| D2 | **`sensei.repositories.dojo_id` → renamed `tenant_id`, holds `dojo.tenants.id`** | A uuid fits the column and its documented meaning. `projects.dojo_id` holds a MEMBERSHIP id, so keeping the name would give one name two meanings. Requires `tenant_id` in the API responses (§5). |
| D3 | ~~Per-repo governance pull is IN scope~~ **DEFERRED — see §9.** Not built; the cycle is token → gate 1 → register → plan → push. | §V.4 claims the daemon "pulls governance for allowed only". No per-repo pull exists — `resolved_pack_rules` is tenant/namespace-scoped. Building it makes the claim true rather than leaving it silently unimplemented. |
| D4 | ~~New `dojo_sync_scheduler`, config-driven interval~~ **SUPERSEDED by `docs/spec/daemon/schedules.md` step 5.** No such module and no `dojo.sync_interval_secs` key exist: the cadence is a `sensei.schedules` row. | Matches `metrics_scheduler` (3600s, `metrics.interval_secs`). Daily would leave hourly-computed metrics unshared for up to a day. Key: `dojo.sync_interval_secs`. |
| D5 | ~~register only when the shared set changed~~ **NOT IMPLEMENTED.** The cycle POSTs the full shared set every tick. The write is idempotent so it is correct, but at a 60s cadence it re-registers up to 500 repositories a minute — the wasted work D5 existed to prevent. | The plan must never be cached (§V.4) — that is the whole design. Repository identity rarely moves, so re-registering every tick is wasted work. |
| D6 | **Gate on `repo_key ∈ allowed`; no denial-reason handling yet** | Phase-1 `denied` is provably always empty: `all_my_repositories` hardcodes `sync_enabled = true`, `denied_reason = null`. Decoding a non-empty array must not crash, but building reason UX now would be speculative. |
| D7 | **A failed plan fetch is log-and-skip, recorded in `sensei.sync_state`** | Needs a new `sensei.sync_entity` value (§6) — none of the five existing values names a whole-cycle fetch. Without it there is no schema-legal `(entity, key)` to record against, and the failure would be invisible. |
| D8 | **Sharing is configured explicitly; within that step, public repos default ON and private repos default OFF (subscription-gated)** | See §2a. Resolves "why is nothing shared" (claim C3) without making sign-in start sharing. |

### 2a. What sharing is FOR, and what the default is (D8)

The push exists so the dōjō has something to be useful about. Locally, sensei already sees a
single person's own metrics — publishing them changes little. **The value appears at two or more
people on one repository**, where "me vs the rest" becomes a comparison, and where governance and
insight sharing — the core proposition — have anyone to share with.

So the default should be ON where it is cheap and useful. But "signing in must not silently start
sharing" still holds. Both are satisfied by putting the default **inside an explicit configuration
step** rather than at sign-in:

| repository | default once configured | gated by |
|---|---|---|
| public on the forge | **shared** | nothing — the code is already public; the user may still turn it off |
| private | **off** | **subscription.** Unsubscribed, no metrics, governance or insights are shared at all |

The distinction that matters: this is a **default within a decision the user makes**, not an
inference drawn from forge visibility behind their back. Nothing flips at sign-in. The user opts
into sharing, and at that moment the public/private split decides what the sensible starting
position is — and remains changeable per repository, along with the push cadence (which is now a
`sensei.schedules` row, so "how often" is the same mechanism as everything else).

Private-repo sharing is where billing enters (phase 2 of the parent spec: claim, seat, billing).
Until then a private repository stays local-only regardless of configuration.

**Verification order, which is also the done gate.** Start from the real initial state and change
one thing at a time, so each step's effect is observable:

1. All repositories `private`, `dojo_sync` schedule disabled — confirm the cycle pushes nothing
   and says so, rather than erroring.
2. Set one repository to `shared`. Confirm gate 1 now yields it.
3. Enable the `dojo_sync` schedule and set its cadence.
4. Let it run. Confirm a full cycle: register → plan → push → the row observable in
   `dojo.repository_metrics`.
5. Re-run. Confirm idempotence — the same row updates rather than duplicating, and
   `shared_at` advances so the next cycle does not re-push unchanged rows.

## 3. Persona registry (D1) — WITHDRAWN, it already exists

> This section originally proposed a new `sensei.dojo_personas` table on the
> premise that *"sign-in state lives only in the Keychain and nothing can list
> it."* **That premise was false.** Checked against the code before building it;
> every field the table wanted already exists, so it was never created.

| §3 wanted | already is | evidence |
|---|---|---|
| `persona` (Keychain slot) | `sensei.personas.session_slot` | **NOT `label`.** `session.rs::account_for` formats `refresh_token.{slot}` from the string the sign-in was started with; `link_persona_identity` REWRITES `label` to the verified GitHub login, so signing in as `default` yields a row labelled `sensei-hq-org` whose session is still at `refresh_token.default`. The first version of this table claimed they were the same string — they are not, and looking the session up by label silently skipped the persona while the cycle reported success. `session_slot` records what the sign-in actually used. |
| `dojo_url` | a **global** setting | `settings::dojo_url()` — env `DOJO_URL`, then local settings, then a default. Not per-persona |
| `signed_in_at` | `personas.verified_at` | `link_persona_identity` sets `verified_at = now()` on every completed OAuth callback |
| `last_sync_at` | `sync_state.synced_at` | keyed `(entity='dojo_sync_plan', entity_key=session_slot)` — the entity value **D7 already adds** |

So the registry is a query, not a table:

```sql
select session_slot from sensei.personas
 where session_slot is not null and verified_at is not null;
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

**Five, across two schemas.** This section said "two, both in `sensei`" — false,
and the omitted one was the column that fixed §3's own error, so an auditor
looking for the label/slot fix found no trace of it here.

```
sensei.repositories  ~ dojo_id uuid  →  tenant_id uuid   -- D2
                       comment: the dojo.tenants.id this repository is
                       enrolled with. NULL = not federated.

sensei.sync_entity   + 'dojo_sync_plan'                  -- D7

sensei.personas      + session_slot text                 -- §3, the label/slot fix
                     + personas_session_slot_unique (partial)
                       The Keychain slot the sign-in used. NOT the label, which a
                       sign-in rewrites to the verified login. Looking the session
                       up by label skipped the persona while reporting success.

sensei.metrics       + grant select to authenticated, service_role
                       So the dojo view below can be read. service_role is the one
                       that matters — the Worker holds that key.

dojo.metric_catalogue  NEW VIEW over sensei.metrics (id, key)
                       The sanctioned cross-schema read: `sensei` is deliberately
                       unexposed to PostgREST, so a dojo view qualifies it
                       internally, as dojo.rule_pack_library already does.

dojo.repository_metrics ~ unique (…) → unique NULLS NOT DISTINCT (…)
                       Without it the constraint fired for NOTHING we push (every
                       row has principal_id NULL). See C5.
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
for each slot in (select session_slot from sensei.personas
                   where session_slot is not null
                     and verified_at is not null):        § 3
    token = live_access_token(persona)          § 4 — skip persona on failure
    shared = shared_repositories()               gate 1, local
    if shared changed since last register:       D5
        POST /v1/you/repositories
        store tenant_id per repo                 D2
        log unmapped[]                           D6
    plan = GET /v1/you/sync/plan                 every tick, never cached
        on failure → mark_sync_error(dojo_sync_plan, persona) and SKIP    D7
    push unpushed_metric_rows WHERE repo_key ∈ plan.allowed
    (D3: pull governance for plan.allowed — DEFERRED, see §9)
```

## 8. Done gate

- [ ] a persona with no shared repos syncs nothing and errors nothing
- [ ] a shared, mapped repo's metrics reach the dōjō; an `unmapped` one's do not
- [ ] a repo whose `visibility` is flipped to `private` stops syncing on the next tick
- [ ] a failed plan fetch leaves `sync_state` with `state = 'error'` and pushes nothing
- [ ] two signed-in personas each fetch a plan and each push only what their plan allows, and
      neither stalls the other (D1). NOT "both push the same rows": `shared_at` is machine-global,
      so a repository pushed by one persona is not re-pushed by the other
- [ ] an expired persona is reported signed-out and does not stall the others

## 9a. Claims (re-verified 2026-08-28, AFTER the slice shipped)

Every assertion this spec makes about what already exists, with the check that would disprove it.

> **The ledger drifted inside a single day, which is the lesson.** Its first version was written at
> `f94dfdb0` (11:16) and three of its five verdicts were falsified by `9468acd0` (11:47) and
> `cb48d354` (11:59) — the commits that FIXED the claims it recorded as false. A reader hours later
> saw "C3: 0 of 67, FALSE / marking a repository shared is a prerequisite" and would have rebuilt a
> route that already shipped. **Re-run the checks; do not trust the date in the heading.**

| # | claim | check | expect | actual | verdict |
|---|---|---|---|---|---|
| C1 | `unpushed_metric_rows` has a production caller | `rg -l 'unpushed_metric_rows' crates/ -g '*.rs'` minus the definition and tests | ≥1 | **1** — `tasks/dojo_sync.rs` | CONFIRMED *(was FALSE: 0)* |
| C2 | `personas.principal_id` is unset, so user-scoped rows cannot be attributed | `select count(principal_id) from sensei.personas` | 0 | **0 of 3** | CONFIRMED |
| C3 | some repository has opted into sharing | `select count(*) from sensei.repositories where visibility='shared'` | ≥1 | **1 of 67** (`github.com/sensei-hq/dbd`) | CONFIRMED *(was FALSE: 0 of 67 — the prerequisite shipped as `PATCH /api/repositories/{*repo_key}`)* |
| C4 | the push query carries `scope`/`grain`/`props`/`commit_sha`/`source` | read the SELECT in `sync.rs` | present | **present** | CONFIRMED *(was FALSE: 5 fields absent)* |
| C5 | `dojo.repository_metrics` can absorb a re-push idempotently | **insert the same repo-scoped row twice and expect a unique violation** | rejected | **rejected** | CONFIRMED *(was recorded CONFIRMED on a check that could not establish it — see below)* |

### C5 was recorded CONFIRMED and was FALSE

The original check was *"read the unique index in its DDL"*. An index existed, so it was marked
CONFIRMED. **Reading that a constraint exists does not establish that it fires.**

`unique (metric_id, repository_id, scope, principal_id, commit_sha, computed_on, grain)` defaults to
NULLS DISTINCT, and every repo-scoped row the daemon pushes carries `principal_id = NULL` (day-grain
rows also carry `commit_sha = NULL`). So the constraint applied to **nothing we send**. Proven
against Postgres 17: two byte-identical inserts → 2 rows.

Idempotence therefore rested entirely on a non-atomic select-then-insert in TypeScript, and the test
that "proved" it passed because `fakeDojoDb` compares with `===`, treating `null === null` as equal —
**the fake was STRICTER than Postgres**, which is the dangerous direction for a test double to err.

Fixed by declaring the constraint `unique nulls not distinct (…)`, matching
`sensei.repository_metrics`, which had the clause and the comment explaining why all along. The
check in the table above is now the assertion that would have caught it: insert twice, expect
rejection.

**The lesson for the ledger format:** a check that reads a declaration is weaker than one that
exercises behaviour. Prefer "do X and expect Y" over "read Z".

### C3's original finding, kept for the record



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

**Deferred, not dropped — each was a decision above that the code does not honour,
and saying so here is the point:**

- **D3, the per-repo governance pull.** The cycle does not pull governance. §7's
  last line is struck until it does. D3 existed to stop the parent's §V.4 claim
  being "silently unimplemented" — leaving D3 asserted made it unimplemented in
  two documents instead of one.
- **D5's register-only-when-changed guard.** Needs a `sync_state` row over a hash
  of the shared set.
- **D8's public/private default.** Currently *unimplementable*, not merely
  unimplemented: `sensei.repositories` has no forge-visibility column, so nothing
  can decide "is this public". §2a describes the intended rule; the only setter
  that exists requires an explicit value and rejects an absent one. No
  configuration step exists to apply a default in.
- **Two personas pushing the same repository.** `shared_at` is machine-global with
  no persona dimension, and the push `sync_state` mark is keyed on `repo_key`
  alone while the plan mark is keyed per persona. Done-gate item 5 is restated
  below to say what is actually observable.
