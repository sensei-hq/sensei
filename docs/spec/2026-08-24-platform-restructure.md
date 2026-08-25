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
- **Q12** *A turn is **one exchange**, not prompt-to-prompt.* Rationale: even an acknowledgement is a turn, and **fewer turns is often better** — a definition that folds several exchanges into one prompt boundary hides exactly the signal the metric exists to show. This is the transcript definition, so the merge keeps `transcript_turns`' grain. **It moves numbers** — see §3.3.
- **Q15** *The daemon gets a **restricted Postgres role***, not the default `authenticated`. Its JWT should be able to do only what sync needs.
- **Q16** *Git-alias claiming: **verified-email match only.*** No admin-review path in v1.
- **Q17** *Reuse GitHub's access control rather than inventing one.* GitHub already knows the user's **verified emails** (`user:email`) and **repo access** (`read:org`, collaborator lists). Take both as the source of truth; anything GitHub cannot answer is **dōjō-admin managed**. Email **domain** (`sensei-hq.com`, `example-corp.com` vs `gmail.com`/`icloud.com`) is a **hint for proposing** an org mapping — never an authorization signal on its own (see §3.3).
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

### 3.3 Two answers with teeth (Q12, Q17)

#### Q12: wipe and reprocess, so there is no cutover at all

Original concern was a dated cutover and a spliced series. **Superseded — we wipe
derived history and reprocess from source instead.** That gives one consistent
definition across the whole corpus with no discontinuity to explain, which is
strictly better than splicing.

Verified the sources actually survive before agreeing:

| Source | DB coverage | Source on disk | Recomputable |
|---|---|---|---|
| **zed** | 2025-05-07 → 2026-05-15 · 1,986 turns / 222 sessions | `threads.db` **127 MB, intact** (last write 2026-05-27, matches) | ✅ all |
| **opencode** | 2026-01-07 → 2026-08-23 · 246 turns / 30 sessions | own store, files back to 2024-12 | ✅ all |
| **claude_code** | 2026-05-21 → 2026-08-24 · 2,007 turns / 112 sessions | **72 of 77** cursor files present | ⚠️ 5 files gone |

**73 turns across 5 sessions** have no surviving transcript, all from
**2026-07-21 → 07-23** — `strategos/gateway` (50), `dbd-rs` (10 + 1),
`alert-platform` (10), `strategos` (2). That is **1.7% of 4,239 turns**.

Those are pre-rename paths, and the renames **are** known — `folder_path_aliases`
already holds exactly them:

```
/Users/Jerry/Developer/dbd-rs            → /Users/Jerry/Developer/dbd
/Users/Jerry/Developer/strategos/gateway → /Users/Jerry/Developer/gateway
/Users/Jerry/Developer/strategos/monorepo→ /Users/Jerry/Developer/torii
```

and `repair_sessions_from_transcripts` resolves through `find_folder_for_path`,
which is alias-aware. So those sessions **attribute correctly** — the rename is
not the problem.

The problem is narrower: **the transcript files themselves are gone.** The
directories survive holding only `memory/`, and the session UUIDs appear nowhere
on disk. An alias resolves a *path*; it cannot resurrect a *deleted file*. So
these 73 turns cannot be regenerated.

**Which makes the blanket wipe the wrong shape.** They are already ingested and
already attributable, so the fix is a carve-out rather than an accepted loss:

> Truncate the derived layer **except rows whose source unit no longer exists**
> (`transcript_cursor.file_path` absent on disk for a file-backed source).
> Reprocess everything else.

Zero loss, and the reprocess still yields one consistent definition.

Two of the four cwds — `/Users/Jerry/Developer/strategos` and
`/Users/Jerry/Work/Alert/repos/alert-platform` — have **no alias and no folder**,
so 12 of those turns would orphan even with their files. Worth adding aliases
before Phase 8 runs.

And to answer the May question directly: **all 319 Claude turns predating the
oldest surviving file are in files that still exist** — zero are in the vanished
five. File mtime is *last write*, so a session opened 2026-05-21 and continued
into July still has its transcript. Nothing from May is at risk.

#### Do not synthesize replacement transcripts either — the data is already better preserved

Considered rebuilding a transcript file from the surviving turn rows for the five
sessions with no file. **Recommend against it**, and it turns out to be
unnecessary.

Measured, for those 5 sessions:

```
activity.transcript_turns   73 rows    user_text ×73, assistant_text ×60,
                                       model ×73, tokens ×60,
                                       attrs: cwd, gitBranch, uuid, parentUuid,
                                              timestamp, version, promptId …
activity.assistant_events   2,109 rows PreToolUse 999 · PostToolUse 940
                                       UserPromptSubmit 46 · Stop 59
                                       SessionStart 6 · SessionEnd 5 · PreCompact 1
```

**The events already cover the dimension a rebuild would miss.** `transcript_turns`
retained **zero** tool content — 0 turns contain a tool block, and the 13
empty-`assistant_text` turns are precisely the tool-only ones. A transcript
reconstructed from those rows would represent 73 prose turns and **no tool use at
all**, against 999 real tool calls. Every metric derived from it would be quietly
wrong for those sessions.

Three further reasons:

1. **A round-trip can only lose.** rows → synthetic file → re-ingest extracts a *subset* of what the rows already hold. It cannot add information.
2. **False provenance.** A file asserting it is a Claude transcript when it is not. Placed in `~/.claude/projects` it corrupts another tool's state; placed elsewhere, ingestion must be told it is synthetic — which is the carve-out, reached by a longer road.
3. **`assistant_events` is kept anyway**, so the tool and structure dimensions survive the wipe untouched.

**Recovery audit — every candidate source checked, all ruled out on evidence:**

| Candidate | Verdict |
|---|---|
| Renamed transcript dirs (`dbd-rs`→`dbd`, `strategos/gateway`→`gateway`) | aliases already exist and resolve; the **files** are absent from those dirs |
| `~/.claude/file-history` (66 dirs, 190 MB) | Claude's file-**edit version store** (`<hash>@v1/@v2`), not transcripts; none of the 5 UUIDs present |
| Zed `threads.db` (127 MB) | covers those repos only to **2026-02-24 / 2026-03-20**; Zed's last activity anywhere is 2026-05-15, two months before |
| `database/import/staging/assistant_events.jsonl` (66 MB) | spans **2026-06-01 → 06-15** only; 0 lines for any of the 5 session ids — and it is a *subset* seed export (15,342 lines vs 76,710 DB rows in that window), not an archive |
| Synthetic rebuild from turn rows | possible but strictly worse — 0 tool content retained vs 999 real tool calls |
| Whole home tree, by session UUID | not present anywhere |

**Consequence — these five are barely a special case.** Because events survive,
the per-exchange turn derivation can be recomputed for them from events, the same
path `activity.turns` uses. Only the *prose* is unrecoverable, and the carve-out
preserves it verbatim. Nothing needs to be fabricated.

#### Do not move the transcript files — fix the mapping instead

Considered moving old-named transcript dirs into their new names
(`-…-dbd-rs/*` → `-…-dbd/`). **Recommend against it**, on three grounds:

1. **It cannot recover the five.** Those UUIDs exist nowhere on disk — searched the whole home tree. Moving files cannot create files that are absent.
2. **The directory name is a weaker cwd source than the content.** The encoding maps `/`→`-` and leaves literal `-` alone, so `-Users-Jerry-Developer-sensei-hq-sensei` is ambiguous between `sensei-hq/sensei` and `sensei/hq/sensei`. That ambiguity **is** resolvable by testing candidates against disk — verified: only `/Users/Jerry/Developer/sensei-hq/sensei` exists. But it resolves only while the path still exists, which is precisely the case where we do not need it. For a renamed or deleted folder — the case that matters — **disk cannot disambiguate** (neither `/Users/Jerry/Developer/dbd-rs` nor `/Users/Jerry/Developer/dbd/rs` exists). The adapters therefore read cwd from transcript *content* (`attrs.cwd`), which is authoritative in both cases. Reorganising by directory name changes nothing about how resolution actually works.
3. **It would break two things for no gain.** `transcript_cursor.file_path` keys on the absolute path, so every moved file re-ingests as new. And `~/.claude/projects` is Claude Code's own state directory — its resume/index behaviour reads that layout. Mutating another tool's state to fix our attribution is the wrong direction.

**What actually fixes it:** the unresolved cwds are a *mapping* gap in our own DB,
not a filesystem one. Measured — 16 distinct cwds resolve to no folder and no
alias:

| Action | Count | Paths |
|---|---|---|
| **Track it** (exists on disk) | 6 | `Developer/sensei-hq`, `Developer/jovy`, `Developer/llm-rules`, `Work/Alert`, `Work/Babb`, `Work/Got-a-guy` |
| **Add an alias** (gone) | 10 | `Developer/sensei`, `Developer/reader`, `Developer/magpie-scanner`, `jovy/wix-mirror`, `Work/AI`, `Work/Wombat`, `Work/FizzBot`, `Work/Value Pricing`, `Work/Basketball App`, `Alert/example-alert-site` |

`/Users/Jerry/Developer/sensei` → `/Users/Jerry/Developer/sensei-hq/sensei` is
almost certainly **this repo's old location**, so one alias recovers a block of
Zed history for sensei itself.

**Not in that list, and worth stating:**
`/Users/Jerry/Work/Alert/repos/alert-platform` **already resolves** — it has an
exact `sensei.folders` row (810 rows exist under `Alert/repos`). Its transcript
directory `-Users-Jerry-Work-Alert-repos-alert-platform` maps correctly too; it
simply holds **0 `.jsonl`** (only `memory/`), because the file was pruned. So that
session needs no alias and no tracking — its 10 turns are already correctly
attributed in the DB, and the vanished-file carve-out is what preserves them.
The same is true of `strategos/gateway` and `dbd-rs`, which alias correctly.

That is the general shape: **a missing transcript and an unresolvable cwd are
different failures.** Only the second is fixable by mapping; the first is fixable
only by not deleting the rows we already have.

Both actions are reversible rows in our own database, and they make the reprocess
attribute correctly without touching a single file on disk. This should run
**before** Phase 8.

#### What must NOT be wiped

Everything above regenerates from a durable source. These do not:

| Table | Rows | Why it cannot be regenerated |
|---|---|---|
| `activity.assistant_events` | 298,353 | the hook stream — **the source** `turns` derive from |
| `sensei.memories` | 16 | `origin='authored'` rows are **user-written**; nothing can recreate them |
| `sensei.tool_insights` | 34,697 | accumulated observation, not a pure function of transcripts |
| `inference.recommendations` | 2,459 | LLM output — re-running costs spend **and returns different text** |
| `inference.drift_items` | 3,032 | same |
| `inference.detected_patterns` | 1,367 | same |
| `activity.session_process_evidence` | 406 | LLM-derived per session; non-deterministic on re-run |
| `sensei.playbook_rules` / `consolidated_rulesets` / `memory_outcomes` | 6 / 2 / 3 | accumulated learning, some human-accepted |

`inference.communities` (76,895) regenerates from the code graph and can go.

So Phase 8 is: **truncate the derived layer** (`transcript_turns`, `turns`,
`sessions`, `repository_metrics`, watermarks, cursors), **keep events and the
learned/authored layer**, then reprocess. The four derived columns (`segment`,
`is_correction`, `triage_signal`, `tool_calls`) still need **redefinition** for
per-exchange grain before the reprocess runs — that remains the real design work;
the wipe just removes the migration and the discontinuity.

#### Q17 — reuse GitHub, with one boundary

Taking GitHub as the source of truth is right and removes most of the identity
problem: its **verified emails** are verified *by GitHub* (mailbox control
proven), which is a far stronger assertion than a git commit trailer — anyone can
put any address in a commit. So:

```
git commit email  ──matches──►  a GitHub-verified email of the authenticated user
                                        │
                                        ▼
                                  alias is CLAIMED (Q16)
otherwise ─────────────────────► unclaimed; stays local, never attributed in dōjō
```

That closes the attribution attack from the ADR without an admin queue.

**The one boundary:** the email-domain heuristic is a *suggestion*, not
authorization. `dev@example-corp.com` proves mailbox control at that
domain — it does **not** prove membership of the Seneca Global org. **GitHub org
membership is the authoritative signal**; the domain merely proposes *which*
tenant to offer. Keeping those separate is what stops "I own an address at your
domain" from becoming "I am in your dōjō".

Scope note: this needs `user:email` and `read:org`. Auth doc Scenario 21 already
covers reduced scope — preserve existing rows, log, never delete.

### 3.2 What Q13 strands — CORRECTED

An earlier revision of this section claimed ~7,600 LOC targets the
non-existent dōjō service and should be triaged for retirement. **That was
wrong, and the correction matters** — acting on it would have deleted the
confidentiality layer.

