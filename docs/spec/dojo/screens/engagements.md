# Engagements — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.
- Route: `/org/[slug]/engagements` — `(app)/org/[slug]/[section]` with section=`engagements`. Read: `GET /v1/t/{origin}/{org}/engagements` (LEAD floor). Writes: `POST` / `PATCH …/{id}` / `DELETE …/{id}` / `POST …/{id}/bind`.
- Mockup: dojo2-app.jsx `ScrEngagements` (L1165)
- Access axis: tenant-primary — the org console is a specific dōjō, keyed by `tenant_id` (entity-access-model §3 "Org console (`/org/[slug]`) → Tenant → `tenant_id`"). All reads/writes are `resolveTenantAccess(..., ACCESS.lead)` → `.eq('tenant_id', …)`. This is genuinely tenant-scoped client work, so **universal source-dereference is CRITICAL** on everything that leaves (canon §5): the strip is always-on, no per-item lead override.
- Status: PARTIAL — register list + create/close/delete wired to real lead endpoints; but lessons/dropped counts are hardcoded 0, the confidentiality panel is a static fixture, and the `client` column still needs the `client_tenant_id`+`client_name` split.

## Elements → data (contract)
| Element | Mockup field | Source (loader/API/table.field) | Status: have/bind/plumb | Realtime? |
| --- | --- | --- | --- | --- |
| Header count | `D2.consoles.engagements.length` | `data.engagements.length` ← `listEngagements(tenantKey)` → `GET …/engagements` → `dojo.engagements` rows | have | no |
| Banner "Share the lesson, never the source" | static copy | none (literal in `ScrEngagements.svelte`) | have (static) | no |
| Row glyph | `e.kanji` (客) | constant `CLIENT_KANJI='客'` in `client-map.ts::toKitEngagement` | have (const) | no |
| Row client name | `e.client` | `dojo.engagements.client` → `Engagement.client` → `KitEngagement.client` | **plumb** — schema fix: `client text` → `client_tenant_id uuid` + `client_name text` (register Rule C, `engagements.ddl` L4). Repoint `client-map.ts`, `engagements/+server.ts` COLS + POST body, `client-data.ts` `Engagement.client` | no |
| Row projects | `e.projects` | `bindingsLabel(dojo.engagements.project_bindings jsonb [{project_id,name}])` | have | no |
| Row "since" | `e.since` | `relativeAge(engagements.starts_on ?? created_at)` | have | no |
| Row "lessons kept" | `e.lessons` | **hardcoded `0`** in `toKitEngagement` — real source is per-engagement kept-artifact count | **plumb** — needs `count(dojo.artifacts where engagement_id=x and status published)` via `GET …/audit/artifacts` aggregate; not fetched by the list loader | no |
| Row "stripped" | `e.dropped` | **hardcoded `0`** in `toKitEngagement` — real source is per-engagement stripped/held count | **plumb** — same `…/audit/artifacts` aggregate | no |
| Row status (drives Close visibility) | `e.status` | `dojo.engagements.status` (`active`/`ended`) | have | no |
| "Audit" row button | `<K2Btn icon=document>Audit` | — (no handler; navigates nowhere) | **plumb** — should open per-engagement artifact audit (`…/audit/artifacts?engagement=`), unbuilt | no |
| Confidentiality "What crosses" kept/dropped | `c.kept[]` / `c.dropped[]` | `data.confidentiality` ← `confidentialityFor(slug)` **fixture** (`kit/fixtures.ts` L1069) | **bind/plumb** — static kit constant, no tenant source. Real source = tenant confidentiality policy pack (`policy_overrides` / `dojo.policies`), not wired | no |
| Confidentiality raw→stripped example | `c.example.raw/.stripped` | same fixture constant | have (static illustrative) | no |
| "New engagement" button | — | `createEngagement(tk, {client})` → `POST …/engagements` | have (name-only) | no |

