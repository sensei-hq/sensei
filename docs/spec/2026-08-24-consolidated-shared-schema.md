# Consolidated shared schema — the table set, mirrored local ⇄ dojo

> **⚠ Partially superseded.** The plan of record is
> [`2026-08-24-platform-restructure.md`](./2026-08-24-platform-restructure.md).
> This document is retained for its **measurements and rationale**, which remain
> valid. Superseded since writing:
> - **No `people` table.** The login is Supabase `auth.users` and the profile
>   (username, display name, avatar) lives there only — a username does not change
>   by tenant. Membership is `tenant_users(tenant_id, user_id, role)`: one user
>   belongs to MANY tenants (personal + employer + clients), so the `people.tenant_id`
>   and `people.display_name` in this doc are both wrong.
> - `identities` → Supabase `auth.identities`; local git-email grouping →
>   **`personas`** (labelled), deliberately **kept apart** in dōjō rather than
>   merged. See root spec §3 and the ADR.
> - **No local-authored metrics.** The catalog is product-owned; tenants toggle
>   `metric_activations`.
> - Teams are a confirmed level, with a default team per tenant.

Supersedes the table sections of the two prior design docs. Design for
discussion; no DDL changed.

**Decisions locked so far**

- **Q3** one repository belongs to exactly one project; a project holds many.
- **Q7** *all workers are local.* Dojo has no job runner; it holds the governance
  model for push. The schema is designed so a dojo-side worker is *possible*
  later (org-level consolidation is the plausible first case), but nothing
  depends on one existing.
- Same table set on both sides. Locally `tenant_id` is **nullable** and stays
  null until dojo registration fills it in.

---

## 1. The measurement that changes the design

`repository_metrics.identity` is a **git commit email**, read from
`~/.gitconfig` (`git_identity.rs`) and used as `git log --author=<email>`. There
is no person model. Live data:

```
(repo-scope)                    14,630 rows   67 repos
me@jerrythomas.name                422 rows   26 repos
hi@sensei-hq.com                   108 rows    2 repos
owner@example.com          84 rows    2 repos
dev@sensei-hq.com                   74 rows    1 repo
dev@example-corp.com       62 rows    9 repos
contributor@example.com           17 rows    1 repo
```

At least five of those six are the same human. **"Metrics by user" in dōjō would
today show one person as five contributors**, and the numbers would be wrong in
both directions — fragmented per alias, and never summed.

So the email-alias table you wanted for auth is not only auth plumbing. It is
the join key that makes user-scoped metrics correct. That is the strongest
argument for adding `people` to your list.

---

## 2. Answer to "anything else?"

Your list, plus four:

| # | Table | In your list | Why |
|---|---|---|---|
| 1 | `tenants` | ✅ | org / dōjō |
| 2 | `teams` | ✅ | group within a tenant |
| 3 | `team_members` | ✅ | who is in the team |
| 4 | `projects` | ✅ | collection of repos |
| 5 | `repositories` | ✅ | keyed by `repo_key` |
| 6 | `repositories_in_projects` | ✅ | membership, `unique(repository_id)` |
| 7 | `metrics` | ✅ | the config |
| 8 | `repository_metrics` | ✅ | the values |
| 9 | **`people`** | ➕ | one row per human — without it §1 stays broken |
| 10 | **`person_emails`** | ➕ | git aliases + verified auth emails, one table |
| 11 | **`identities`** | ➕ | `(provider, subject)` → person; the auth link |
| 12 | **`tenant_members`** | ➕ | tenant-level seat/role, distinct from team access |

On 12: `dojo.memberships` already is this (tenant_id, user_id, role, kind,
seat). Teams sit *inside* a tenant, so both levels exist — tenant membership is
billing and identity, team membership is access. If you'd rather have one level,
teams collapse into tenants and `team_members` becomes `tenant_members`; I'd
keep both, because "everyone in the org can see every repo's numbers" is the
thing teams exist to prevent.

---

## 3. The hierarchy

```
tenant                     (GitHub org, or personal)
 ├── tenant_members        → people          (seat, role, billing)
 └── teams
      ├── team_members     → people          (access)
      └── projects
           └── repositories_in_projects → repositories   (1 repo → 1 project)
                                              └── repository_metrics
                                                       ↑
                                              metrics (config)
```

Authorization for metric reads follows exactly one path:

```
person → team_members → team → projects → repositories_in_projects → repository
```

This is why `repository_access` from the previous draft is **dropped**. GitHub's
per-repo `admin|write|read` is how provisioning *discovers* which repos to
create — it is not how metric visibility is decided. Conflating them would let
GitHub's ACL silently define what a teammate can see in dōjō.

---

## 4. Tier assignment

