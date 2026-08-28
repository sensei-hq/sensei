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
  the only gate the daemon owns — **superseded, see §8a**: it is the only gate the
  daemon owns for repositories where the USER holds authority, and not for
  org-mandated ones. Landed `d363f720`.
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
        ⚠ §8a B1: the cycle must ASK before filtering. Offering only the
          locally-shared set is what makes an org mandate unreachable, and
          an empty set currently returns before the dōjō is asked at all.
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

## 8a. REVISION — sharing is two questions, and the code answers one

> **Added 2026-08-28**, after `docs/requirements/repository-sharing.md`. This
> revises what was already SHIPPED, so it is written as a diff against reality
> rather than as a plan.

### What the shipped code assumes

Gate 1 is `sensei.repositories.visibility = 'shared'`, set by the user, and the
daemon treats it as sovereign. `dojo.all_my_repositories` hardcodes
`sync_enabled = true, denied_reason = null`. So the shipped model is: **the user
elects, and the dōjō permits everything.**

### What is actually required

Entitlement and election are independent (parent spec §IV.3, corrected), and
**election authority depends on the repository**:

| owner | forge visibility | authority |
|---|---|---|
| personal | private / public | user |
| organization | public | user |
| organization | private | **organization — mandatory** |

### What that changes, concretely

