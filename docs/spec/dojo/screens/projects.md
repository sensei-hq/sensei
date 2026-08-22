# Projects — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.

- Route: `/you/projects` = `(app)/you/[section]/+page.svelte` (`section === 'projects'` branch) + `[section]/+page.ts`
- Mockup: dojo2-app.jsx `ScrProjects` (L119) — board "1b · Your projects (list)"
- Access axis: **user/membership-primary** — canonical `entity-access-model.md` §3 row 2: "Projects, contributions (`/you`) … Primary axis = User … user's projects / memberships; project↔membership binding is the routing, not the access filter." Spans every repo the user touches, not one tenant.
- Status: **STUB** — the screen (`ScrProjects.svelte`) is built and renders faithfully, but the loader returns `projects: []` (honest-empty). There is **no `/v1` projects endpoint** and no dōjō projects table; every field would need new plumbing.

## Elements → data (contract)
Live: `[section]/+page.ts` → `{ projects: [] }` → `ScrProjects.svelte` (`showDojo={false}`, `onOpenProject`→`/you/projects/[id]`).

| Element | Mockup field | Source (loader/API/table.field) | Status | Realtime? |
|---|---|---|---|---|
| SectionHead eyebrow | `Everything in flight` / `You · every project you touch` | static (`ScrProjects` default eyebrow) | have | — |
| SectionHead title | `Your projects` | static | have | — |
| header count | `projects.length` | `data.projects.length` (= 0) | have (empty) | — |
| Filter btn (right) | `tuning-2 Filter` | static, **not wired** (no filter logic) | plumb | — |
| flush card list | `projects.map(ProjectRow)` | `data.projects` — **empty; NO source** | plumb | — |
| row: name | `p.name` | project registry — **none in dōjō** (candidate: daemon `sensei.projects.name`, not federated) | plumb | — |
| row: repo | `p.repo` (`acme/lumen-auth`) | `sensei.projects` git path / slug — not federated | plumb | — |
| row: classification chip | `p.classification` company/client/personal/community | `sensei.projects.classification` — not federated | plumb | — |
| row: phase pill | `p.phase` watch/notice/adopt | not federated | plumb | — |
| row: owning-dōjō (showDojo) | `p.dojoName` | project↔membership `dojo_id` binding — not federated; here `showDojo=false` | plumb | — |
| row: lastRun | `p.lastRun` (`8m`) | derivable from `relay_sessions.last_event_at` grouped by project (needs project on the session, see inbox §4) | plumb | maybe |
| row: needs badge | `p.needs` | count of pending asks for the project's runs | plumb | maybe |
| row: note | `p.note` | project signal summary — not federated | plumb | — |
| row: spark / runsWeek | `p.spark` / `p.runsWeek` | run cadence from `relay_sessions` grouped by project | plumb | — |
| empty state | (mockup always has rows) | `EmptyState 空 No projects yet` — dōjō-added honest-empty | have | — |

## APIs / loaders
- **load()** (`[section]/+page.ts`): section-guard (`YOU_SECTIONS`), then returns `projects: []` with the comment "No backing route yet → honest empty, never fabricated (F4)." No fetch.
- **mutations**: none.
- **realtime**: none. (If lastRun/needs are derived from `relay_sessions`, they could ride the relay realtime channel later.)
- **Would-be source**: a new `GET /v1/…/projects` (user-wide, membership-joined) backed by federating the daemon's `sensei.projects` registry into the dōjō, OR deriving a project list from distinct `relay_sessions.project_slug` (once persisted — see `inbox.md` Gap 4). Neither exists.

## Interactions & states
- **Open a project** → `onOpenProject(p)` → `goto('/you/projects/' + p.id)` → `project-detail` drill-in. Works structurally; nothing to open while empty.
- **Empty**: renders the honest-empty state ("Projects appear here once you join a dōjō or sensei starts watching a repository…").
- **Filter**: button present, no behavior.
- **Responsive**: `ScrProjects` takes a `mobile` prop (stacked rows, `showDojo` off); this route does not pass `mobile` — the `(app)` shell swaps AppShell/MobileShell instead, and `ProjectRow` `compact` is unused here. Confirm phone rows still read (G3).

## Gap / to-do (vs mockup), ranked
1. **No data source** — decide the projects source: (a) federate `sensei.projects` into the dōjō via a new `GET /v1/…/projects`, or (b) derive from `relay_sessions` (distinct project once `project_slug` is persisted). Everything else blocks on this.
2. **Access axis** — the endpoint must be user-wide (all memberships), keyed off the user, not a single `tenant_id` — same correction as the inbox (`owns_membership`).
3. **Classification / phase / note / spark** — these are daemon-side project attributes; confirm which federate to the dōjō vs stay desktop-only (the drill-in preview also needs classification — see `project-detail.md`).
4. **Filter** control — define the filter axes (by dōjō / classification / phase) or drop the button.
5. **Confidentiality** — repo names + paths crossing into the dōjō are subject to universal source-dereference (`entity-access-model.md` §5). A personal projects list showing real repo names to the user's own dōjō is local-context (not cross-boundary), but a federated `projects` row that any org member could read must be dereferenced.