### T0 — local only, never syncs

`activity.*` in full (sessions, turns, transcript_turns, assistant_events,
snapshots, session_process_evidence, memory_loads, task_executions,
transcript_cursor) · `folders` · `folders_to_watch` · `folder_path_aliases` ·
`nodes` · `edges` · `symbol_names` · `communities` · `scan_state` ·
`sync_state` · `metric_watermarks`

Plus the one local link: **`folders.repository_id`**. Everything about *where a
repo lives on this machine* is local.

### T1 — dojo authoritative, pulled down

`metrics` (the config). A team must compute a metric identically or the
comparison is meaningless. **Superseded:** local-only metrics are not possible —
every metric is bound to a worker via `task_name`, so the catalog is
product-owned and a tenant controls only *activation* (`metric_activations`).
See root spec §2.

### T2 — mirrored, two-way

`tenants` · `teams` · `team_members` · `tenant_members` · `people` ·
`person_emails` · `identities` · `projects` · `repositories` ·
`repositories_in_projects` · `repository_metrics`

Same DDL both sides. That is what makes push a straight upsert rather than a
translation layer — and it is why `tenant_id` nullable locally works: an
unregistered install is simply the same schema with one column unset.

---

## 5. Table definitions (the changes that matter)

### `people` + `person_emails` — new

```sql
create table people (
  id            uuid primary key default gen_random_uuid()
, tenant_id     uuid references tenants(id)      -- NULL until dojo registration
, display_name  text
, primary_email citext
, created_at    timestamptz not null default now()
);

create table person_emails (
  person_id   uuid not null references people(id) on delete cascade
, email       citext not null
, verified    boolean not null default false     -- true only via auth provider
, source      text not null                      -- 'git' | 'github_oauth' | 'sso' | 'manual'
, linked_at   timestamptz not null default now()
, removed_at  timestamptz                        -- soft; auth doc Scenario 10 needs the history
, primary key (person_id, email)
);

create unique index person_emails_live_unique
    on person_emails(email) where removed_at is null;
```

`citext` because email matching must be case-insensitive. The partial unique
makes "does this email already belong to someone" a lookup, while still allowing
re-linking after removal.

One table serves both jobs: a `source='git'` row is a commit alias discovered
locally; a `source='github_oauth'` row is a verified address from sign-in. When
they collide, the same human is proven — which is auth doc Scenario 6, achieved
without a second mechanism.

### `identities` — global, not tenant-scoped

```sql
create table identities (
  id            uuid primary key default gen_random_uuid()
, person_id     uuid not null references people(id) on delete cascade
, provider      text not null              -- sso | github_oauth | device_code
, subject       text not null
, last_login_at timestamptz
, unique (provider, subject)               -- ← global, as the auth doc assumes
);
```

The current `dojo.identities` is `unique(tenant_id, provider, subject)`, so one
GitHub sign-in provisioning four tenants makes four rows for one human with
nothing tying them. Tenant scoping belongs on membership, which already has it.

### `repository_metrics` — the identity column

**Keep `identity` as the email. Add `person_id` as a resolved FK.**

```sql
  identity   citext         -- what git actually said; NULL for scope='repo'
, person_id  uuid references people(id)   -- resolved via person_emails; nullable
```

Deliberate: collapsing five alias rows into one `person_id` at write time would
be a destructive merge requiring recomputation (each was computed by
`git log --author=<email>`, so a merge must *sum*, not replace). Keeping the raw
attribution and resolving through `person_id` makes unification a **derivation**
— re-runnable, correctable when an alias is discovered later, and honest about
what git reported.

Roll-ups group by `person_id`, falling back to `identity` when unresolved. The
unique index gains nothing and stays as-is.

### `repositories`

```sql
  id          uuid primary key
, tenant_id   uuid references tenants(id)   -- NULL until registration
, repo_key    text not null                 -- github.com/org/repo
, remote_url  text
, name        text not null
, visibility  text not null default 'private'   -- syncs only when 'shared'
, dojo_id     uuid                          -- exists today, unused on all 67
, synced_at   timestamptz
, unique (repo_key, tenant_id)              -- see Q8 below
```

### `repositories_in_projects`

```sql
create table repositories_in_projects (
  project_id     uuid not null references projects(id)     on delete cascade
, repository_id  uuid not null references repositories(id) on delete cascade
, role           text                       -- 'primary' | 'library' | …
, primary key (project_id, repository_id)
, unique (repository_id)                    -- Q3: one repo, one project
);
```

`folders.project_id` becomes **derived** (folder → repository → project) rather
than authoritative. That removes a live drift class: today a folder can claim a
project that disagrees with its repository's.

### `metrics` — pull-only

