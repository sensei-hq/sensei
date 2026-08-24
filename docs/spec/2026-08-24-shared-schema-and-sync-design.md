# Shared schema + local/dojo sync — DB layer design

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

Status: **design for discussion**, no code or DDL changed. Companion to
`2026-08-24-task-worker-system-analysis.md`; that one covers workers, this one
covers what we change at the database level first.

Evidence from the live daemon DB, 2026-08-24.

---

## 1. What the data already tells us

Four measurements that decide most of the design:

```
sensei.project_metrics       15,389 rows
  repository_id IS NULL           0      ← already 100% repo-grained
  folder_id set                   0      ← dead column
  session_id set                  0      ← dead column
  grain: daily                15,389     ← 'session' enum value never used
  scope: repo                 11,813
  scope: user                  3,576     ← both scopes live; this IS the dojo split

repos mapped to >1 project          0    ← project is derivable from repo
metric rows whose project_id
  disagrees with folders map        0    ← project_id is pure redundancy

sensei.repositories                67
  repo_key set                     67    ← every repo has a global identity
  remote_url set                   67
  dojo_id set                       0    ← sharing seam exists, unused
```

**`project_metrics` is misnamed, not mis-modelled.** Its unique index is already
`(metric_id, repository_id, scope, identity, commit_sha, computed_on, grain)` —
`project_id` is not in it. The rename you proposed is mostly a column drop.

`repo_key` is a normalized remote (`github.com/org/repo`), identical for every
user of that repo on every machine, derivable with no central registry. That is
the join key the entire sync hangs on.

---

## 2. The sharing boundary

Three tiers. This is the core decision; everything else follows from it.

### T0 — Local only, never leaves the machine

Raw activity and anything naming a path on disk.

`activity.sessions` · `activity.turns` · `activity.transcript_turns` ·
`activity.assistant_events` · `activity.snapshots` ·
`activity.session_process_evidence` · `activity.memory_loads` ·
`activity.task_executions` · `activity.transcript_cursor` ·
`sensei.folders` · `sensei.folders_to_watch` · `sensei.nodes` · `sensei.edges` ·
`sensei.symbol_names` · `sensei.communities` · `sensei.scan_state`

Your rule, stated exactly: *sessions, turns, events are user only and never
shared*. The code graph joins them — it is derived from local checkouts and
carries file paths, so it stays local too.

### T1 — Shared config, dojo is authoritative (dojo → local)

`sensei.metrics` — the metric registry (key, name, formula, rating_scale,
weight, target, cadence, direction, derives_from …).

This is the table your first sentence was about. It is already a clean config
table with a unique `key`, effective-dated (`effective_from`/`effective_until`)
and versioned by `modified_at`. Making it shared means: **dojo owns the
definition, local pulls it.** A team must compute the same metric the same way
or the comparison in dojo is meaningless.

**Superseded:** an earlier draft allowed a local/private metric. It cannot exist —
`metrics.task_name` binds every metric to a worker, so a user-authored metric would
have no computation. The list is product-owned; a tenant toggles *activation*.
See the root spec §2.

### T2 — Shared data, two-way (local ⇄ dojo)

`sensei.repositories` · `sensei.projects` · `sensei.project_repositories` (new) ·
`sensei.repository_metrics` (renamed)

A project is *a collection of repos*, shared by the team. A repository is
identified by `repo_key`. The **only** local part is which folder on this
machine happens to be that repository — `folders.repository_id`, which stays T0.

---

## 3. Table-by-table changes

### 3.1 `project_metrics` → `repository_metrics`

```
DROP  project_id     -- derivable via project_repositories; 0 disagreements today
DROP  folder_id      -- 0 rows, and a folder is local; a shared metric cannot key on it
DROP  session_id     -- 0 rows; session-grain is local-only by the T0 rule
KEEP  metric_id, repository_id, scope, identity, commit_sha,
      computed_on, grain, value, props, source, modified_at
ADD   shared_at timestamptz    -- null = not yet pushed to dojo
ADD   origin metric_origin     -- 'local' | 'dojo'  (who computed this row)
```

`project_metrics` becomes a **view** (your guess was right):

```sql
create view project_metrics as
select pr.project_id, rm.*
  from repository_metrics rm
  join project_repositories pr on pr.repository_id = rm.repository_id;
```

Safe because no repo maps to more than one project today. If that ever changes,
the view fans out and the roll-up must `sum`, not `avg` — worth a constraint
decision now (§7 Q3).

**`origin` is load-bearing.** It answers "did I compute this or did dojo hand it
to me", which is what makes pull-else-compute idempotent instead of a loop where
local recomputes what it just pulled and pushes it back.