1. **`dojo.repositories` needs an election, separate from forge visibility.**
   Today it has `visibility` (`private | public`, the forge's answer, phase 1) and
   nothing recording whether sharing was elected or by whom. Two different things
   currently share one word in two schemas: `sensei.repositories.visibility` is
   `private | shared` (INTENT) and `dojo.repositories.visibility` is
   `private | public` (FORGE). Neither is an election record.

2. **Forge visibility must be captured at sign-in — and there is a chicken/egg.**
   `dojo.repositories` rows are created by `registerRepositories` and NOTHING ELSE
   (verified: `metrics-ingest.ts` only reads them). The daemon calls that only for
   locally-shared repos. So at sign-in there is **no row to write visibility onto**
   for precisely the repos this revision needs to reach — org-mandated ones the
   user has not elected locally. `forge-github.ts` also has no repo-listing call
   at all today (`fetchGithubUser`, `fetchGithubOrgs`, `fetchGithubFacts`).

   **REJECTED: have sign-in insert a row per visible forge repo.** It would create
   repository rows the daemon never registered and the user never chose to
   disclose — turning a sign-in into an inventory upload of every repo they can
   see. That is a worse privacy defect than the one this revision fixes, and it is
   sufficient on its own.

   *(An earlier version also cited the project's "never mint identity on a miss"
   rule. That citation was wrong — inserting rows from a successful authenticated
   listing is real data on a SUCCESS path, not fabrication on a failure path. The
   privacy argument does not need it.)*

   **RESOLVED: uncaptured is a state, and capture refreshes what exists.**
   - `registerRepositories` creates rows with visibility **NULL / uncaptured**.
     Authority fails closed on it: no authority → no election → no sync, with
     `refused_by = 'election'`, `reason = 'forge_visibility_unknown'`.
   - The sign-in path refreshes visibility for the rows that already exist, keyed
     on `repo_key` — a read of facts about repositories the user has already
     registered, not a listing of everything they can see.
   - **Cost, stated:** a newly registered repository is unshareable until the next
     sign-in captures its visibility. That lag is the price of not storing a forge
     token and not uploading an inventory. If it proves unacceptable, the fix is a
     narrowly-scoped refresh endpoint the console can call — not a stored token.

3. **Forge visibility is not populated today —
   every row sits at the `private` default, including `github.com/sensei-hq/dbd`,
   which is public. Registration cannot fetch it: the caller holds a SUPABASE
   token, not a forge token (`setCookieFromSession` strips `provider_token`, which
   is why provisioning reaches GitHub through the kavach `onSessionSync` hook). So
   capture happens where a provider token exists — the sign-in/provisioning path
   that already lists the user's orgs.

4. **`all_my_repositories` must compute, not hardcode.** `sync_enabled` becomes
   `can_sync AND elected`, and `denied_reason` must distinguish which of the two
   refused — plus, when election refused, WHICH AUTHORITY holds it. "off" that
   does not say whether the user or the org turned it off is the same
   indistinguishable-failure shape as "nothing to sync".

5. **Gate 1 stops being sovereign for org-mandated repos.** The daemon must offer,
   and push, org-mandated repos regardless of the local flag.

   **This is enforced in TWO independent code paths, and an earlier version of
   this section named neither.** Both are verified:

   - **B1 — the cycle short-circuits before the dōjō is ever asked.**
     `dojo_sync.rs`: `let shared = pg.shared_repositories(…); if shared.is_empty()
     { return Ok(()) }`. A new employee whose only repository is the org-mandated
     private one has nothing locally shared, so the pass returns before
     `register_repositories`, before `sync_plan`, before any push. **The feature is
     structurally defeated for exactly the population it exists to serve**, and no
     amount of dōjō-side correctness reaches it. The daemon cannot pre-filter on
     "is this mandated" either — it has no local data that answers that; only the
     plan does. So the cycle has to ask FIRST and filter after.
   - **B2 — the push query hardcodes gate 1 in SQL.** `unpushed_metric_rows`:
     `WHERE r.visibility = 'shared' … AND r.repo_key = ANY($2)`. Both. So even
     with registration and the plan corrected, a mandated repo's metric rows are
     still excluded at PUSH time. Fixing the offer side does nothing here.
     `unpushed_metric_count` carries the same predicate (reporting only).

   **The count was wrong twice.** First "three", then "at least eight". It is at
   least **TWELVE**, and the useful split is not by kind but by whether the site
   ENFORCES the rule or merely STATES it:

   **ENFORCING (must change or the feature does not work):** B1
   (`dojo_sync.rs` early return) · B2 (`unpushed_metric_rows` SQL) ·
   `unpushed_metric_count`'s identical predicate (reporting only).

   **STATING (must be corrected or they assert a falsehood):**
   `shared_repositories`' doc · `dojo_sync`'s module doc · §V.3 (now marked) ·
   §1 and §7 of THIS document (now marked) · `push_allowed`'s inline comment ·
   the `tests.rs` section banner · `repo_visibility.ddl`'s own comment ·
   `sensei/repositories.ddl`'s comment · `api/handlers/repositories.rs`' module
   doc (in the handler that WRITES gate 1) · `v1/you/repositories/+server.ts` ·
   and a **live passing assertion**:
   `"a repository the user did not share must NEVER be offered — that is gate 1"`,
   which must be rescoped or it will keep asserting a now-partly-false invariant.

   The two DDL comments matter more than their length suggests: they are the
   schema's published contract and survive into `\d+` and every generated schema
   doc.

### Status: NOT-READY as first written — and what closed the gap

A plan-depth review returned NOT-READY on the first version of this section. Its
two blocking findings were both real and both absent from my change list: **B1**
(the cycle returns before asking the dōjō when nothing is locally shared) and
**B2** (the push query hardcodes gate 1 in SQL, independently of the plan). I had
also undercounted the blast radius as three places when it is at least eight, and
given no schema for the election record while the sibling sections of this spec
family give full DDL sketches.

Closed above: B1 and B2 named, the count corrected, the election and org-policy
schemas sketched, the capture chicken/egg resolved with the speculative-row
approach explicitly rejected, and the denial vocabulary specified. Of the three
"not yet decided" items, two were load-bearing and are now decided; the third is
genuinely unreachable.

### BLOCKING ordering constraint — the default is unsafe for the new rule

Verified against the live dōjō before writing this, and it is not hypothetical:

```
github.com/sensei-hq/dbd  origin=organization  forge_visibility=private
                          → authority = ORG (MANDATED)
```

`sensei-hq/dbd` is **public on GitHub**. It reads `private` because that is the
column DEFAULT and nothing populates it. Under the new rule it therefore resolves
to org-mandated and would be shared **with no election by anyone** — the exact
inverse of the correct answer, which is "public, so the user decides".

The cause is one column serving two consumers whose safe directions are
**opposite**:

| consumer | safe default | why |
|---|---|---|
| entitlement | `private` | do not treat unknown code as free to host |
| **authority** | **`public`** | do not treat unknown code as org-mandated |

No single default is safe. So:

1. **"Not captured" must be a distinct state**, not a value that happens to mean
   something to both consumers — `NULL`, or an explicit `unknown` in the phase-2
   `dojo.forge_visibility` enum.
2. **Authority resolution must fail closed on an uncaptured repo**: no authority,
   therefore no election, therefore no sync — and a `denied_reason` that says
   `forge_visibility_unknown` rather than implying anyone chose anything.
3. **Capture must land before the authority rule does.** If the view is rewritten
   first, every existing org repository becomes silently mandated. That is a data
   leak on a deploy ordering, not on a code path, so it will not show up in any
   unit test.

Order, therefore: capture at sign-in → backfill existing rows → then the view.

### The schema, which was missing

An earlier version of item 1 said "`dojo.repositories` needs an election" and gave
no column names — while the sibling sections of this spec family (e.g.
`dojo.seat_allocations`) give full DDL sketches. Unbuildable as written. Proposed:

```
dojo.repository_elections            -- WHO chose, for WHOM, when
  tenant_id     uuid not null references dojo.tenants(id)     on delete cascade
  repository_id uuid not null references dojo.repositories(id) on delete cascade
  authority     dojo.share_authority not null   -- 'user' | 'organization'
  principal_id  uuid references dojo.principals(id) on delete cascade
        -- the electing user; NULL when authority = 'organization'
  elected       boolean not null
  elected_at    timestamptz not null default now()
  -- one election per (repo, authority, principal). A user's election and an org's
  -- mandate are DIFFERENT rows, so an authority change does not overwrite history.
  unique nulls not distinct (repository_id, authority, principal_id)

dojo.tenant_share_policy             -- the org's DEFAULT, so a new repo is covered
  tenant_id            uuid primary key references dojo.tenants(id) on delete cascade
  private_repos_shared boolean not null default false
        -- false, deliberately: an org that has not decided has not mandated.
  set_by               uuid references dojo.principals(id)
  set_at               timestamptz not null default now()

dojo.share_authority  enum ('user', 'organization')

-- REQUIRED by item 2's "uncaptured is a state": the column is `not null default
-- 'private'` today, so NULL is not writable. Without this the implementer hits a
-- not-null violation and falls back to 'private' — restoring the exact mis-default
-- the BLOCKING section proves leaks a public repo as org-mandated.
alter table dojo.repositories alter column visibility drop not null;
alter table dojo.repositories alter column visibility drop default;
-- and confirm the CHECK admits NULL (`visibility in (…)` is NULL-tolerant, but
-- assert it rather than assume).
```

`repository_elections` + `tenant_share_policy` together answer `elected()`: an org
mandate is the tenant policy unless a per-repo `authority='organization'` row
overrides it, and a user election is the row for that principal.

### Why the per-repo row and the tenant default are BOTH needed

A tenant-wide flag alone cannot express "share all private repos except this one",
which is the first thing any org asks for. A per-repo row alone means a newly
created repository is un-elected until someone touches it — so an org's mandate
silently fails to cover new work, which is the failure mode the mandate exists to
prevent.

### Still not decided — but no longer blocking the build

- **What happens to an election when authority changes.** A repo goes public →
  authority moves org → user. The unique key above keeps the two elections as
  separate rows, so the org's mandate does not silently become the user's choice
  and vice versa — which satisfies acceptance criterion 7's *"does not silently
  survive"* structurally. What remains undecided is whether the stale row is
  deleted, or kept and ignored. **Keeping it is the better default** (an audit
  trail of who mandated what, when) and nothing depends on the choice, so it is
  genuinely deferrable now.
- **Whether `internal` elects as private.** Not reachable: the column's CHECK
  allows only `private | public`, and the `dojo.forge_visibility` enum with
  `internal` is phase 2/3 (parent §V.1). Nothing in this slice can encounter it.

### The denial vocabulary, which criterion 5 needs

`denied[]` carries one opaque `reason` string today
(`user_plane.rs::DeniedRepo`, `repositories.ts`), and the view's comment
enumerates only entitlement reasons. Criterion 5 asks for entitlement-vs-election
AND which authority holds it, so the wire needs:

```
denied: [{ repo_key, tenant,
           refused_by: 'entitlement' | 'election',
           reason:     'unclaimed' | 'not_subscribed' | 'subscription_expired'
                     | 'no_seat'                          -- entitlement
                     | 'not_elected' | 'forge_visibility_unknown',   -- election
           authority:  'user' | 'organization' | null }]  -- who holds the election
```

`refused_by` is a separate field rather than an inference from `reason`, so a
reason added later cannot land on the wrong side of the split.

**D6 must be revisited as a prerequisite, not a phase-2 footnote.** It deferred
`denied[]` reason UX on the grounds that phase-1 `denied` is provably always
empty. That was true of a hardcoded view; it is false the moment the view
computes, and this revision is what makes it compute.

## 8b. Worked scenarios — how each case is configured and evaluated

The view returns **two independent verdicts**, never one boolean:

- `may_share` — ENTITLEMENT. *Is this repository allowed to be shared at all?*
- `elected`   — ELECTION. *Did whoever holds authority actually choose it?*
- `sync_enabled = may_share AND elected`

Keeping them apart is the whole point: "allowed but nobody chose" and "chosen but
not allowed" are different states with different fixes, and a single boolean
cannot tell a user which one they are in.

### The inputs

| input | where it lives | values |
|---|---|---|
| owner | `dojo.tenants.origin` | `personal` \| `organization` |
| forge visibility | `dojo.repositories.visibility` | `NULL` (uncaptured) \| `private` \| `public` |
| org default | `dojo.tenant_share_policy.private_repos_shared` | bool |
| per-repo election | `dojo.repository_elections` | `(authority, principal_id, elected)` |
| subscription | `dojo.billing_accounts.status` + period | `active` \| … |
| seat | `dojo.seat_allocations` | present \| absent |

### The scenarios

`authority` is derived, never stored on the repository:
`organization AND visibility <> 'public'` → **ORG**, else **USER**.

| # | owner | forge | subscribed | org policy | user elected | authority | `may_share` | `elected` | **sync** | reason shown |
|---|---|---|---|---|---|---|---|---|---|---|
| A | personal | private | — | — | ✅ | USER | ✅ | ✅ | **yes** | — |
| B | personal | private | — | — | ❌ | USER | ✅ | ❌ | no | election · `not_elected` · you |
| C | personal | public | — | — | ❌ | USER | ✅ | ❌ | no | election · `not_elected` · you |
| D | org | public | — | — | ✅ | USER | ✅ | ✅ | **yes** | — |
| E | org | public | — | on | ❌ | USER | ✅ | ❌ | no | election · `not_elected` · **you** |
| F | org | private | ✅ | on | ❌ | ORG | ✅ | ✅ | **yes** | — (mandated) |
| G | org | private | ✅ | off | ✅ | ORG | ✅ | ❌ | no | election · `not_elected` · **org** |
| H | org | private | ❌ *(row exists, `past_due`)* | on | — | ORG | ❌ | ✅ | no | entitlement · `not_subscribed` |
| **H2** | org | private | ❌ **no billing row at all** | on | — | ORG | ❌ | ✅ | no | entitlement · `not_subscribed` |
| I | org | private | ✅ | on | — | *none* | ❌ | ❌ | no | election · `forge_visibility_unknown` |
| J | org | private | ✅ | off + per-repo ON | — | ORG | ✅ | ✅ | **yes** | — (mandated, exception) |

### What each row is teaching

- **A/B/C — personal is simple.** Always entitled (`origin = 'personal' → ALLOW`),
  so the only question is whether the user chose. C matters: a *public* personal
  repo still does not sync unelected. Public means *free to host*, never
  *automatically shared*.
- **D/E — an org cannot elect its members' open source.** E is the row that would
  be wrong under a one-question model: the org policy is ON, the repo belongs to
  the org's tenant, and it still does not sync, because a public repo's authority
  is the USER. The org is not paying for open source and a contributor's metrics
  are their own.
- **F — the mandate, working.** The user has NOT elected it and it syncs anyway.
  Their local `sensei.repositories.visibility` is `private` and irrelevant. This is
  the row that requires B1 and B2 to be fixed; today the daemon returns before
  asking, and the push query excludes it.
- **G — a mandate cuts both ways.** The org said no, so the user *cannot* say yes.
  An individual may not publish the company's private code on their own
  authority. The user elected it and it still does not sync — and the reason names
  the ORG so they know who to ask.
- **H — a mandate is an election, not an entitlement.** The org mandated it and
  the subscription lapsed. `elected` is ✅, `may_share` is ❌, and the reason is
  `not_subscribed` — pointing at billing, not at the user. Collapsing the two
  axes would report this as "not shared" and send them hunting a toggle.
- **H2 — the row that was WRONG, and the reason `can_sync` now fails closed.**
  With **no** `billing_accounts` row — the state of all 3 live tenants — the
  original predicate DENIED nothing: `claimed_at` and `seat_allocations` do not
  exist, so those terms cannot fire, and `NULL <> 'active'` evaluates to NULL, not
  TRUE, so the status term does not fire either. Every path fell through to
  `otherwise → ALLOW`. Combined with the mandate supplying `elected = true`, an
  org's private repository would have pushed per-identity metrics **with no user
  election and no subscription whatsoever** — precisely the composite the mandate
  was meant to be gated by. §IV.3 now tests for the MISSING ROW before testing its
  value.
- **I — uncaptured fails closed on BOTH axes.** No authority can be derived, so
  there is no election to consult. This is the row that makes the ordering
  constraint safe: without it, `NULL`/default visibility in an org tenant would
  resolve to ORG-mandated and share everything.
- **J — the exception the tenant flag alone cannot express.** Policy off, one repo
  explicitly mandated. Also works in reverse: policy on with a per-repo
  `elected = false` excludes a single repository.

### The view

```sql
-- authority: derived, and NULL when we do not yet know the forge's answer
, case when r.visibility is null                                  then null
       when t.origin = 'organization' and r.visibility <> 'public' then 'organization'
       else 'user'
  end                                                           as authority

-- ENTITLEMENT
, case when r.visibility is null           then false   -- fail closed on unknown
       when t.origin = 'personal'          then true
       when r.visibility = 'public'        then true
       when b.status = 'active' and now() between b.period_start and b.period_end
             and sa.id is not null         then true
       else false
  end                                                           as may_share

-- ELECTION: the org's policy for ORG-authority rows, the user's for USER
, coalesce(
      case when t.origin = 'organization' and r.visibility <> 'public'
           then coalesce(oe.elected, p.private_repos_shared)   -- per-repo, else default
           else ue.elected                                     -- this principal's
      end, false)                                              as elected
```

`oe` is the per-repo `authority='organization'` election, `ue` the
`authority='user' AND principal_id = m.user_id` one, `p` the tenant policy, `b`
billing, `sa` the seat. `coalesce(…, false)` throughout: an absent row is *not
elected*, never *elected by default*.

### Why `refused_by` is a column and not an inference

Rows H and G both read `sync_enabled = false`. They need different actions — pay
the invoice, versus ask an admin. Row E needs a third (elect it yourself). A
consumer that has to infer which from a reason string will get it wrong the first
time a reason is added, so the view states it.

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
- **D8's public/private default.** The forge-visibility DATA EXISTS:
  `dojo.repositories.visibility` (`private | public`, text+CHECK) has been there
  since phase 1. What is missing is smaller and specific: **nothing populates it.**
  `registerRepositories` never sets the column, so every row sits at the `private`
  default — including `github.com/sensei-hq/dbd`, which is public on GitHub.
  Applying D8's default therefore reads `private` for every repository and shares
  nothing.

  Populating it needs a GitHub call, and the timing is the real question: at
  registration the caller holds a SUPABASE token, not a forge token
  (`setCookieFromSession` strips `provider_token`, which is why provisioning uses
  the kavach `onSessionSync` hook). So the value has to be captured where a
  provider token exists — sign-in/provisioning — or a token has to be stored.
  That is a decision, not an oversight.

  Phase 2 additionally promotes the column to a `dojo.forge_visibility` enum
  (`public | private | internal`) per the parent spec §V.1 — `internal` gates as
  private. The enum is what does not exist yet; the column does.
- **Two personas pushing the same repository.** `shared_at` is machine-global with
  no persona dimension, and the push `sync_state` mark is keyed on `repo_key`
  alone while the plan mark is keyed per persona. Done-gate item 5 is restated
  below to say what is actually observable.