## Open questions (for Jerry)
1. Where do projects come from for the dōjō web app — federate the daemon `sensei.projects` registry, or derive a thin list from relay sessions? (The desktop app owns the rich project registry; the dōjō may only ever need name + last-run.) - [JT] Derive a thin list from relay sessions.+ publish active user repos.
2. Is a personal cross-dōjō projects list even in scope pre-release, or does it stay honest-empty until the desktop app is the projects surface and the dōjō only shows in-flight runs (the Inbox)? [JT] - personal projects yes. Cross dojo -no projects have single owner personal|org (client|employer). 
3. If federated: what's the dōjō-visible project identity under source-dereference — a stable dereferenced slug (so rows are clickable/consistent) or fully opaque? [JT] federate user's active projects
4. `KitProject.id` used for the drill-in URL — what is the stable id when the source is `relay_sessions` (no project PK)? A dereferenced project slug? [jt] a project slug

### Resolved design (2026-07-30)
- **Source of truth:** a **new lightweight `dojo.projects` table** (NOT derive-on-read from `relay_sessions`). Single-owner rows (`personal | org(client|employer)`), user-scoped.
- **Population seam:** on any project that has a relay run, the **daemon sends the project info alongside the plan payload** — `{ slug (dereferenced), name, classification, phase }` — and the dōjō upserts a `dojo.projects` row. Shares the `project_slug` persistence with `inbox.md` Gap 4.
- **Read:** `GET /v1/…/projects` selects the user's `dojo.projects` rows (user-wide across the user's memberships — rides **WS-0 Rule A**). Drill-in key = the dereferenced `slug`.
- **Payload includes `classification` + `phase`** (feeds constitution-ladder-by-classification + phase).
- **Depends on:** Rule A (user-wide personal read) + the new `dojo.projects` DDL + the daemon plan-payload extension.

## Components & state (three-layer, per `sensei:ui-state-pattern`)

**DB** `relay_sessions` grouped by `project_slug` (once persisted — see `inbox.md` Gap 4) + published active-user repos; no dōjō projects table, read-only · **API** Load `loadProjects` in `projects.ts` (today `→ []`); would-be `GET /v1/…/projects` (user-wide, `owns_membership`) · **UI** `projectsState` + `ProjectsList`/`ProjectCard` over the `Project` domain type.

**Domain types** (UI-shaped; Load maps the thin relay/repo rows → these):
```ts
type Project = { id: string /* dereferenced slug = drill-in key */; name; repo;
  classification?: 'company'|'client'|'personal'|'community'; phase?: 'watch'|'notice'|'adopt';
  dojoName?: string; lastRun?: string; needs: number; note?: string; runsWeek?: number;
  spark?: number[] }
type ProjectFilter = { classification?: Project['classification']; phase?: Project['phase'] }
```
`classification`/`phase`/`note`/`spark` are daemon-side attributes → **optional**: a thin relay-derived
`Project` may omit them and `ProjectCard` renders each chip only when present (never a fabricated value).

**State** — `projects-state.svelte.ts` → `projectsState`
- data: `projects: Project[]`, `filter: ProjectFilter`
- `$derived`: `shown` (apply `filter`), `count`
- methods: `load(projects)`, `setFilter(f)` — no `select`; a row navigates via `goto('/you/projects/'+id)`

**Load** — `projects.ts` → `loadProjects()`
- mock-first: hand-crafted `Project[]` exercising company/client/personal/community · watch/notice/adopt ·
  empty · error → build `ProjectCard` + tests to mockup fidelity NOW
- real (body-swap only): **user-wide** read (all memberships, keyed off the user) — distinct projects from
  `relay_sessions.project_slug` + published active repos → `Project[]`. Repo names on a *federated*
  (org-readable) row are source-dereferenced (§5); the user's own list is local-context.

**Components** (pure, semantic, own styles + `md:` — fidelity verified per component)
- `ProjectsList` — `SectionHead` + filter control + `ProjectCard[]` from `projectsState.shown` +
  `EmptyState` when none. (replaces `ScrProjects`; `showDojo=false` on the personal list)
- `ProjectCard` — one `Project`: name · repo · classification chip · phase pill · lastRun · needs badge ·
  note · spark; `onopen→goto`. **Mockup-match + `md:` live here.** Solar icons for classification/phase/needs
  (not kanji). (replaces `ProjectRow`)
- Filter: if it graduates from the inert `tuning-2` button to real axes, a schema-based `@rokkit/forms`
  control (selectable classification/phase/dōjō) → `projectsState.setFilter`; otherwise drop the button.

**Copy** (paraglide `m.<key>()` from `$lib/paraglide/messages`, no inline literals): `m.projects_title()`/
`m.projects_eyebrow()`, classification + phase labels, `m.projects_empty()` honest-empty copy. Kanji (空)
stays a `KanjiToken` brand mark.

**Realtime = State**: none today; if `lastRun`/`needs` derive from `relay_sessions`, a later `patch(project)`
can ride the relay channel. **Test seams:** `projectsState` filter logic (no DOM); `ProjectCard` with a mock
`Project` (each optional field present/absent); Load mock → shape.

**New open question (from this exercise):** the thin relay-derived `Project` (JT: "derive from relay sessions
+ publish active repos") carries no `classification`/`phase` — but `project-detail.md` composes the
constitution ladder *by classification*. Does the published-repo payload include `classification` + `phase`, [jt] yes
or does the drill-in preview degrade/become unreachable for relay-only projects? The shared `Project` type is
the seam both screens depend on. [jt] any project that has a relay run we can send the info from sensei to dojo along with the plan payload.
