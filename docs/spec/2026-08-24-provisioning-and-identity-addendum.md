# Addendum — GitHub provisioning, identity, and where the worker runs

Extends `2026-08-24-shared-schema-and-sync-design.md` after reading
`docs/spec/dojo/dojo-auth-provisioning.md`. Design for discussion; no code or
DDL changed.

---

## 0. Decision recorded

**Q3 answered:** one repository belongs to exactly one project; a project may
hold many repositories. So:

```sql
create table project_repositories (
  project_id     uuid not null references projects(id)     on delete cascade
, repository_id  uuid not null references repositories(id) on delete cascade
, role           text
, primary key (project_id, repository_id)
, unique (repository_id)              -- ← the decision: one repo, one project
);
```

That makes `project_metrics` an unambiguous view and every project roll-up a
plain `sum`/`avg` over a disjoint set. It also means "which project is this
repo in" is a lookup, not a policy.

---

## 1. The auth doc is well-aligned — with three real gaps

Checked every field the Gherkin references against the DDL. `tenants.key/origin/
org/scope/dojo_url`, `memberships.role/kind/org_slugs/authenticated_via/
sync_status/disabled_at`, `identities.provider/subject/last_login_at` all exist,
and the enum values match (`tenant_origin` `github|org`, `membership_kind`
`employer|client|community|personal`, `member_role` `contributor|maintainer|lead|
admin`, `auth_method` `sso|github_oauth|device_code`, `sync_status` `healthy|
stale|error|authenticating`).

Three things the doc assumes that the schema cannot currently express.

### Gap 1 — `identities` is tenant-scoped, which blocks "one user, many dōjōs"

```sql
-- today
identities( tenant_id uuid NOT NULL references dojo.tenants(id), … )
constraint identities_provider_subject_unique unique (tenant_id, provider, subject)
```

The doc says (Scenario 4) *"the existing identity is matched by
`(provider=github_oauth, subject=<github_user_id>)`"* and (Scenario 22) *"the
`(provider, subject)` unique constraint on `dojo.identities` prevents
duplicates"*. **That constraint does not exist** — it is
`(tenant_id, provider, subject)`.

The consequence is the exact thing you want to avoid. One GitHub sign-in
provisions a personal tenant plus one per org — say 4 tenants — and therefore
**4 identity rows with identical `(provider, subject)`**, differing only by
tenant. "Which human is this" then has no single row to point at, and `user_id`
(the field that would tie them) has nothing to derive itself from on first
sign-in.

**Proposal: an identity is global; membership is what is tenant-scoped.**

```sql
-- identity = how a human proves who they are. Tenant-independent.
create table identities (
  id            uuid primary key default gen_random_uuid()
, user_id       uuid not null                  -- the human (Supabase auth user)
, provider      dojo.auth_method not null
, subject       text not null                  -- GitHub user id
, display_name  text
, created_at    timestamptz not null default now()
, last_login_at timestamptz
, unique (provider, subject)                   -- ← global, as the doc assumes
);
```

`tenant_id` is dropped. The tenant relation already lives on
`memberships(tenant_id, user_id)` with its own unique constraint — which is the
right place for it, and is what makes one login fan out to many dōjōs without
duplicating the identity.

This directly serves *"map user's login to a single user but multiple dojos"*:
one `identities` row, one `user_id`, N `memberships`.

### Gap 2 — there is no email alias model

`identities.email` is a single nullable `text`. Scenarios 6–12 need:

- a **list** of verified emails per user (Scenario 7: "updated with the full list of verified emails")
- **cross-provider matching** on any of them (Scenario 6: link a GitHub login to an existing user found by `work@acme-corp.com`)
- **removal tracking** (Scenario 10: "the email is marked as removed", which then disables a membership)
- **domain→org association** (Scenario 8)

```sql
create table identity_emails (
  user_id     uuid not null              -- the human, not the identity row
, email       citext not null
, verified    boolean not null default false
, is_primary  boolean not null default false
, source      dojo.auth_method not null  -- which provider asserted it
, linked_at   timestamptz not null default now()
, removed_at  timestamptz                -- soft: Scenario 10 needs the history
, primary key (user_id, email)
);
create unique index identity_emails_verified_unique
    on identity_emails(email) where verified and removed_at is null;
```