Measured per file rather than in aggregate:

| | LOC | Outbound refs |
|---|---:|---|
| `dojo/client.rs` — the HTTP transport | 1,101 | 122 |
| `crates/dojo-protocol` — its wire types | 1,561 | (types only) |
| **everything else in `dojo/`** | **4,979** | **0–9 each** |

The 4,979 is not transport code. It is:

- **`attribution.rs` (704)** — *"the confidentiality SAFETY NET … client identifiers must NEVER leave the machine"*. Transport-independent, and the highest-stakes code in the layer.
- **`gate.rs` (1,165)** — the hook-gate decision core for the daemon↔agent control leg. Pure local logic, reached by `/hook/gate`, and **unrelated to the dōjō service entirely**.
- **`contribute.rs` (1,259)**, `routing.rs` (419), `memberships.rs` (431), `relay_*` — building, anonymising and routing what would be published. Nine outbound references across 1,259 lines in the largest of them; the rest is local.

So the accurate statement is: **~2,660 LOC is transport-bound, and it gets
REPLACED rather than deleted** when the Supabase path lands. Every transport
needs the privacy gate, the dereference, and the routing that sits above it —
including the one Phase 7 builds.

**Recommendation: retire nothing.** The transport is not costing anything (the
publish path has never run — `dojo_outbox` is still 0 rows), and deleting it
would take the confidentiality layer with it or leave the local logic with no
output. `dojo-protocol` is also reusable as the payload schema for the Supabase
path even when the HTTP client is not.

What the earlier framing got right: nothing should be BUILT on `client.rs`
before Phase 7 decides the transport.

### 3.3 Two answers with teeth (Q12, Q17)

#### Q12: wipe and reprocess, so there is no cutover at all

Original concern was a dated cutover and a spliced series. **Superseded — we wipe
derived history and reprocess from source instead.** That gives one consistent
definition across the whole corpus with no discontinuity to explain, which is
strictly better than splicing.

Verified the sources actually survive before agreeing:

| Source | DB coverage | Source on disk | Recomputable |
|---|---|---|---|
| **zed** | 2025-05-07 → 2026-05-15 · 1,986 turns / 222 sessions | `threads.db` **127 MB, intact** (last write 2026-05-27, matches) | ✅ all |
| **opencode** | 2026-01-07 → 2026-08-23 · 246 turns / 30 sessions | own store, files back to 2024-12 | ✅ all |
| **claude_code** | 2026-05-21 → 2026-08-24 · 2,007 turns / 112 sessions | **72 of 77** cursor files present | ⚠️ 5 files gone |

**73 turns across 5 sessions** have no surviving transcript, all from
**2026-07-21 → 07-23** — `strategos/gateway` (50), `dbd-rs` (10 + 1),
`alert-platform` (10), `strategos` (2). That is **1.7% of 4,239 turns**.

Those are pre-rename paths, and the renames **are** known — `folder_path_aliases`
already holds exactly them:

```
/Users/Jerry/Developer/dbd-rs            → /Users/Jerry/Developer/dbd
/Users/Jerry/Developer/strategos/gateway → /Users/Jerry/Developer/gateway
/Users/Jerry/Developer/strategos/monorepo→ /Users/Jerry/Developer/torii
```

and `repair_sessions_from_transcripts` resolves through `find_folder_for_path`,
which is alias-aware. So those sessions **attribute correctly** — the rename is
not the problem.

The problem is narrower: **the transcript files themselves are gone.** The
directories survive holding only `memory/`, and the session UUIDs appear nowhere
on disk. An alias resolves a *path*; it cannot resurrect a *deleted file*. So
these 73 turns cannot be regenerated.

**Which makes the blanket wipe the wrong shape.** They are already ingested and
already attributable, so the fix is a carve-out rather than an accepted loss:

> Truncate the derived layer **except rows whose source unit no longer exists**
> (`transcript_cursor.file_path` absent on disk for a file-backed source).
> Reprocess everything else.

Zero loss, and the reprocess still yields one consistent definition.

Two of the four cwds — `/Users/Jerry/Developer/strategos` and
`/Users/Jerry/Work/Alert/repos/alert-platform` — have **no alias and no folder**,
so 12 of those turns would orphan even with their files. Worth adding aliases
before Phase 8 runs.

