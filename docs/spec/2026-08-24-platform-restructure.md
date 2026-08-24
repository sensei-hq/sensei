# Platform restructure — root spec

One root document for three restructures that share a spine. Phased, and each
phase broken down per surface.

**Status:** design. No code or DDL changed yet.
**Detail docs** (evidence and rationale; this document is the plan of record):

- [`2026-08-24-task-worker-system-analysis.md`](./2026-08-24-task-worker-system-analysis.md) — worker inventory, pipelines, findings
- [`2026-08-24-shared-schema-and-sync-design.md`](./2026-08-24-shared-schema-and-sync-design.md) — metrics/repo/project DB layer
- [`2026-08-24-provisioning-and-identity-addendum.md`](./2026-08-24-provisioning-and-identity-addendum.md) — GitHub onboarding, identity gaps
- [`2026-08-24-consolidated-shared-schema.md`](./2026-08-24-consolidated-shared-schema.md) — the mirrored table set
- [`2026-08-24-adr-supabase-auth-and-sync.md`](./2026-08-24-adr-supabase-auth-and-sync.md) — **ADR**: Supabase identity linking, optional sensei login, and the git-attribution constraint
- [`dojo/dojo-auth-provisioning.md`](./dojo/dojo-auth-provisioning.md) — the Gherkin this is reconciled against

---

## 1. The three restructures

| | Workstream | Core change | Independent? |
|---|---|---|---|
| **A** | **Task/worker** | typed payloads, `Processor` trait, coordinator→worker, backfill as a parameter, log archival | Yes — ships alone |
| **B** | **Dōjō sign-in** | GitHub provisioning as local tasks, global identity, email aliases, tenants/teams | Needs A's task shape for the provisioning pipeline; needs C's identity tables |
| **C** | **Shared entity/config** | one vocabulary for scope/origin/owner across metrics, skills, agents, rules, memories, playbooks; mirrored local ⇄ dōjō tables | Partly independent; the local half ships without dōjō |

They share one spine: **a typed, scoped entity model plus a task system that can
carry it.** That is why they are one document and not three projects.

**Locked decisions**

