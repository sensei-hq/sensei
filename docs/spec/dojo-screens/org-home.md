# Org home — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.
- Route: `(app)/org/[slug]` — `+page.svelte` + `+page.ts` (`dojo/src/routes/(app)/org/[slug]/`)
- Mockup: dojo2-app.jsx `ScrOrgHome` (L577)
- Access axis: tenant-primary — the org console `/org/[slug]/*` is genuinely tenant-scoped (`docs/architecture/entity-access-model.md` §3, row "Org console … → Tenant → tenant_id"). Correct as built.
- Status: PARTIAL — the real `ScrOrgHome` renders and the org name/slug resolve from real memberships, but every data element is a fixture or a hardcoded 0; the jurisdiction stat row (members, needs) is literally `0` and the project list only populates for slug `acme`.

## ⚠ Fabricated-data debt — MUST fix on build (2026-07-29 fallback audit)
The live post-auth `(app)` route renders fixtures: org index `(app)/org/[slug]/+page.ts:21` (`orgProjectsFor`) + `:24` (`needsYou`), and `[section]/+page.ts:223` (`orgConstitutionFor`) / `:224` (`orgProjectsFor`). **Impact:** a real member sees fabricated repos, constitution, and a "needs you" band for slug `acme` (other slugs render the fixture-shaped empty), not this jurisdiction's real data. **Fix on build:** drive every field from the real `/v1` read; on a fetch error render an explicit error state — NEVER the fixture; honest-empty only when genuinely empty. (Ties the daemon-side fabrication fixes + the "no fabricated fallbacks" rule.)

## Elements → data (contract)
| Element | Mockup field | Source (loader/API/table.field) | Status: have/bind/plumb | Realtime? |
|---|---|---|---|---|
| Eyebrow "{org} · jurisdiction" | `org.name` | `+page.ts` `org.name` ← `orgBySlug(memberships, slug)` → `DojoOrg.name` ← `dojo.tenants.name`/`.org` (via `server/dojo-orgs.ts tenantToOrg`) | have | no |
| Title "N projects under this dōjō" | `projs.length` | `+page.ts` `projects.length` ← `orgProjectsFor(slug)` **FIXTURE** (`components/kit/fixtures.ts`; only `acme` seeded, else `[]`) | plumb | no |
| Stat badge · members | `org.members` | `stats.members` ← `org.members` ← `tenantToOrg` **HARDCODED `members: 0`** (`server/dojo-orgs.ts:59`) | plumb | no |
| Stat badge · need a maintainer | `org.needs` | `stats.needs` ← `org.pending` ← `tenantToOrg` **HARDCODED `pending: 0`** | plumb | no |
| Stat badge · projects in flight | `org.projects` | `stats.projects` ← `projects.length` (fixture, as above) | plumb | no |
| Needs-a-maintainer band | `D2.needsYou.slice(0,2)` | `+page.ts` `needs` ← `needsYou.slice(0,2)` **FIXTURE** (cross-dōjō sample re-cast as this jurisdiction) | plumb | maybe (queue) |
| Band action (`onAct`) / resolved chips | `onAct`, `resolved` | none — `+page.svelte` passes **only** `onOpenProject`; `onAct`/`resolved` never wired → band buttons inert | plumb | — |
| Project row · repo | `slug + "/" + name` | `orgProjectsFor` maps `repo = slug/name` (fixture) → real: `sensei.namespaces` (scope=project) bound to tenant | plumb | no |
| Project row · note | `team · N maintainers` | fixture `note = team + " · " + maintainers` → real: no maintainer-per-project count wired | plumb | no |
| Project row · lastRun | `runsWeek + "/wk"` | fixture `runsWeek` → real: `activity.runs`/`relay_sessions` cadence for the project | plumb | no |
| Project row · classification/phase/needs | `p.classification` `p.phase` `p.needs` | fixture (`OrgProjectSeed`) → real: namespace visibility/classification + per-project needs count | plumb | no |
| "By team" button | (label only) | no handler — inert | plumb | — |
| Empty state | (fixture-driven) | `EmptyState` renders when `projects.length === 0` (honest-empty, DJ1) — already correct | have | — |