```sql
  dojo_version bigint          -- product catalog version pulled down
-- NOTE: no `owner` column. The catalog is product-owned; per-tenant on/off
-- lives in `metric_activations(scope_id, metric_id, enabled)` — root spec §2.
```

Catalog rows replace wholesale on sync; activation rows are local state and are
never overwritten by a catalog pull. A
key collision resolves in dojo's favour with the local one renamed and flagged,
never silently dropped.

---

## 6. Provisioning, now that all workers are local

Q7 resolves the awkward part cleanly: **the local daemon is the provisioner.**
It holds the user's GitHub token, calls the API, fills local
`tenants`/`repositories`/`people`, then pushes to dōjō under dōjō's governance
rules. Dōjō validates and accepts; it never fetches.

Pipeline (all in `senseid`):

| Task | Payload | Idempotency |
|---|---|---|
| `SyncGitHubIdentity` | `{ token_ref }` | one in flight |
| `LinkPersonEmails` | `{ person_id }` | upsert `(person_id, email)` |
| `ProvisionTenants` | `{ orgs[] }` | upsert `tenants.key` |
| `SyncOrgRepositories` | `{ org, cursor }` | upsert `repo_key`; cursor resumes |
| `ReconcileMemberships` | `{ person_id }` | disables per Scenarios 10–12 |
| `PushRepositoryMetrics` | `{ repo_key, from }` | upsert on the metric identity |

`token_ref`, never the token — `dojo_memberships.credential_ref` already
establishes the pattern (device token in the OS keychain, never in Postgres).
Follow that.

**Gap this creates:** a user who signs into dōjō on the web without a local
sensei install has nothing to provision them. Options: dōjō does a minimal
inline provisioning (personal tenant only, no repo walk), or web sign-in is
gated on having a linked install. Q10 below.

---

## 7. Naming cleanup, consolidated

| From | To | Why |
|---|---|---|
| `sensei.project_metrics` | `repository_metrics` | already repo-grained; `project_id` not in its unique index |
| `activity.turns` | *(retired into `session_turns`)* | merged |
| `activity.transcript_turns` | `activity.session_turns` | source becomes a column |
| `activity.transcript_cursor` + `sensei.metric_watermarks` | `sensei.pipeline_watermarks` | one concept |
| `activity.task_sessions` | `activity.agent_sessions` | disambiguate from `sessions` |
| `TaskKind::ReconcileIdentity` | `ReconcileRepoMetadata` | **it reads repo frontmatter — nothing to do with user identity**; the name will collide badly with the identity work above |
| `TaskKind::Backfill*` | `Ingest*` | backfill is a parameter, not a kind |

---

## 8. Open questions

Still open:

- **Q1** Project identity across machines — dojo assigns id, local adopts? *(leaning yes)*
- **Q2** Repo sync default private opt-in? *(leaning yes)*
- **Q4** Who sees `scope='user'` rows — self + team, or self + admins only?
- **Q5** Materialize weekly/monthly or keep views? *(leaning views)*
- **Q6** Retired metric definitions — keep local history? *(leaning keep)*
- **Q8** Can one `repo_key` exist in two tenants? I have written
  `unique(repo_key, tenant_id)` above, which permits it — a consultant with the
  same repo under two clients, or a fork. If you'd rather forbid it, make
  `repo_key` globally unique and a repo can never be shared to two dōjōs.
- **Q9** Do inactive auto-provisioned tenants sync repos before activation? *(leaning no)*
- **Q10 (new)** Web sign-in with no local install — minimal inline provisioning, or gate on a linked install?
- **Q11 (new)** Are teams a real level now, or do we start with tenant-only and add teams when a tenant needs them? Starting tenant-only is less schema now, but `team_members` retro-fitted later means an access migration.

---

## 9. Sequencing

**Local, no dojo dependency — worth doing regardless:**

1. `project_metrics` → `repository_metrics`; drop `project_id`/`folder_id`/`session_id`; add view.
2. `people` + `person_emails`; backfill from the 6 known git identities; add `repository_metrics.person_id`, resolve.
3. `repositories_in_projects` with `unique(repository_id)`; backfill from `folders`; make `folders.project_id` derived.
4. Rename `ReconcileIdentity` → `ReconcileRepoMetadata` before the identity work makes it actively confusing.

**Registration-dependent:**

5. `tenants`/`teams`/`team_members`/`tenant_members` (nullable `tenant_id` everywhere).
6. `identities` global.
7. `visibility`/`synced_at`/`origin`/`shared_at`; `sync_state`.
8. Dojo mirror + RLS + push governance.

Step 2 is the one with immediate user-visible payoff: it makes "my metrics"
correct on this machine today, before any dōjō exists.