- **Q3** One repository → exactly one project **per tenant**; a project holds many. *(Relaxed from a global unique by Q8 — see §3.1.)*
- **Q7** *All workers are local.* Dōjō has no job runner and is not getting one now; it holds the governance model for accepting pushes. The design keeps a dōjō-side worker possible (org-level consolidation across tenants is the plausible first case) but nothing depends on it.
- **Q11** *Teams are a real level — schema now, UI later.* `teams`, `team_members`, `team_projects` are created up front with a **default team containing everyone**, so nothing is blocked and no access migration is needed later; the team-management UI is provisioned when it is wanted. Access is granted per team, not per org. Tenant membership is billing and identity; **team membership is access**.
- **Q9** *Provisioned ≠ synced.* Tenants discovered from GitHub are created **inactive**. They are genuinely needed — a user may run sensei first and link a login later, and the tenant rows must already exist to attach to — but **nothing syncs until activation**. Activation is the gate, and it is tied to **entitlement/pricing**: the shared metrics and governance plane is a paid surface.
- **Q10** *Web sign-in is for admins and viewers only.* Dōjō consumes; it does not produce. Anyone who wants data — every developer — needs sensei locally, and dōjō should **recommend the download**. This closes the bootstrapping question entirely: there is no "web user with no install" gap to fill, because a web-only user is a viewer by design.
- **Identity comes from Supabase** (`auth.users` + `auth.identities`), not from tables we build. The profile (username, avatar) lives on the login only — it does not vary by tenant. Local `personas` group git emails and link to at most one login each. **Caveat:** Supabase auto-links identities sharing a verified email and this cannot be disabled — persona separation depends on disjoint emails. See ADR §2.1.
- **Q1** *Project identity:* local sensei **derives** it where it can; a user may combine projects on either side; **dōjō wins on conflict.** So the local uuid is provisional and dōjō's is canonical once registered.
- **Q2** *Sync is gated on authentication, not per-repo opt-in.* Once the user authenticates to dōjō **from sensei**, repos sync automatically both ways. No login → no connection → no sync. (This makes first login the consent moment: it must state plainly what will sync, because after it, every tracked repo's metrics do.)
- **Q4** *User-scoped metric visibility: **self + admins**.* Not the whole team — a teammate cannot see another member's individual numbers.
- **Never key data on `auth.users.id`.** A dōjō-side `principals` row is the stable identity our FKs reference; `auth_user_id` is a re-pointable pointer. This is what makes a merged account splittable later without losing history — see ADR §2.2.
- **Q13** *There is no dōjō Rust service. Dōjō is entirely Supabase-backed.* This voids the "Fork 1 / Supabase = auth ONLY" decision completely, and makes sensei→Supabase login the **only** transport rather than one option. See §3.2 for what it strands.
- **Q14** *Self-hosted dōjō is dropped*, but `dojo_url` stays **configurable** — a self-host is then just the same deployment at a different URL, with no second auth plane to maintain.
- **Q18** *Cross-tenant user-metric visibility is deferred* — acknowledged as needed, not blocking. Until decided the safe default holds: **home-tenant admins only**, client sees repo-scope aggregates.
- Mirrored schema: same table set both sides; locally `tenant_id` is nullable and filled on dōjō registration.

---

## 2. Cross-cutting: one vocabulary for shared entities

This is the part that makes C bigger than metrics. Skills, agents, rules,
memories and playbooks are all shareable entities, and **each invented its own
word for the same two ideas**:

```
memories.scope         = global | project          ← visibility
memories.origin        = authored | learned        ← provenance
rule_packs.source      = "OWASP · sensei", "Kent Beck · XP · sensei", …
                                                   ← a citation string, not a scope
playbooks.source       = builtin                   ← provenance
library_skills.source  = manifest                  ← provenance
library_agents.source  = manifest                  ← provenance
federated_memories     = (nothing)
```

`source` means three unrelated things across three tables, and none of them
expresses local-vs-remote.

### Two classes, not one

Before the vocabulary: **metrics are not the same kind of thing as skills and
rules**, and treating them alike was an error in the earlier draft.

| | **Code-backed catalog** | **Content entities** |
|---|---|---|
| Members | `metrics` | skills, agents, rule_packs, memories, playbooks, consolidated_rulesets, intake_guide |
| Who authors | **the product** (sensei/dōjō) only | anyone, at several scopes |
| Why | every metric has `task_name` binding it to a worker — **a tenant-authored metric would have no computation** | content needs no code to exist |
| Tenant control | **activation** (on/off) | scope + authorship |
| Sync | pulled, identical everywhere | scoped push/pull |

So there are **no local-only metrics**. The list is global and product-managed;
what a tenant chooses is which of them are *active*. Disabling one hides it
**and skips its computation** — a real saving, since the planner then enqueues no
task for it.

```sql
create table metric_activations (
  scope_id   uuid            -- tenant_id; NULL = this install's default
, metric_id  uuid not null references metrics(id) on delete cascade
, enabled    boolean not null default true
, updated_at timestamptz not null default now()
, primary key (scope_id, metric_id)
);
```

Today `active_metrics()` filters on `effective_from`/`effective_until` only —
that is the **product's** lifecycle (27 active, 2 retired of 29) and it stays.
Activation is a second, independent filter layered on top: product retirement
removes a metric for everyone; activation removes it for one tenant.

### The vocabulary (content entities only)

Applied to **every content entity**:

```sql
scope   entity_scope   -- 'local' | 'project' | 'tenant' | 'public'
origin  entity_origin  -- 'authored' | 'learned' | 'imported'
owner_person_id  uuid  -- set when scope='local'
owner_project_id uuid  -- set when scope='project'
owner_tenant_id  uuid  -- set when scope='tenant'
attribution text       -- the citation ("OWASP · sensei") — free text, kept, renamed off `source`
```

- **scope** answers *who may see it* and therefore *whether it syncs*.
- **origin** answers *where it came from* — needed to know what a re-import may overwrite.
- **attribution** keeps the human-readable credit that `rule_packs.source` holds today; it stops pretending to be an enum.

Entities taking this vocabulary: `memories`, `rule_packs`, `playbooks`,
`library_skills`, `library_agents`, `consolidated_rulesets`, `intake_guide`.
**Not `metrics`** — see the split above.

`public` is the marketplace tier (`marketplace/catalog.json`) — already a real
distribution channel, currently unmodelled in the DB.

---

## 3. The entity model

```
auth.users            THE LOGIN — profile lives here and nowhere else:
 │                    username, display name, avatar, primary email
 ├──< auth.identities  (github, google… Supabase links providers for free)
 │
 ├──< tenant_users    (tenant_id, user_id, role, kind)   ← membership ONLY, no profile
 │      one user → MANY tenants: personal dōjō + employer + each client
 │
 └──< team_members    (team_id, user_id, role)           ← ACCESS

tenant                        (GitHub org, or personal; NULL locally until registration)
 └── teams                    (a default team exists from creation)
      └── team_projects → projects     (mapping, so a project can span teams)
           └── repositories_in_projects → repositories   (1 repo → 1 project PER TENANT)
                                              └── repository_metrics

repositories                  ONE row per repo_key, globally
 └──< repository_tenants      (repository_id, tenant_id, is_owner)
        a repo may be linked to SEVERAL tenants; exactly ONE is the owner

principals (DŌJŌ)             THE STABLE IDENTITY every FK points at
 └── auth_user_id → auth.users   a re-pointable POINTER, not the key

personas (LOCAL ONLY)         one per working identity you keep apart
 ├── label                    'sensei-hq' | 'personal' | …
 ├── persona_emails           the git addresses that persona commits under
 └── principal_id             nullable, UNIQUE; at most one login per persona

skills / agents / rules / memories / playbooks   (scope + origin + owner_*)
```

**There is no `people` table.** An earlier draft had one with `tenant_id` and
`display_name`; both were wrong:

- `tenant_id` on a person is wrong because **one user belongs to many tenants** —
  personal dōjō plus employer plus each client. The tenant relation is
  `tenant_users`, a separate row per membership.
- `display_name`/avatar on a person duplicates the login. **A username does not
  change by tenant**, so the profile lives once on `auth.users` and every tenant
  reads the same one. Storing it per membership would let the same human render
  under different names in different dōjōs by accident.

`tenant_users` therefore carries **only** the relationship: role, kind, seat,
activation state. Nothing describing the human.

**And no FK points at `auth.users`.** They point at `principals`, whose
`auth_user_id` is a pointer we can re-aim. Supabase has no split-user operation —
`unlinkIdentity` deletes an identity and re-signing-in re-merges while the email
still matches — so the only way to undo an accidental account merge without
losing history is to own the indirection ourselves. ADR §2.2.

### 3.1 One repo, several tenants (Q8) — the consulting case

The driving scenario: an SG employee has access to a **client's** repo. The repo
should be reachable inside SG (with teams), but **ownership and management sit
with the client**, whose team may mix client members and SG members for
comparison — while other SG members see nothing.

```sql
create table repositories (              -- ONE row per repo, globally
  id        uuid primary key default gen_random_uuid()
, repo_key  text not null unique         -- github.com/client/api
, remote_url text
, name      text not null
);

create table repository_tenants (        -- the repo's reach
  repository_id uuid not null references repositories(id) on delete cascade
, tenant_id     uuid not null references tenants(id)      on delete cascade
, is_owner      boolean not null default false
, primary key (repository_id, tenant_id)
);
create unique index repository_one_owner
    on repository_tenants(repository_id) where is_owner;
```

**One repository row, N tenant links, exactly one owner** — rather than a copy of
the repo per tenant. Duplicating the row would duplicate its metrics and let the
two copies diverge, and there would be no answer to "which one is real".

- **Owner tenant** (the client) governs: metric activation, retention, who may be
  linked. `is_owner` is unique per repo, so governance is never ambiguous.
- **Linked tenant** (SG) gets *reach*, not control.
- **Access is still purely team-based**: an SG member who is not on the client's
  team sees nothing, because the path `principal → team_members → team → projects
  → repositories_in_projects` never reaches the repo for them. Being an SG
  employee grants nothing by itself.
- **Metrics are stored once** against `repository_id`, so a comparison across a
  mixed client/SG team is the same numbers, not two tenants' recomputations.

#### This modifies Q3

Q3 locked *"one repository → exactly one project"* with `unique(repository_id)`.
That is now **too strong**: the same repo may sit in the client's project *and* in
an SG project. The constraint relaxes to **one project per tenant**:

```sql
create table repositories_in_projects (
  project_id    uuid not null references projects(id)      on delete cascade
, repository_id uuid not null references repositories(id)  on delete cascade
, tenant_id     uuid not null references tenants(id)
, role          text
, primary key (project_id, repository_id)
, unique (repository_id, tenant_id)      -- ← was unique(repository_id)
);
```

Every roll-up is still unambiguous, because roll-ups are always evaluated within
one tenant. The property Q3 was protecting survives; only its scope narrows.

### 3.2 What Q13 strands — the transport that was never live

Dōjō is Supabase-backed end to end; **there is no Rust service**. That is not a
small correction, because a substantial amount of built code targets one:

```
crates/senseid/src/dojo/          6,080 LOC   DojoClient, contribute, relay_*
crates/dojo-protocol/             1,561 LOC   the wire types for that service
                                  ─────────
                                  ~7,600 LOC  targeting /v1/t/{tenant}/… — nothing serves it
14 files reference DojoClient      publish_segments, publish_run, collective/inbox,
                                   relay_drivers, agent_spawn, 3 API handlers
dojo/src/lib/triage-data.ts                    calls ${dojoApiUrl}/v1/t/{tenant}/triage
```

`collective/promote.rs` (the ~987-LOC promotion/k-anonymity engine the old build
plan describes) **is not in this tree at all** — it lived in the service.

**But it was never exercised.** Live local rows:

```
sensei.dojo_memberships   1     (the seeded global dōjō)
sensei.dojo_outbox        0     ← never sent anything
sensei.dojo_inbox         0     ← never received anything
```

So this is built-but-never-live code, not working code we would break. That
changes the framing of the sync workstream: **Phase 7 builds a transport, it does
not reshape an existing one.**

Three consequences for the plan:

1. **The ADR recommendation to "keep a governed write path in the service" is
   void** — there is no service to keep it in. Governance must live in RLS +
   constraints + Postgres functions, or in dōjō Edge Functions, or locally in the
   daemon. Anything genuinely needing a service (k-anonymity ≥3 across
   contributors, promotion scoring) has to be re-sited deliberately, not assumed.
2. **`dojo-protocol` and `dojo/client.rs` become retirement candidates**, along
   with the device-token plane and `dojo_outbox`/`dojo_inbox` (both empty). Worth
   a decision rather than leaving ~7.6k LOC pointed at nothing — dead code
   describing an architecture we no longer have is exactly what misled the
   earlier drafts of this spec.
3. **The relay features** (`relay_drivers`, `relay_project`, `relay_nudge`,
   `publish_segments`, `publish_run`) need triage: how much of each is local
   behaviour worth keeping versus outbound calls with no destination.

None of this blocks Phases 0–5, which are entirely local.

#### And it opens a real privacy question (Q18)

If Jerry (SG) commits to a client repo, his **user-scoped** metrics attach to a
repo the client owns. Q4 says user-scoped rows are visible to *self + admins* —
but **whose admins?**

- the **owner** tenant's admins (client sees an SG contractor's individual numbers), or
- the person's **home** tenant admins (SG sees them; client sees only repo-scope), or
- both

