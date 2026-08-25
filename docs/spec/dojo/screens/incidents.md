# Incidents — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.
- Route: `/org/[slug]/incidents` — `(app)/org/[slug]/[section]` with section=`incidents`. Read: `GET /v1/t/{origin}/{org}/incidents` (LEAD floor). Writes: `POST` / `PATCH …/{id}` / `DELETE …/{id}`.
- Mockup: dojo2-app.jsx `ScrIncidents` (L1208)
- Access axis: tenant-primary — org lead console, keyed by `tenant_id` (entity-access-model §3). All calls `resolveTenantAccess(..., ACCESS.lead)` → `.eq('tenant_id', …)`. Confidentiality incidents are containment records for client work; the underlying artifacts obey the always-on universal strip (canon §5), incidents log that a near-leak was held.
- Status: PARTIAL — list + report/resolve/delete wired to real lead endpoints; but the client name is only a truncated engagement id (no name join), and the retention/read-access footer chips are hardcoded.

## Elements → data (contract)
| Element | Mockup field | Source (loader/API/table.field) | Status: have/bind/plumb | Realtime? |
| --- | --- | --- | --- | --- |
| Header count | `D2.consoles.incidents.length` | `data.incidents.length` ← `listIncidents(tenantKey)` → `GET …/incidents` → `dojo.incidents` | have | no |
| Banner "Contain a near-leak fast" | static copy | literal in `ScrIncidents.svelte` | have (static) | no |
| Row glyph | `it.kanji` (盾) | constant `INCIDENT_KANJI='盾'` in `incidents-map.ts::toKitIncident` | have (const) | no |
| Row title | `it.title` | `dojo.incidents.title` → `Incident.title` → `KitIncident.title` | have | no |
| Row client | `it.client` | `incidents.engagement_id.slice(0,8)` else `'—'` (`toKitIncident`) | **bind** — shows a short uuid, not the client name; join `engagement_id → engagements.client_name` (needs the Rule C column) | no |
| Row severity chip | `it.severity` | `dojo.incidents.severity` (`low`/`medium`/`high`/`critical`); tone via `incidents-view.ts::severityTone` | have | no |
| Row state dot | `it.state` | `incidentState()`: `resolved_at≠null`\|status=resolved→`resolved`; status=investigating→`contained`; else `open`. Tone via `stateToneClass` | have | no |
| Row "when" | `it.when` | `relativeAge(dojo.incidents.opened_at)` | have | no |
| Ordering | worst-first | `shapeIncidents` sort: `SEVERITY_RANK` then `opened_at desc` (server) | have | no |
| "Report" button | — | `createIncident(tk, {title})` → `POST …/incidents` | have (title-only) | no |
| "Open" row button | `<K2Btn>Open` | `onOpen(it)` prop — **no handler passed** in `+page.svelte` | **plumb** — incident detail view unbuilt (`…/incidents/{id}`) | no |
| Footer "Retention · 1 year" | static chip | hardcoded in `ScrIncidents.svelte` | **bind/plumb** — real source = tenant retention policy (`policy_overrides`/`dojo.policies`), not wired | no |
| Footer "Client read-access · off" | static chip | hardcoded in `ScrIncidents.svelte` | **bind/plumb** — same, not wired | no |

## APIs / loaders
- Loader `+page.ts` → `guardedFor('incidents', [], listIncidents, list => toKitIncidents(list.incidents))`, behind `guardTenantScope`. Degrades to `[]` + `incidentsError` on failure. (`open_count` from the envelope is fetched but not surfaced on this screen.)
- Read API: `GET /v1/t/{origin}/{org}/incidents` (`incidents/+server.ts`, `ACCESS.lead`) → `{ incidents: Incident[], open_count }`, worst-first, `open_count = count(resolved_at is null)`. Cols: `id, engagement_id, artifact_id, title, description, severity, status, owner_id, sla_due_at, resolution, opened_at, resolved_at`.
- Write APIs (lead-gated, `+page.svelte` `act()`→`invalidateAll()`): `POST …/incidents` (`parseNewIncident`, severity defaults `medium`; records `incident_opened` audit row); `PATCH …/incidents/{id}` (`parsePatchIncident`: `resolved:true`/`status:'resolved'` stamps `resolved_at`, reopen clears it); `DELETE …/incidents/{id}`.
- Store logic: `dojo/src/lib/server/incidents-data.ts`. Client: `client-data.ts`. Mapper: `incidents-map.ts`. View tones: `incidents-view.ts`.

## Interactions & states
- Report → `window.prompt('New incident — title?')` → `onReport(title)` → POST (severity server-defaults `medium`; owner/engagement/SLA not captured).
- Resolve → shown when `it.state !== 'resolved'` → PATCH `{resolved:true}` (stamps `resolved_at`).
- Delete → DELETE (hard).
- Open → button rendered but no callback wired (inert).
- Empty → dedicated `EmptyState` ("No incidents on record.").
- Failure → dismissible danger toast; loader failure → empty list + surfaced error.