## APIs / loaders
- Loader `(app)/org/[slug]/[section]/+page.ts` → `guardedFor('engagements', [], listEngagements, toKitEngagements)`. Runs behind `guardTenantScope(org.url, …)`; degrades to `[]` + `engagementsError` on 403 (non-lead) / 404 (dev) / 500. `confidentiality: confidentialityFor(slug)` is fixture.
- Read API: `GET /v1/t/{origin}/{org}/engagements` (`engagements/+server.ts`, `ACCESS.lead`) → `{ engagements: Engagement[] }`, `order created_at desc`. COLS: `id, client, description, project_bindings, policy_overrides, status, starts_on, ends_on, created_at, updated_at`.
- Write APIs (all lead-gated; handlers in `+page.svelte` via `act()` → `invalidateAll()`): `POST …/engagements` (create, `client` required); `PATCH …/engagements/{id}` (close = `status:'ended'`, via `updateEngagement`/`parsePatchEngagement`); `DELETE …/engagements/{id}` (hard delete); `POST …/engagements/{id}/bind` (`bindEngagementProject`/`mergeBinding`, idempotent on project_id) — **server logic exists, no UI**.
- Store logic: `dojo/src/lib/server/engagements-data.ts`. Client: `dojo/src/lib/client-data.ts`. Mapper: `dojo/src/lib/client-map.ts`.

## Interactions & states
- New engagement → `window.prompt('New engagement — client name?')` → `onNew(client)` → POST. Minimal (no dates/description/policy_overrides capture).
- Close → shown only when `e.status !== 'ended'` → PATCH `status:'ended'` (retained for audit).
- Delete → DELETE (hard).
- Audit button → present but inert.
- Failure → dismissible danger toast (`actionError`), never silent (feedback_no_silent_errors).
- Empty list → renders the `ListSection` with no rows (no dedicated EmptyState in `ScrEngagements.svelte`).
- Loader failure → empty register + surfaced error; screen always renders (DJ1 honest-empty).

## Gap / to-do (vs mockup)
- `lessons`/`dropped` are `0` — wire the per-engagement kept-vs-stripped artifact counts (`…/audit/artifacts` aggregate).
- Confidentiality panel is a global fixture — bind to the tenant's real policy/strip config or mark explicitly illustrative.
- Schema: split `client` → `client_tenant_id` + `client_name` (Rule C) and repoint reader/writer/mapper.
- Bind-project has server + client fns but no UI affordance.
- Create flow captures only a name (no start/end date, description, policy overrides).
- "Audit" button should deep-link to the engagement's artifact audit view.

## Open questions (for Jerry)
- Migrate `engagements.client` → `client_tenant_id` (FK to `dojo.tenants`, nullable) + `client_name` now, or defer past the stable release? (register lists it under Part 1 schema fixes.)

### Resolved design (2026-07-30)
- **Q1 client split → MIGRATE NOW (WS-0 Rule C, agreed):** `dojo.engagements.client text` → `client_tenant_id uuid` (FK `dojo.tenants`, nullable) + `client_name text not null`. Repoint `client-map.ts`, `engagements/+server.ts` COLS + POST body, `client-data.ts` `Engagement` type.
- **New-Q kept/stripped → COMPUTE the per-engagement aggregate now:** `lessonsKept = count(dojo.artifacts where engagement_id=x and status='published')`, `stripped = count(... held)` — a small aggregate read alongside `listEngagements`. No hardcoded 0.
- **Confidentiality panel → tenant `dojo.policies`/`policy_overrides`** (real), NOT the `confidentialityFor(slug)` fixture. Universal dereference is always-on (Rule B): no per-item override, no `dereferenced` mode; `attribution_mode = named | anonymous` (credit only). The panel only DISPLAYS what always crosses.
- **Depends on:** Rule C schema migration + the per-engagement aggregate read + `dojo.policies` for confidentiality.
- Where do `lessons kept` / `stripped` counts come from — a live aggregate over `dojo.artifacts` per engagement, or a denormalized counter? Is a per-engagement `…/audit/artifacts` count endpoint in scope for this screen?
- Is the confidentiality kept/dropped/example panel a fixed product statement (leave static) or should it reflect the tenant's adopted compliance pack (`policy_overrides`)?