This is a contractual question as much as a technical one, and the answer changes
the RLS policy. Flagged as **Q18**; the safe default until decided is
**home-tenant admins only**, with the client seeing repo-scope aggregates.

**Personas are not aliases of one person, and must not be merged.** The local
identities measured here are two (or three) deliberate working identities, not
one human's duplicates:

```
me@jerrythomas.name             422 rows / 26 repos / 2019-06-25 → 2026-08-24
owner@example.com       84 / 2  / 2026-06-15 → 2026-08-21   ┐ personal
hi@sensei-hq.com                108 / 2  / 2026-07-26 → 2026-08-24   ┐ sensei-hq
dev@sensei-hq.com                74 / 1  / 2026-06-13 → 2026-08-21   ┘
dev@example-corp.com    62 / 9  / 2018-09-08 → 2026-03-20   ← employer, ended
contributor@example.com        17 / 1  / 2025-05-24 → 2025-06-25   ← may be another human
```

So the rule is:

- **Locally** — every persona is visible in one place, each row **tagged** with its
  persona label. Grouping is by persona, and "all of me" is a union the UI offers,
  never a merge the schema forces.
- **In dōjō** — personas are **separate logins**. sensei-hq work does not appear
  under the personal account and vice versa. That separation is a privacy
  boundary the user chose; the schema must make violating it impossible, not
  merely discouraged.