`citext` because email matching must be case-insensitive; the partial unique
index is what makes Scenario 6's "detect the email match" a lookup rather than a
scan, while still allowing the same address to reappear after removal.

Note the key is `user_id`, not `identity_id` — an email links a *human* across
providers, which is the whole point of Scenario 6.

**Domain→org association** (Scenario 8) then becomes a derived join rather than
stored state: `split_part(email,'@',2)` against a new `tenants.email_domains
text[]`. Storing the association would immediately go stale when either side
changes.

### Gap 3 — repos are mapped by a string array, which cannot hold what the doc needs

Scenarios 14–16 map repositories to tenants via `memberships.org_slugs text[]`.
But Scenario 14's own table carries per-repo data that an array cannot hold:

```
| repo         | visibility | access_level |
| backend-api  | private    | admin        |
| frontend-app | private    | write        |
| docs         | public     | read         |
```

`org_slugs` can say "this membership covers the acme-corp org". It cannot say
"this user has write on frontend-app but only read on docs" — which is exactly
what §5 of the main design needs for *"only repos accessible by the user get the
data"*.

So `org_slugs` stays as a **routing hint** and the entity arrives as designed:

```sql
create table repositories (            -- dojo side
  id          uuid primary key default gen_random_uuid()
, tenant_id   uuid not null references dojo.tenants(id)
, repo_key    text not null            -- github.com/org/repo — the global identity
, remote_url  text
, name        text not null
, visibility  text not null            -- 'public' | 'private'
, unique (repo_key)                    -- one repo, one row, one tenant
);

create table repository_access (       -- who may see what; drives RLS
  repository_id uuid not null references dojo.repositories(id) on delete cascade
, user_id       uuid not null
, access_level  text not null          -- 'admin' | 'write' | 'read'
, synced_at     timestamptz not null default now()
, primary key (repository_id, user_id)
);
```

`repository_access` is the table §5's RLS actually keys on. Deriving access from
membership alone would grant every org member every repo, which GitHub itself
does not do.

**Note the tension with Q3.** `repositories.repo_key` unique globally + one repo
one project means a repo cannot sit in two tenants. For a fork or a repo an
individual has under both a personal and an org account, that is a real
constraint. Flagged as Q8.

---

## 2. Provisioning as a pipeline (your "background processor")

This is a sixth pipeline for the task analysis, and it is the clearest possible
case for the coordinator/worker shape — the sign-in callback must not do it
inline. GitHub's repo list is paginated and an org with 500 repos is many
round-trips; holding an OAuth callback open for that is the broken-process
pattern from two commits ago.

| Stage | Task | Payload | Idempotency |
|---|---|---|---|
| Coordinate | `SyncGitHubIdentity` | `{ user_id, token_ref }` | one in flight per user |
| Derive | `LinkIdentityEmails` | `{ user_id }` | upsert on `(user_id, email)` |
| Derive | `ProvisionTenants` | `{ user_id, orgs[] }` | upsert on `tenants.key` |
| Ingest | `SyncOrgRepositories` | `{ user_id, org, cursor }` | upsert on `repo_key`; cursor resumes pagination |
| Derive | `SyncRepositoryAccess` | `{ user_id, repo_keys[] }` | upsert on `(repository_id, user_id)` |
| Reconcile | `ReconcileMemberships` | `{ user_id }` | disables per Scenarios 10–12 |

Every one is an upsert keyed on a natural identity, so Scenario 22's concurrent
sign-in is handled by the schema rather than by locking.

`SyncOrgRepositories` carrying a `cursor` is what makes a 500-repo org resumable
instead of an all-or-nothing job that times out.

**The token.** `token_ref` not the token itself — the payload lands in a task
row and a raw GitHub OAuth token in a queue table is a credential at rest in
plaintext. Store it in Supabase Vault / an encrypted column and pass the handle.
Scenario 21 (reduced scope) means the worker must also tolerate a token that can
no longer see orgs: preserve existing rows, log, do not delete.