And to answer the May question directly: **all 319 Claude turns predating the
oldest surviving file are in files that still exist** — zero are in the vanished
five. File mtime is *last write*, so a session opened 2026-05-21 and continued
into July still has its transcript. Nothing from May is at risk.

#### Do not synthesize replacement transcripts either — the data is already better preserved

Considered rebuilding a transcript file from the surviving turn rows for the five
sessions with no file. **Recommend against it**, and it turns out to be
unnecessary.

Measured, for those 5 sessions:

```
activity.transcript_turns   73 rows    user_text ×73, assistant_text ×60,
                                       model ×73, tokens ×60,
                                       attrs: cwd, gitBranch, uuid, parentUuid,
                                              timestamp, version, promptId …
activity.assistant_events   2,109 rows PreToolUse 999 · PostToolUse 940
                                       UserPromptSubmit 46 · Stop 59
                                       SessionStart 6 · SessionEnd 5 · PreCompact 1
```

**The events already cover the dimension a rebuild would miss.** `transcript_turns`
retained **zero** tool content — 0 turns contain a tool block, and the 13
empty-`assistant_text` turns are precisely the tool-only ones. A transcript
reconstructed from those rows would represent 73 prose turns and **no tool use at
all**, against 999 real tool calls. Every metric derived from it would be quietly
wrong for those sessions.

Three further reasons:

1. **A round-trip can only lose.** rows → synthetic file → re-ingest extracts a *subset* of what the rows already hold. It cannot add information.
2. **False provenance.** A file asserting it is a Claude transcript when it is not. Placed in `~/.claude/projects` it corrupts another tool's state; placed elsewhere, ingestion must be told it is synthetic — which is the carve-out, reached by a longer road.
3. **`assistant_events` is kept anyway**, so the tool and structure dimensions survive the wipe untouched.

**Recovery audit — every candidate source checked, all ruled out on evidence:**

| Candidate | Verdict |
|---|---|
| Renamed transcript dirs (`dbd-rs`→`dbd`, `strategos/gateway`→`gateway`) | aliases already exist and resolve; the **files** are absent from those dirs |
| `~/.claude/file-history` (66 dirs, 190 MB) | Claude's file-**edit version store** (`<hash>@v1/@v2`), not transcripts; none of the 5 UUIDs present |
| Zed `threads.db` (127 MB) | covers those repos only to **2026-02-24 / 2026-03-20**; Zed's last activity anywhere is 2026-05-15, two months before |
| `database/import/staging/assistant_events.jsonl` (66 MB) | spans **2026-06-01 → 06-15** only; 0 lines for any of the 5 session ids — and it is a *subset* seed export (15,342 lines vs 76,710 DB rows in that window), not an archive |
| Synthetic rebuild from turn rows | possible but strictly worse — 0 tool content retained vs 999 real tool calls |
| Whole home tree, by session UUID | not present anywhere |

**Consequence — these five are barely a special case.** Because events survive,
the per-exchange turn derivation can be recomputed for them from events, the same
path `activity.turns` uses. Only the *prose* is unrecoverable, and the carve-out
preserves it verbatim. Nothing needs to be fabricated.

#### Do not move the transcript files — fix the mapping instead

Considered moving old-named transcript dirs into their new names
(`-…-dbd-rs/*` → `-…-dbd/`). **Recommend against it**, on three grounds:

1. **It cannot recover the five.** Those UUIDs exist nowhere on disk — searched the whole home tree. Moving files cannot create files that are absent.
2. **The directory name is a weaker cwd source than the content.** The encoding maps `/`→`-` and leaves literal `-` alone, so `-Users-Jerry-Developer-sensei-hq-sensei` is ambiguous between `sensei-hq/sensei` and `sensei/hq/sensei`. That ambiguity **is** resolvable by testing candidates against disk — verified: only `/Users/Jerry/Developer/sensei-hq/sensei` exists. But it resolves only while the path still exists, which is precisely the case where we do not need it. For a renamed or deleted folder — the case that matters — **disk cannot disambiguate** (neither `/Users/Jerry/Developer/dbd-rs` nor `/Users/Jerry/Developer/dbd/rs` exists). The adapters therefore read cwd from transcript *content* (`attrs.cwd`), which is authoritative in both cases. Reorganising by directory name changes nothing about how resolution actually works.
3. **It would break two things for no gain.** `transcript_cursor.file_path` keys on the absolute path, so every moved file re-ingests as new. And `~/.claude/projects` is Claude Code's own state directory — its resume/index behaviour reads that layout. Mutating another tool's state to fix our attribution is the wrong direction.