## Components & state (three-layer, per `sensei:ui-state-pattern`)

**DB** — `dojo.engagements` (tenant-scoped; cols in *Elements → data* / *APIs* above — not restated). **Rule C schema fix (register 1C):** `client text` → `client_tenant_id uuid` (FK `dojo.tenants`, nullable) + `client_name text not null`; the domain type below is already shaped to it. Count sources = a per-engagement aggregate over `dojo.artifacts` (`lessonsKept` = published, `stripped` = held); confidentiality = tenant `dojo.policies`/`policy_overrides`, not the `confidentialityFor(slug)` fixture. **Universal dereference (canon §5) is CRITICAL and always-on** here: everything leaving is source-stripped unconditionally — there is no per-item lead override and no `dereferenced` mode; `attribution_mode = named | anonymous` is credit only.

**API** — reuse the documented `GET/POST/PATCH/DELETE …/engagements` + `POST …/{id}/bind` (all `ACCESS.lead`); the kept/stripped counts additionally need a `…/audit/artifacts` per-engagement aggregate the list loader does not yet fetch.

**UI** (components / state / types):

**Domain types** (UI-shaped; Load maps wire → these):
```ts
type Engagement = { id; clientTenantId: string|null; clientName: string; description: string|null;
  projects: { id; name }[]; status: 'active'|'ended'; since: string;
  lessonsKept: number; stripped: number }
type Confidentiality = { kept: string[]; dropped: string[]; example: { raw: string; stripped: string } }
```
`clientName` replaces the flat `client` (Rule C). No `attribution_mode`/strip toggle on the type — the panel only *displays* what always crosses; the strip is unconditional.

**State** — `engagements-state.svelte.ts` → `engagementsState`
- data: `engagements: Engagement[]`, `confidentiality: Confidentiality|null`, `error`
- `$derived`: `count`, `active` (status≠ended)
- methods: `load({ engagements, confidentiality })`, `create(input)`, `close(id)`, `remove(id)`, `bindProject(id, project)` — mutations wrap the write endpoints → re-`load` (keep the honest error toast, never silent)

**Load** — `engagements.ts` → `loadEngagements(tenantKey)`
- mock-first: hand-crafted `Engagement[]` (active/ended, bound/unbound, real kept/stripped counts) + a `Confidentiality` sample → build register + panel to fidelity NOW
- real (body-swap only): `listEngagements` + the `…/audit/artifacts` aggregate for kept/stripped + tenant policy for `confidentiality` (replaces the fixture); map `client_name`/`client_tenant_id`

**Components** (pure, semantic, own styles + `md:`) — replace `ScrEngagements`:
- `EngagementRegister` — shell: `SectionHead` + banner + `EngagementRow[]` from `engagementsState` + New button
- `EngagementRow` — one `Engagement`: `KanjiToken 客` (Solar icon for action glyphs), clientName, project chips, since, lessonsKept/stripped, status; Close (status≠ended) / Delete / Audit / Bind → state methods. **Mockup-match + `md:` live here.**
- `ConfidentialityPanel` — kept/dropped lists + raw→stripped example from `confidentiality`
- New-engagement + Bind forms via `@rokkit/forms` (schema: `clientName` required, dates, description, policy_overrides) — replaces `window.prompt`

**Copy** — paraglide `m.<key>()` (banner "Share the lesson, never the source", labels, empty/error); `客` stays a `KanjiToken` brand mark; universal-dereference wording only (no client-only framing).

**Realtime = State**: none today — register refreshes via `invalidateAll` after writes (keep). **Test seams:** state methods (no DOM); `EngagementRow`/`ConfidentialityPanel` with a mock prop; Load mock → shape.

**New open question:** is the `…/audit/artifacts` per-engagement count aggregate in scope this pass (feeds `lessonsKept`/`stripped`), or do the counts stay honest-zero until it lands?