## APIs / loaders
- **Loader:** `dojo/src/routes/(app)/org/[slug]/+page.ts` (`PageLoad`). Resolves org via `orgBySlug(memberships, params.slug)` (from the `(app)` layout's `loadConsoleContext`); redirects `307 → /you` for a non-member (DJ1). Returns `{ slug, orgName, projects, needs, stats }`.
- **No `/v1` fetch on this route.** Unlike the `[section]` route, the org-home index calls no Worker endpoint — projects/needs/stats are pure fixtures/derived-constants.
- **Available-but-unused real sources** for the wiring:
  - members count → `dojo.memberships` `count where tenant_id=? and disabled_at is null` (same table `server/admin-data.ts listMembers` reads; no count helper yet).
  - jurisdiction projects → `sensei.namespaces` (scope=project) ∪ `dojo.seats.tenant_id` (the billing seat join already loads private-project namespaces in `server/billing-data.ts loadActiveSeatRows`).
  - needs/queue → `dojo.triage_queue` (health rollup already counts `state='queued'`) and/or relay gates.

## Interactions & states
- **Open project** → `goto(orgHref(slug,'projects') + '/' + p.id)` → `/org/[slug]/projects/[id]` (URL-driven preview; shell keeps "Projects" active). Wired.
- **Needs band act / By team / resolved** → not wired (no handlers passed).
- **Empty** → `EmptyState` ("No projects in this jurisdiction yet"). Wired + honest.
- **Non-member** → redirect to `/you`. Wired.
- **Mobile** → drops the stat row (`!mobile && stats`), tighter padding. Wired.

## Gap / to-do (vs mockup)
1. **Stat row is fake.** `members` and `needs` are hardcoded `0` in `tenantToOrg`; `projects` counts a fixture. Wire a tenant rollup (`GET /v1/t/[origin]/[org]/…` — no such summary endpoint exists yet; closest is `/health`, which lacks member/project counts).
2. **Projects list is fixture-only** (`acme`). Needs a real jurisdiction-projects read (namespaces × tenant seats/`dojo_id`) + run-cadence.
3. **Needs band is a cross-dōjō sample**, not this jurisdiction's queue, and its actions are inert. Repoint to an org-scoped gate/triage queue and pass `onAct`/`resolved` through the page.
4. **"By team" grouping** unimplemented.

## Open questions (for Jerry)
- Is there (or should there be) a single `GET …/summary` (members · projects · needs) for the stat row, or compose it from `/members` + a new `/projects` + `/triage`?

### Resolved design (2026-07-30)
- **Single `GET /v1/t/[origin]/[org]/summary`** → `{ members, projects, needs }`, server-side rollup in one round-trip. `members` = `count(dojo.memberships where tenant_id=? and disabled_at is null)`; `projects` = jurisdiction projects (via `dojo.projects` once it lands / `sensei.namespaces scope=project`); `needs` = `dojo.triage_queue` count. The members read derives `dojo_url` from `tenant_id → tenants.dojo_url` (register §1C), NOT the dropped `memberships.dojo_url`.
- **Build constraint (fabricated-data debt):** every stat is a real number — never the hardcoded `0` in `tenantToOrg` nor a fixture length; honest-empty/error, never the `acme` fixture.
- **Depends on:** the new `/summary` endpoint + `dojo.projects` (project count) + `dojo.triage_queue` (needs).
- "Need a maintainer" count — define it: unowned scope queues (`scopeOwners` fallback), queued triage rows, or open relay gates? (Mockup conflates all four via `needsYou`.)
- Jurisdiction "projects in flight" — is the source `sensei.namespaces` bound by `dojo.seats.tenant_id`, or a project↔membership `dojo_id` binding (entity-access-model §Project)? No project-per-tenant list endpoint exists yet.

## Components & state (three-layer, per `sensei:ui-state-pattern`)

**Domain types** (UI-shaped; the Load layer maps wire → these):
```ts
type OrgHome = { slug: string; name: string; stats: OrgStats;
  needs: MaintainerAsk[]; projects: OrgProject[] }
type OrgStats = { members: number; needs: number; projects: number }
type OrgProject = { id: string; repo: string; note: string; classification: string;
  phase: string | null; needs: number; runsWeek: number }
type MaintainerAsk = { id: string; scope: string; reason: string; resolved: boolean }
```
Every stat is a real number (members / needs / projects), not the hardcoded `0` in `tenantToOrg` and not a fixture length — the domain type is the contract that forces the tenant rollup.

**State** — `org-home-state.svelte.ts` → `orgHomeState`
- data: `home: OrgHome | null`
- `$derived`: `projectCount`, `hasProjects` (list vs `EmptyState`), `visibleNeeds` (`needs.slice(0,2)`, un-resolved first)
- methods: `load(home)`, `resolveAsk(id)` (band `onAct` → optimistic mark + Load mutation)

**Load** — `org-home.ts` → `loadOrgHome(slug)`
- mock-first: hand-crafted `OrgHome` exercising projects / empty / needs-band / resolved (matches `ScrOrgHome`) → build to fidelity NOW (this route makes **no** `/v1` call today)
- real (later, body-swap only): compose the tenant rollup — members `count(dojo.memberships where tenant_id=? and disabled_at is null)`; jurisdiction projects `sensei.namespaces(scope=project) × dojo.seats.tenant_id`; needs from `dojo.triage_queue` (all referenced in APIs above, not restated). No `/summary` endpoint exists — either add `GET …/summary` or compose `/members` + a new `/projects` + `/triage`. Tenant-primary is correct; the members read derives `dojo_url` from `tenant_id → tenants.dojo_url`, not the dropped `memberships.dojo_url` (register §1C).

**Components** (pure, semantic, own styles + `md:` — no `K2*`)
- `OrgJurisdiction` — header: eyebrow (`{org} · jurisdiction`) + title (`{n} projects…`) + `OrgStatRow`
- `OrgStatRow` — three stat badges (members · needs-a-maintainer · in-flight); dropped on mobile
- `MaintainerBand` — `MaintainerAsk[]` (slice 2) with a wired `onAct → orgHomeState.resolveAsk` + resolved chips (today inert)
- `OrgProjectList` + `OrgProjectRow` — repo · note · classification · phase · needs · runsWeek; `onselect → goto(orgHref(slug,'projects')+'/'+id)`
- `EmptyState` (reuse) — honest-empty when `!hasProjects`
- Shell: `+page.svelte` composes; `+page.ts` = Load wiring → `orgHomeState.load`

**Copy** (paraglide `m.<key>()`, no inline literals): `m.org_jurisdiction_eyebrow({org})`, `m.org_projects_count({n})`, stat labels `m.stat_members/stat_needs_maintainer/stat_in_flight()`, band + empty copy. Stat/band glyphs = Solar icons; any kanji brand mark = `KanjiToken`.

**Realtime = State**: no live subscription today; if the needs-band repoints to `dojo.triage_queue`, a `subscribe()` → `patch` would stream queue changes (low priority). **Test seams:** state methods (`resolveAsk`, `visibleNeeds` slice); `OrgProjectRow`/`OrgStatRow`/`MaintainerBand` with mock props; Load mock → shape.