This corrects the earlier draft, which assumed one human = one person and would
have merged all six into a single contributor.

Metric-read authorization follows exactly one path, enforced by RLS:

```
auth.uid() → team_members → team → projects → repositories_in_projects → repository
```

GitHub's per-repo `admin|write|read` is used **only** to discover which repos to
create during provisioning — never to decide what a teammate can see.

**Attribution constraint (security).** A git commit email is an *unverified
assertion* — anyone can `git config user.email` to a colleague's address. Locally
that is harmless (own machine, own data). Once user-scoped metrics are pushed and
shown per person it becomes an attribution attack. So a shared `scope='user'` row
is keyed on the **authenticated** person (`auth.uid()`), never on the git email;
the git email travels as a property. Git aliases are **claimed**, never
auto-linked. Full reasoning in the ADR §3.

---

## 4. Surfaces

| Surface | What lives there |
|---|---|
| **DB** | `database/ddl` — local `sensei.*`/`activity.*`, and the mirrored `dojo.*` |
| **senseid** | Rust daemon: queue, workers, sync client, HTTP API |
| **dojo** | SvelteKit + Supabase: governance, RLS, read UI |
| **app** | desktop UI: metrics, follow/progress, settings |
| **CLI / MCP** | call surfaces that enqueue and follow |
| **marketplace** | skills/agents distribution (`public` scope) |