**What actually fixes it:** the unresolved cwds are a *mapping* gap in our own DB,
not a filesystem one. Measured — 16 distinct cwds resolve to no folder and no
alias:

| Action | Count | Paths |
|---|---|---|
| **Track it** (exists on disk) | 6 | `Developer/sensei-hq`, `Developer/jovy`, `Developer/llm-rules`, `Work/Alert`, `Work/Babb`, `Work/Got-a-guy` |
| **Add an alias** (gone) | 10 | `Developer/sensei`, `Developer/reader`, `Developer/magpie-scanner`, `jovy/wix-mirror`, `Work/AI`, `Work/Wombat`, `Work/FizzBot`, `Work/Value Pricing`, `Work/Basketball App`, `Alert/example-alert-site` |

`/Users/Jerry/Developer/sensei` → `/Users/Jerry/Developer/sensei-hq/sensei` is
almost certainly **this repo's old location**, so one alias recovers a block of
Zed history for sensei itself.

**Not in that list, and worth stating:**
`/Users/Jerry/Work/Alert/repos/alert-platform` **already resolves** — it has an
exact `sensei.folders` row (810 rows exist under `Alert/repos`). Its transcript
directory `-Users-Jerry-Work-Alert-repos-alert-platform` maps correctly too; it
simply holds **0 `.jsonl`** (only `memory/`), because the file was pruned. So that
session needs no alias and no tracking — its 10 turns are already correctly
attributed in the DB, and the vanished-file carve-out is what preserves them.
The same is true of `strategos/gateway` and `dbd-rs`, which alias correctly.

That is the general shape: **a missing transcript and an unresolvable cwd are
different failures.** Only the second is fixable by mapping; the first is fixable
only by not deleting the rows we already have.

Both actions are reversible rows in our own database, and they make the reprocess
attribute correctly without touching a single file on disk. This should run
**before** Phase 8.

#### What must NOT be wiped

Everything above regenerates from a durable source. These do not:

| Table | Rows | Why it cannot be regenerated |
|---|---|---|
| `activity.assistant_events` | 298,353 | the hook stream — **the source** `turns` derive from |
| `sensei.memories` | 16 | `origin='authored'` rows are **user-written**; nothing can recreate them |
| `sensei.tool_insights` | 34,697 | accumulated observation, not a pure function of transcripts |
| `inference.recommendations` | 2,459 | LLM output — re-running costs spend **and returns different text** |
| `inference.drift_items` | 3,032 | same |
| `inference.detected_patterns` | 1,367 | same |
| `activity.session_process_evidence` | 406 | LLM-derived per session; non-deterministic on re-run |
| `sensei.playbook_rules` / `consolidated_rulesets` / `memory_outcomes` | 6 / 2 / 3 | accumulated learning, some human-accepted |

`inference.communities` (76,895) regenerates from the code graph and can go.

So Phase 8 is: **truncate the derived layer** (`transcript_turns`, `turns`,
`sessions`, `repository_metrics`, watermarks, cursors), **keep events and the
learned/authored layer**, then reprocess. The four derived columns (`segment`,
`is_correction`, `triage_signal`, `tool_calls`) still need **redefinition** for
per-exchange grain before the reprocess runs — that remains the real design work;
the wipe just removes the migration and the discontinuity.

#### Q17 — reuse GitHub, with one boundary

Taking GitHub as the source of truth is right and removes most of the identity
problem: its **verified emails** are verified *by GitHub* (mailbox control
proven), which is a far stronger assertion than a git commit trailer — anyone can
put any address in a commit. So:

```
git commit email  ──matches──►  a GitHub-verified email of the authenticated user
                                        │
                                        ▼
                                  alias is CLAIMED (Q16)
otherwise ─────────────────────► unclaimed; stays local, never attributed in dōjō
```

That closes the attribution attack from the ADR without an admin queue.