## Gap / to-do (vs mockup)
- Client column shows a truncated engagement uuid, not a client name — join through `engagement_id`.
- Incident detail / "Open" flow unbuilt (owner, SLA, resolution, artifact link).
- Report captures only a title — no severity/owner/engagement/SLA capture (all supported by `parseNewIncident`/DDL).
- Retention + client-read-access footer chips are hardcoded — bind to tenant policy or mark illustrative.
- `sla_due_at` breach alerting (wrong-gate "open past SLA without an alert" in lead-console spec) not surfaced.

## Open questions (for Jerry)
- Should the row show the client name (join `engagement_id → engagements.client_name`) instead of a short uuid — depends on the Rule C `client_name` column landing.
- Is the incident detail view (owner/SLA/resolution/artifact) in scope for this pass, or list-only for now?
- Are retention + client read-access real per-tenant policy values (surface from `dojo.policies`/`policy_overrides`) or fixed display copy?
- SLA breach alerting — in scope here, or a separate Health/monitor concern?

### Resolved design (2026-07-30)
- **Q1 client name → join `engagement_id → dojo.engagements.client_name`** (Rule C, agreed); short-id/`'—'` until `client_name` lands.
- **Q2 detail view → IN SCOPE:** build the incident detail pane (owner / SLA / resolution / linked artifact) via a detail read.
- **Q3 retention + client read-access chips → REAL from `dojo.policies`/`policy_overrides`** (per-tenant values), not hardcoded copy.
- **Q4 SLA-breach alerting → a HEALTH concern.** This screen shows per-incident SLA + breach state (`isSlaBreached`, already computed); proactive SLA-breach ALERTING lives on the Health alert feed.
- **Depends on:** Rule C (`client_name`) + an incident detail read + `dojo.policies` (chips) + WS-1 (owner name).

## Components & state (three-layer, per `sensei:ui-state-pattern`)

**DB** — `dojo.incidents` (tenant-scoped; cols in *Elements → data* / *APIs* above — not restated). Client name resolves via `engagement_id → dojo.engagements.client_name` (depends on the Rule C column, register 1C — until it lands the row keeps the honest short-id / `'—'`). Retention + client-read-access footers come from `dojo.policies`/`policy_overrides`, not literals. Tenant-scoped `tenant_id`; an incident logs that a near-leak was *held* — the underlying artifact strip is the always-on universal dereference (canon §5), never a per-incident toggle.

**API** — reuse the documented `GET/POST/PATCH/DELETE …/incidents` (`ACCESS.lead`, worst-first + `open_count`); the client-name join additionally needs the engagements read.

**UI** (components / state / types):

**Domain types** (UI-shaped; Load maps wire → these):
```ts
type Incident = { id; engagementId: string|null; clientName: string|null; artifactId: string|null;
  title: string; description: string|null; severity: 'low'|'medium'|'high'|'critical';
  state: 'open'|'contained'|'resolved'; owner: string|null; slaDueAt: string|null;
  resolution: string|null; when: string }
```
`clientName` (joined) replaces the truncated-uuid `it.client`. `severity` (status descriptor) and the report/resolve **actions** are kept distinct — consistent with the inbox status-vs-action rule.

**State** — `incidents-state.svelte.ts` → `incidentsState`
- data: `incidents: Incident[]`, `retention`/`clientReadAccess` (from policy), `error`
- `$derived`: `count`, `openCount`, `worstFirst` (server already sorts; keep the client sort stable)
- methods: `load({ incidents, retention, clientReadAccess })`, `report(input)`, `resolve(id)`, `reopen(id)`, `remove(id)`

**Load** — `incidents.ts` → `loadIncidents(tenantKey)`
- mock-first: hand-crafted `Incident[]` spanning severities × states, with/without client → fidelity NOW
- real (body-swap only): `listIncidents` + join `engagement_id → engagements.client_name`; footer values from `dojo.policies`

**Components** (pure, semantic, own styles + `md:`) — replace `ScrIncidents`:
- `IncidentList` — shell: `SectionHead` + banner + `IncidentRow[]` + Report + policy footer chips (Retention / Client-read-access from state, not literals)
- `IncidentRow` — `KanjiToken 盾` (Solar icons for severity/state/action glyphs), title, clientName, severity chip (tone via `incidents-view.ts`), state dot, when; Resolve (state≠resolved) / Delete / Open → state methods. **Mockup-match + `md:` here.**
- Report form via `@rokkit/forms` (title required, severity/owner/engagement/SLA) — replaces `window.prompt`
- `IncidentDetail` — follow-on for `…/incidents/{id}` (owner/SLA/resolution/artifact); currently unbuilt

**Copy** — paraglide `m.<key>()` (banner "Contain a near-leak fast", labels, empty/error); `盾` stays a `KanjiToken` brand mark; footer chip copy sourced from policy.

**Realtime = State**: none today — refreshes via `invalidateAll` after writes. **Test seams:** state methods (no DOM); `IncidentRow` with a mock `Incident`; Load mock → shape.

**New open question:** same Rule-C dependency — surface `clientName` only once `engagements.client_name` lands (short-id/`'—'` until then); and is `IncidentDetail` in scope this pass or list-only?