---

## 5. Phases

Each phase is shippable and forward-only; no phase depends on a later one.

### Phase 0 — Reclaim (A) · no behaviour change

The 1.5 GB win and the lies in the names. Nothing here needs a decision.

| Surface | Work |
|---|---|
| DB | Prune + roll up `activity.task_executions` (**4.8M rows / 1,568 MB / 69 days, nothing prunes it**); add `task_execution_daily`; make `task_kind` an enum (three rename orphans live in it today: `compute_metrics`, `plan_metric_days`, `resolve_edges`) |
| senseid | Retention worker; rename `TaskKind::ReconcileIdentity` → `ReconcileRepoMetadata` (**it reads repo frontmatter — the name will collide badly with Phase 3's identity work**) |
| app | none |

**Done when:** `task_executions` holds coordinator + failed rows only, workers roll up nightly, and `task_kind` cannot take an unknown value.

### Phase 1 — Typed payloads + `Processor` trait (A)

| Surface | Work |
|---|---|
| DB | `tasks.payload jsonb`; `trace_id` |
| senseid | `TaskPayload` enum replacing `folder_path`/`path`/`module_id`/`branch`/`url`; `Processor` trait carrying `KIND`/`PIPELINE`/`STAGE`/`BUDGET`; registry replaces the 35-arm match |
| CLI/MCP | enqueue wrappers take typed payloads |

**Why first among A:** every handler currently re-parses stringly-typed fields with its own convention — `folder_path` is variously a path, a capture-source name, and (in `BackfillCoverage`) a stringified week count. Backfill cannot become a parameter until there is somewhere typed to put it.

**Done when:** no handler parses `task.path`; a malformed payload fails at enqueue, not at run.

### Phase 2 — Coordinator → worker, backfill as a parameter (A)

| Surface | Work |
|---|---|
| DB | `pipeline_watermarks` replacing `metric_watermarks` + `transcript_cursor` |
| senseid | Per pipeline: one coordinator (what needs doing) + one worker (one unit, idempotent, updates its own watermark). Retire `Backfill*` kinds in favour of `Ingest*` with a `from` date |
| app | Backfill buttons post a range, not a special endpoint |
| CLI/MCP | `sensei backfill --repo X --from D` → the same coordinator |

Metrics already proves the model (`as_of: None` = today, `Some(d)` = that day). Coverage and transcripts each grew a second kind instead; this generalises the metrics case.

**Done when:** no task kind has "backfill" in its name and the same code path serves live and historical.

### Phase 3 — Personas and correct user metrics (C, local only)

Ships with **no dōjō dependency** and has immediate user-visible payoff.

| Surface | Work |
|---|---|
| DB | `personas` (id, label, `dojo_user_id` nullable), `persona_emails` (citext, soft-delete, partial unique); `repository_metrics.persona_id` |
| senseid | Resolve git author email → persona; roll-ups group by persona |
| app | Metrics tagged by persona; a "combined" view offered, not forced |

Six git identities today resolve to **two or three personas** (see §3), not one
person. The payoff is not merging — it is that "sensei-hq work" and "personal
work" become separable at all, which today they are not.

**Keep `identity` (the raw git email) and add `persona_id` as a resolved FK.**
Collapsing at write time would be destructive: each row came from
`git log --author=<email>`, so combining must *sum*, and a persona
reassignment later would need recomputation. Resolution stays a re-runnable
derivation over immutable raw attribution.

Unassigned emails default to their own persona rather than to a guessed one —
`contributor@example.com` may be a different human entirely, and quietly
folding it into yours would be a fabricated attribution.

### Phase 4 — Repository/project model (C, local only)

| Surface | Work |
|---|---|
| DB | `project_metrics` → `repository_metrics` (drop `project_id`, `folder_id`, `session_id`); `project_metrics` becomes a view; `repositories_in_projects` with `unique(repository_id)`; `folders.project_id` becomes derived |
| senseid | Writers target `repository_metrics` |
| app | reads unchanged (the view preserves the contract) |

**Nearly free:** measured 15,389 rows — `repository_id` NULL = 0, `folder_id` set = 0, `session_id` set = 0, repos in >1 project = 0, `project_id` disagreements = 0. A column drop and a view, not a migration.

### Phase 5 — Entity scope vocabulary (C, local only)

| Surface | Work |
|---|---|
| DB | `entity_scope`/`entity_origin` enums; apply `scope`/`origin`/`owner_*`/`attribution` to metrics, memories, rule_packs, playbooks, library_skills, library_agents, consolidated_rulesets, intake_guide; migrate `rule_packs.source` → `attribution` |
| senseid | One resolver for "what is in scope here", replacing per-entity logic |
| app | scope shown and editable per entity |
| marketplace | `public` scope maps to `catalog.json` |

### Phase 6 — Identity + tenants (B)

| Surface | Work |
|---|---|
| DB | `identities` **global** (`unique(provider, subject)`, `person_id` FK — today `dojo.identities` is `unique(tenant_id, provider, subject)`, so one sign-in provisioning four tenants makes four rows for one human with nothing tying them); `tenants`, `teams`, `team_members`, `tenant_members`, all with nullable `tenant_id` locally |
| senseid | Provisioning pipeline: `SyncGitHubIdentity` → `LinkPersonEmails` → `ProvisionTenants` → `SyncOrgRepositories { org, cursor }` → `ReconcileMemberships`. Token passed as `token_ref` (follow `dojo_memberships.credential_ref`: keychain, never Postgres) |
| dojo | Mirror tables; sign-in accepts a provisioned push rather than fetching |
| app | Dōjō linking UI; tenant activation (auth doc Scenarios 17–18) |

**Because all workers are local (Q7), the daemon is the provisioner**: it holds the token, walks GitHub (paginated, resumable via `cursor`), fills local tables, then pushes under dōjō's governance. Dōjō validates and accepts; it never fetches.

### Phase 7 — Sync + governance (B + C)

| Surface | Work |
|---|---|
| DB | `sync_state` (upsert per entity+key+direction, not a growing queue); `visibility`/`synced_at` on repositories; `origin`/`shared_at` on `repository_metrics` |
| senseid | Pull-else-compute: if the repo is shared and the metric is repo-scoped, take dōjō's row (`origin='dojo'`) and **do not recompute**; else compute (`origin='local'`) and push |
| dojo | RLS on the authorization path in §3; per-repo push authorization; `metrics` config becomes authoritative and pull-only |
| app | shows whether a number is local or shared |

`origin` is what stops the loop where local recomputes what it just pulled and pushes it back. `scope='user'` rows are always computed locally and pushed as values — **dōjō can show per-user metrics without ever holding a session, turn, or event.**

### Phase 8 — Activity consolidation (A)

Left last because it changes metric inputs.

| Surface | Work |
|---|---|
| DB | Merge `activity.turns` into `transcript_turns` → `activity.session_turns`; `session_id` text → uuid FK; orphan transcripts resolve to a repository by `repo_key` |
| senseid | One standardized `{sessions[], turns[], events[]}` persistence path for every adapter |
| app | unchanged |

Measured: 95% overlap (297 of 313 sessions have both), but **69 of 297 disagree on turn count** — a hook turn is prompt-to-prompt, a transcript turn is one exchange. They are two definitions, not duplicates, so the merge must pick one and date the change (Q12).

---

## 6. Dependencies

```
Phase 0 ──────────────────────────────► (independent)
Phase 1 ──► Phase 2 ──────────────────► Phase 8
Phase 3 ──► Phase 4 ──► Phase 5
Phase 3 ──► Phase 6 ──► Phase 7
Phase 1 ──► Phase 6            (provisioning needs typed payloads)
```

Phases 0, 3 and 4 need no decisions and no dōjō. They are the recommended start.

---

## 7. Open questions

| # | Question | Blocks | Leaning |
|---|---|---|---|
| ~~Q1~~ | **Answered:** local derives; users may combine either side; **dōjō wins on conflict** | — | — |
| ~~Q2~~ | **Answered:** auto-sync once authenticated from sensei; no login = no sync | — | — |
| ~~Q4~~ | **Answered:** **self + admins** only | — | — |
| ~~Q5~~ | **Answered:** start with **views**; materialize only if they measure slow | — | — |
| ~~Q6~~ | **Answered:** **keep** history for retired metrics (`effective_until` already models it) | — | — |
| ~~Q8~~ | **Answered:** yes — one repo row, N `repository_tenants` links, exactly one `is_owner`. **Relaxes Q3** to one project *per tenant* (§3.1) | — | — |
| ~~Q9~~ | **Answered:** provisioned inactive, **no sync until activation**; activation gated by entitlement/pricing | — | — |
| ~~Q10~~ | **Answered:** web = admins/viewers only; devs need sensei; dōjō recommends the download | — | — |
| ~~Q11~~ | **Answered:** teams/team_members/**team_projects** created now with a default team; UI later | — | — |
| ~~Q13~~ | **Answered:** no service exists — dōjō is Supabase-backed end to end (§3.2) | — | — |
| ~~Q14~~ | **Answered:** self-hosting dropped; `dojo_url` stays configurable | — | — |
| ~~Q18~~ | **Deferred** (needed eventually). Default meanwhile: home-tenant admins only | — | — |
| Q15 | Daemon JWT — restricted Postgres role, or full `authenticated`? | 7 | restricted |
| Q16 | Git-alias claiming — verified-email match only, or admin review? | 7 | never silent |
| Q17 | Persona email disjointness — rely on discipline + a loud `unique(dojo_user_id)` failure, or intercept the auth callback? | 6 | discipline + loud failure |
| Q12 | Post-merge turn definition: prompt-to-prompt or per-exchange? | 8 | *(shifts turn-counting metrics; must be dated)* |

**Answered:** Q1 (dōjō wins) · Q2 (auth-gated auto-sync) · Q3 (one repo, one project *per tenant*) · Q4 (self + admins) · Q5 (views first) · Q6 (keep retired history) · Q7 (all workers local) · Q8 (multi-tenant repos, one owner) · Q9 (inactive until activated) · Q10 (web = viewers) · Q11 (teams now, UI later) · Q13 (no Rust service) · Q14 (no self-hosting).

**Still open:** Q12 (post-merge turn definition) · Q15 (daemon JWT role) · Q16 (git-alias claiming) · Q17 (persona email disjointness).

**Deferred:** Q18 (cross-tenant user-metric visibility).