### 3.2 `scope` vs `grain` — these are being conflated

You wrote *"grain at session, repo, day, week"*. Those are two different axes and
the current schema already separates them correctly:

| axis | meaning | values today | proposed |
|---|---|---|---|
| `scope` | **whose** numbers | `repo`, `user` | unchanged |
| `grain` | **time bucket** | `session`, `daily` | `daily`, `weekly`, `monthly` |

`repo` and `user` are scopes, not grains. `session` is currently a *grain* value
that has never been used (0 rows) and cannot be shared anyway — a session is T0.

Proposal: drop `session` from `metric_grain`, add `weekly`/`monthly` **only if**
we intend to materialize them. Today week/month are roll-up views that re-derive
from daily (correctly — never average-of-averages). I would keep it that way and
leave `grain` as `daily` alone unless you want materialized weeklies for dojo
push efficiency.

### 3.3 `repositories` — the shared identity

```
KEEP  id (local uuid), repo_key (global identity), remote_url, name
KEEP  dojo_id                    -- currently 0 rows; becomes the dojo-side uuid
ADD   visibility repo_visibility -- 'private' | 'shared'   (does this repo sync at all)
ADD   synced_at timestamptz
```

`repo_key` is the natural key for sync; `id` stays a local uuid so nothing local
has to change. Dojo keys its side by `repo_key` too, so two users of the same
repo converge without coordination.

### 3.4 `projects` + new `project_repositories`

Today a project's membership is expressed **only** through `folders.project_id`
— a local, path-bound table. That cannot be shared. The membership needs its own
shared table:

```sql
create table project_repositories (
  project_id     uuid not null references projects(id)     on delete cascade
, repository_id  uuid not null references repositories(id) on delete cascade
, role           text                                       -- 'primary' | 'library' | …
, primary key (project_id, repository_id)
);
```

`folders.project_id` then becomes derived (folder → repository → project) rather
than authoritative, which also removes a real class of drift: today a folder can
claim a project that disagrees with its repository's project.

`projects` also needs a shared identity — `id` is a local uuid and two teammates
would generate different ones for the same project. Options in §7 Q1.

**Not shared from `projects`:** `root_abs_path` (a local path). Everything else
(name, description, goal, stack, links, guidelines, backlog, icon) is team
content and shares fine. `privacy` stays local and governs the rest.

### 3.5 `metrics` (config) — becomes pull-only

```
ADD   scope_owner text     -- 'dojo' | 'local'
ADD   dojo_version bigint  -- monotonic; local refuses to overwrite a newer local edit
```

Pull semantics: dojo rows land with `scope_owner='dojo'` and are replaced
wholesale on each sync. (Superseded: there are no local-authored metric rows —
see root spec §2. Retained text below described the abandoned model.) Local rows are never touched by
sync. A key collision resolves in dojo's favour with the local one renamed and
flagged, never silently dropped.

### 3.6 New: `sensei.sync_state`

The outbox/inbox that exists today (`dojo_outbox`/`dojo_inbox`) is
**memory-specific** — it has `memory_id` and `batch_id` columns and a
memory-share signature. It is not a general channel. Rather than overload it:

```sql
create table sync_state (
  entity      sync_entity not null   -- 'metric_def' | 'repository' | 'project'
                                     -- | 'project_repository' | 'repository_metric'
, entity_key  text        not null   -- repo_key, metric key, or a composite
, direction   sync_direction not null -- 'push' | 'pull'
, local_version  bigint
, remote_version bigint
, state       text not null default 'pending'
, last_error  text
, updated_at  timestamptz not null default now()
, primary key (entity, entity_key, direction)
);
```

One row per (entity, key, direction) — an upsert, not a queue that grows. This is
also the watermark for the sync workers.

### 3.7 Orphan transcripts → sessions

Your point that these must resolve. Current state: **67 orphan sessions / 534
transcript_turn rows**, earliest 2025-05-11, all belonging to repos sensei does
not track.

The repair already runs on every backfill and correctly leaves them alone
(attributing to a guessed repo is worse than leaving them). What is missing is
that an untracked repo is invisible — there is no row anywhere saying "we have
history for a repo you have not added".

Proposal: a transcript that resolves to no tracked folder still creates a
**repository row by `repo_key`** (derived from the transcript's cwd via its git
remote, when discoverable) with `visibility='private'` and no folder link. The
session then attaches to a real repository, and the folder link is filled in if
and when the user tracks it. That turns 67 invisible orphans into 67 visible
untracked-repo sessions — and matches your model exactly: *the only local thing
is the folder link*.

Where the cwd yields no remote, the session stays orphaned and is surfaced in a
list rather than silently dropped.

