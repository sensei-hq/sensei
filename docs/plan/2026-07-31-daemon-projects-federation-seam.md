# Scope — daemon → dōjō `dojo.projects` federation seam

> Populate `dojo.projects` (shipped, empty) so the projects screens go from
> honest-empty to live. The daemon upserts a project row on relay runs, alongside
> the status it already federates. Cross-repo: daemon (Rust) + `dojo-protocol` +
> dōjō Worker.

## The seam already half-exists — piggyback it

`publish_run` (`crates/senseid/src/tasks/handlers/publish_run.rs`) already fires on
every run: it resolves the owning membership(s) and POSTs a `RelaySessionUpdate` to
the dōjō, which upserts `dojo.relay_sessions`. That update **already carries
`project_slug`** (`publish_run.rs:119`, from `run_project_slug()` →
`sensei.namespaces.slug` scope=`project`), which the dōjō session endpoint already
consumes to open a billing seat.

So there is **no new endpoint, no new `DojoClient` method, no new task**. The seam is:
1. add a project block to the wire type,
2. fill it next to the existing `project_slug` resolve,
3. upsert `dojo.projects` next to the existing `relay_sessions` upsert.

```
activity.runs.project_id ─→ sensei.projects (name, dojo_id, client)
         │                        │
         │                        └─ dojo_id → dojo_memberships.kind ─→ classification
         └─ run_project_slug() → sensei.namespaces.slug (project)  ─→ slug (as-is; user's own metadata)
                                                                       phase = 'watch' (default)
   publish_run → RelaySessionUpdate{…, project: {slug,name,classification,phase}}
                                     │  (device-token POST, existing)
   dōjō relay/session +server.ts ───┴─→ upsert dojo.relay_sessions (existing)
                                     └─→ upsert dojo.projects  ← NEW
                                          user_id  = caller.userId   (authenticated, not payload)
                                          tenant_id= caller.tenantId or null (personal)
```

## Source-data map (from the daemon audit)

| Field | Source | State |
|---|---|---|
| `slug` | `sensei.namespaces.slug` (scope=project) via `run_project_slug()` `pg_store.rs:7880` | exists, already federated (billing). Federate as-is — it's the user's own project metadata, shown in their own RLS-scoped console. No anonymization (there is NO client-specific dereference — see [[reference_universal_dereference_invariant]]). |
| `name` | `sensei.projects.name` `sensei/projects.ddl:47` | exists. Federate as-is (same reasoning — the user's own metadata). |
| `classification` | bound membership `kind` via `sensei.projects.dojo_id → dojo_memberships.kind` (`project_bound_membership()` `pg_store.rs:6837`) | **net-new mapping** `employer→company`, else 1:1; unbound → `personal`. |
| `phase` | — | **not tracked daemon-side → send `watch`** (the enum default). Advancement is a later governance concern, not this seam. |
| `user_id` (owner) | — | daemon has only git author; **the dōjō sets it from `caller.userId`** (authenticated device-token identity). Never from payload. |
| owning membership + device token | `resolve_run_memberships()` `publish_run.rs:216` + Keychain `credential_ref` | exists — the reusable path. |

## Files to touch

**`crates/dojo-protocol/src/relay.rs`** — add a nested project block to
`RelaySessionUpdate` (`:255`). Keep the existing raw `project_slug` (billing uses
it unchanged):
```rust
#[serde(default, skip_serializing_if = "Option::is_none")]
pub project: Option<RelayProjectInfo>,
// new struct:
pub struct RelayProjectInfo { pub slug: String, pub name: String,
    pub classification: String, pub phase: String }
```

**`crates/senseid/src/dojo/` (projector) + `pg_store.rs`** — build `RelayProjectInfo`:
- a `kind → classification` converter (net-new: `employer→company`, else identity);
- `phase = "watch"`;
- extend `run_project_slug` → a `run_project_info(run_id)` that also returns
  `sensei.projects.name` (join `activity.runs → sensei.projects`) — federated as-is;
- fill it in `publish_run.rs` next to line 119, **only for the owning membership**
  (see decision 2). Best-effort — a missing project never fails federation
  (mirror the existing `.ok().flatten()` on `project_slug`).

**`dojo/src/routes/v1/t/[origin]/[org]/relay/session/+server.ts`** — after the
existing `relay_sessions` upsert + seat open, if `body.project` is present, upsert
`dojo.projects`:
- `user_id = caller.userId`, `tenant_id = classification==='personal' ? null : caller.tenantId`,
  `slug/name/classification` from the payload, `phase` **insert-only** (don't clobber
  a dōjō-advanced phase — `onConflict` update omits phase), `last_run_at = now()`;
- `onConflict: 'user_id,slug'`;
- best-effort like the seat (a projects-upsert failure must not fail the session
  federation). Extract to `$lib/server/projects-data.ts` (`upsertProjectFromRun`)
  so it's unit-testable, mirroring the shipped `listOrgProjects`.

## NO client-dereference step (settled — do not re-raise)

There is **no client-specific dereference** and nothing to anonymize here. Content
(insights/memories/artifacts) is dereferenced at **derivation** — the analysis jobs
keep no code references, so it's dereferenced by construction long before this seam.
A project's own **slug/name is metadata**, shown back to the owner in their own
RLS-scoped console (`user_id = auth.uid()`) — federate it as-is. See
[[reference_universal_dereference_invariant]]. (Earlier drafts of this doc invented
an "anonymized client slug" decision; that was a mistake — removed.)

## One design decision (need a call before building)

**Fan-out gating.** `resolve_run_memberships` publishes an *unbound*
run to **all** enabled memberships (phone visibility). But `dojo.projects` is
`unique(user_id, slug)` — replicating a project across every tenant would race the
row's `tenant_id`. Gate the projects upsert:
- **bound** project → include the project block only when publishing to the *bound*
  membership → one row, `tenant_id` set;
- **unbound** project → `classification='personal'`, and the dōjō forces
  `tenant_id=null` → all fan-out upserts idempotently hit the one personal row.
This is already expressible: only attach `project` to the update for the bound
membership, or personal. Confirm the rule.

## Tests
- **Daemon (pure):** `kind→classification` map (all 4 + unbound→personal);
  `run_project_info` shape (slug + name). Rust unit tests alongside the projector
  (mirror `relay_run_project.rs`'s test style).
- **Dōjō:** `upsertProjectFromRun` (projects-data.spec) — owner from caller not
  payload, tenant null for personal, phase insert-only, best-effort swallow on
  error; the session route spec gains a "upserts a project when the block is
  present / skips when absent" case. ≥90%, fail-closed.

## Deploy
- Daemon: Rust rebuild (`make crates` / the daemon binary) — **not** a dōjō deploy.
  The protocol crate change ripples to daemon + (type-only) the Worker.
- Dōjō: the session-endpoint change ships via the usual clean-rebuild + `wrangler
  deploy` (no new route method — the endpoint exists, so the stale-bundle 405 risk
  is lower, but clean-rebuild anyway).
- No DDL (table already live on prod).

## Effort
Small. The transport, membership resolution, and seat plumbing are done. Net-new
work is three items (kind→classification converter, `phase='watch'` default,
worker-side owner) + the one fan-out-gating decision. All mechanical wiring on
proven paths — no anonymization, no new slug generator.