**The one boundary:** the email-domain heuristic is a *suggestion*, not
authorization. `dev@example-corp.com` proves mailbox control at that
domain — it does **not** prove membership of the Seneca Global org. **GitHub org
membership is the authoritative signal**; the domain merely proposes *which*
tenant to offer. Keeping those separate is what stops "I own an address at your
domain" from becoming "I am in your dōjō".

Scope note: this needs `user:email` and `read:org`. Auth doc Scenario 21 already
covers reduced scope — preserve existing rows, log, never delete.

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

## 3a. Who mints IDs — dōjō is the owner, sensei the receiver

**Decision (2026-08-25, Jerry).** For every SHARED entity, dōjō creates the row
and mints its id; sensei receives and mirrors it. sensei never invents an id for a
shared entity.

This is a structural fix, not a reconciliation strategy. An id conflict is only
possible when two places can both CREATE the same entity — the case we hit when an
org existed in sensei and the "same" org was created in dōjō with a different id.
Removing the second minter removes the possibility; a merge/repair path would only
paper over it, and would have to run forever.

Auth is the natural moment. GoTrue already mints the user id, and the verified
GitHub identity that arrives with it is what tenants and repositories derive from,
so the whole shared graph can be established in one authenticated pass.

### Ownership

| dōjō owns — sensei mirrors                     | sensei owns — pushes up                    |
|------------------------------------------------|--------------------------------------------|
| principal / verified identity, claimed aliases  | git-discovered emails not yet verified     |
| tenants, teams, memberships                     | folders and paths (machine-specific)       |
| repositories, repositories_in_projects          | `repository_metrics` VALUES                |
| `metrics` registry, `metric_activations`        | attribution / client identifiers           |

Two entries are load-bearing and not negotiable by convenience:

* **Folders and paths never go up.** They name a person's disk.
* **Attribution and client identifiers never go up.** See `attribution.rs` — that
  constraint predates this document and is not relaxed by dōjō owning the graph.

### The offline case

sensei discovers repositories locally, including with no session and no network,
and must keep doing so. So "dōjō mints the id" cannot mean "block until dōjō
answers".

A locally-discovered repository is written with its `repo_key` and `dojo_id NULL`,
and a task claims the id when connectivity returns. `repo_key` is already the
machine-independent join key, so it carries the mapping in the meantime.

`dojo_id IS NULL` reads as "not registered yet", which is TRUE — it is not a
fabricated identity, and it is distinguishable from a registered row. That keeps
it inside the never-fabricate rule; minting a local uuid and hoping to reconcile
later would not.

### Consequence for the provisioning pipeline

The order inverts from the earlier sketch. Rather than sensei creating tenants and
pushing them, sensei asks dōjō to provision and then pulls the result:

    SyncGitHubIdentity   → dōjō upserts the principal + claimed aliases
    ProvisionTenants     → dōjō creates tenants from the GitHub orgs
    SyncOrgRepositories  → dōjō registers repositories
    PullSharedGraph      → sensei mirrors ids into its local rows
    PushMetricValues     → sensei sends values keyed by the dōjō ids

Only the last step originates in sensei, which is the point: values are computed
locally because that is where the code is, and nothing else is.

**Blocked on:** the kavach double-`resolve` bug — every write leg is a POST and
POST bodies currently arrive empty. See `docs/backlog.md`.

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
| ~~Q15~~ | **Answered:** restricted Postgres role | — | — |
| ~~Q16~~ | **Answered:** verified-email match only | — | — |
| ~~Q17~~ | **Answered:** reuse GitHub's verified emails + repo access; rest is admin-managed; domain is a hint only | — | — |
| ~~Q12~~ | **Answered:** **per-exchange** — fewer turns is often better, so the boundary must not hide it (§3.3) | — | — |

**Answered:** Q1 (dōjō wins) · Q2 (auth-gated auto-sync) · Q3 (one repo, one project *per tenant*) · Q4 (self + admins) · Q5 (views first) · Q6 (keep retired history) · Q7 (all workers local) · Q8 (multi-tenant repos, one owner) · Q9 (inactive until activated) · Q10 (web = viewers) · Q11 (teams now, UI later) · Q12 (per-exchange turns) · Q13 (no Rust service) · Q14 (no self-hosting) · Q15 (restricted role) · Q16 (verified-email claiming) · Q17 (reuse GitHub).

**All 18 answered or deferred.** The design is decided; what remains is sequencing and build.

**Deferred:** Q18 (cross-tenant user-metric visibility).