---

## 4. What dojo needs that it does not have

Dojo's schema (28 tables in `database/ddl/table/dojo/`) has `tenants`,
`memberships`, `seats`, `identities`, `projects`, `relay_*`, `artifacts`,
`policies` — a multi-tenant app. It has **no** `repositories`, **no** `metrics`
registry, and **no** metric values.

`dojo.projects` today is `(user_id, tenant_id, slug, name, constitution)` — user-
scoped, not a repo collection. It needs to become tenant-scoped with membership,
or gain a sibling.

New dojo tables required:

| Table | Key | Purpose |
|---|---|---|
| `dojo.repositories` | `repo_key` unique per tenant | the shared repo identity |
| `dojo.project_repositories` | `(project_id, repository_id)` | the shared collection |
| `dojo.metrics` | `key` unique | authoritative metric config |
| `dojo.repository_metrics` | `(metric, repo, scope, identity, day, grain)` | the shared values |

---

## 5. Security model

Your requirement: *only repos accessible by the user get the data.*

```
identity → membership → tenant → project → project_repositories → repository
                                                                       ↓
                                                            repository_metrics
```

RLS on `dojo.repository_metrics`: a row is visible iff the requesting identity
has a membership in a tenant that owns a project containing that repository.

Two extra rules that matter:

1. **`scope='user'` rows are visible to their own identity plus tenant admins**, not to every member — otherwise "metrics by user" becomes surveillance of individuals by peers. Worth confirming (§7 Q4).
2. **Push authorization is per-repo**, not per-tenant: a client may only push metrics for repos it can prove it has (it holds the remote). Otherwise any member can poison another repo's numbers.

The pull path must also **fail closed** — if authorization can't be established,
local computes its own rather than receiving an empty set that looks like "no
data".

---

## 6. Pull-else-compute, concretely

```
for (repo, metric, day) in plan:
    if repo.visibility = 'shared' and metric.scope = 'repo':
        row = dojo.get(repo_key, metric, day)          # authorized
        if row:  store(origin='dojo');  continue        # ← do NOT recompute
    row = compute_locally()
    store(origin='local')
    if repo.visibility = 'shared': enqueue push
```

`origin` prevents the loop. `scope='user'` rows are always computed locally (they
need T0 data that never leaves the machine) and pushed up as values only.

That last point is the neat part of your design: **dojo can show per-user metrics
without ever holding a session, turn, or event.** Only the aggregate crosses.

---

## 7. Open questions — I need your call on these

**Q1 — Project identity across machines.** Two teammates create "sensei"
locally and get different uuids. Options: (a) dojo assigns the id and local
adopts it on first sync; (b) key projects by `(tenant, slug)` and keep local
uuids as aliases; (c) derive from the set of member repo_keys. I lean (a) —
simplest, and dojo is already authoritative for config.

**Q2 — Does a repo sync by default?** `visibility` defaults to `private`
(explicit opt-in per repo) or `shared` (opt-out)? Private-by-default is safer
but means the feature does nothing until configured. I lean private-by-default
with a prompt on first dojo link.

**Q3 — Can a repo belong to two projects?** Zero today. If we forbid it, the
`project_metrics` view is a clean join and roll-ups are unambiguous. If we allow
it, every project-level roll-up must decide how to weight a shared repo. I lean
forbid, with a unique constraint on `project_repositories.repository_id`.

**Q4 — Who sees `scope='user'` rows in dojo?** Self + admins, or all tenant
members? This is a people question, not a technical one, and it changes the RLS
policy.

**Q5 — Materialize weekly/monthly, or keep them as views?** Views are correct
today (re-derive from daily). Materializing is only worth it if dojo push
volume becomes the bottleneck.

**Q6 — What happens to a shared metric when dojo retires its definition?**
Local rows computed under the old definition still exist. Keep them with the
retired `metric_id` (honest history) or delete? I lean keep — `effective_until`
already models it.

---

## 8. Proposed order (DB layer only)

Forward-only; each step ships independently.

1. Drop the three dead columns from `project_metrics`; rename to
   `repository_metrics`; add `project_metrics` view. **No behaviour change** —
   0 rows affected, pure rename + derivation.
2. Add `project_repositories`; backfill from `folders`; make `folders.project_id`
   derived.
3. Add `visibility`/`synced_at` to `repositories`; `origin`/`shared_at` to
   `repository_metrics`.
4. Add `sync_state`.
5. Dojo-side tables + RLS.
6. Orphan-transcript → repository-by-repo_key resolution.

Steps 1–2 are worth doing regardless of whether dojo sharing proceeds — they fix
a table whose name has been lying and a membership model that cannot be shared.