---

## 3. The open architectural question: where does this worker run?

This is the one I cannot decide for you, because it is an infrastructure choice.

The provisioning work is **dojo-side** — it acts on a browser OAuth session, for
a user, writing `dojo.*` tables in Supabase. But **every worker we have is in
`senseid`**, which is a local daemon on one user's machine. It cannot act on
behalf of other users, and it is not running when the user signs in on the web.

Dojo has no general job runner today. `dojo.upstream_queue` is artifact-specific
(it has `artifact_id`, `kind dojo.artifact_kind`, `signature`) exactly as
`sensei.dojo_outbox` is memory-specific — neither is a queue we can reuse.

Three options:

**(a) Supabase Edge Function + `pg_cron`.** Provisioning rows land in a
`dojo.jobs` table; a scheduled function drains it. Native to the stack, no new
service, survives the browser closing. Costs: a second worker implementation in
TypeScript, and none of the queue/watchdog/retry/follow machinery we just built
in Rust.

**(b) A dojo-side Rust worker reusing `senseid`'s queue.** Same `Processor`
trait, same execution log, same follow API — one worker system, two deployments
(local daemon, hosted service). Costs: a service to host and a second DB target
(Supabase over the wire rather than local Postgres).

**(c) Do it inline in the auth callback, chunked.** Cheapest now, and wrong for
the same reason coverage backfill was wrong — it is unbounded work on a request
thread.

I lean **(b)**, because the whole point of the task-worker revamp is one
`Processor` abstraction with one execution log and one follow API. Reimplementing
coordinators, retries, watermarks, and progress in Edge Functions gives us the
second drifting copy this entire exercise is about eliminating. But (b) is real
infrastructure and (a) may be right if dojo is meant to stay serverless.

**This decision gates the sync design**, because pull-else-compute (§6 of the
main doc) needs a dojo-side endpoint that is authorized per repo — and whoever
runs provisioning naturally owns that endpoint too.

---

## 4. Updated open questions

Carried forward, plus new:

- **Q1** Project identity across machines — dojo assigns, local adopts? *(leaning yes)*
- **Q2** Repo sync default — private opt-in? *(leaning yes)*
- ~~Q3~~ **answered**: one repo, one project.
- **Q4** Who sees `scope='user'` metric rows — self + admins, or all members?
- **Q5** Materialize weekly/monthly, or keep as views? *(leaning views)*
- **Q6** Retired metric definitions — keep local history? *(leaning keep)*
- **Q7 (new)** Where does the provisioning worker run — (a) Edge Functions, (b) hosted Rust worker, (c) inline? *(leaning b)*
- **Q8 (new)** Can one `repo_key` exist in two tenants? Q3 plus a global unique on `repo_key` says no; forks and personal-vs-org duplicates say sometimes. If yes, the unique becomes `(tenant_id, repo_key)` and "which project" needs a tenant qualifier.
- **Q9 (new)** Auto-provisioned tenants are inactive until activated (doc Scenarios 3, 17, 18). Do their **repositories** sync before activation? I would say no — provision the rows, sync no metrics until a membership is active, otherwise we hold data for dōjōs the user never joined.

---

## 5. Revised DB sequencing

Identity work is independent of the metrics rename and can proceed in parallel.

**Local (senseid):**
1. Drop dead columns; `project_metrics` → `repository_metrics` + view.
2. `project_repositories` with `unique(repository_id)`; backfill from `folders`.
3. `visibility`/`synced_at`, `origin`/`shared_at`.
4. `sync_state`.

**Dojo:**
5. `identities` → global (drop `tenant_id`, unique `(provider, subject)`).
6. `identity_emails` + `tenants.email_domains`.
7. `repositories` + `repository_access`.
8. `metrics` (config) + `repository_metrics` + RLS.
9. A job table, once Q7 is decided.

5–6 are worth doing regardless — the current `identities` shape cannot represent
one human in several dōjōs, which is the premise of the whole onboarding flow.
