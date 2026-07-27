---
title: Dōjō W1 — the fixture→real wiring slice pattern (Constitution worked example)
description: The repeatable recipe for wiring a fixture-backed dōjō surface to real /v1 data on the JWT web plane, plus a per-surface readiness map and the Constitution ladder as the first worked slice.
type: blueprint
status: blueprint
created: 2026-07-27
depends_on:
  - docs/blueprints/2026-07-27-dojo-full-surface.md
references:
  - dojo/src/routes/v1/t/[origin]/[org]/incidents/+server.ts
  - dojo/src/lib/server/rules-data.ts
  - dojo/src/lib/incidents-map.ts
  - dojo/src/routes/(app)/org/[slug]/[section]/+page.ts
---

# Dōjō W1 — the fixture→real wiring slice pattern

## Why this exists

W1 slices are **full vertical features**, not loader flips. A fixture surface renders
`components/kit/fixtures.ts`; making it real means adding a JWT **web-plane** read (the wired consoles'
pattern), not reusing the daemon's **federation-plane** routes (device-token `resolveApiKeyAccess`,
delta/union shapes — e.g. `/v1/rules`). This blueprint pins the recipe once so the remaining surfaces
are mechanical, and grades each surface by how much backend it still needs.

## Two planes (do not confuse them)

| Plane | Auth | Used by | Example |
|---|---|---|---|
| **Federation** | `resolveApiKeyAccess` (device token) | the sensei **daemon** pull/publish | `/v1/rules` (pull deltas), `/v1/rules/resolved` (pack union) |
| **Web (JWT)** | `resolveTenantAccess(origin, org, request, locals, ACCESS.<floor>)` | the dōjō **web app** screens | `/v1/incidents`, `/v1/triage`, … (the wired consoles) |

W1 always adds a **web-plane** read.

## The recipe (7 steps)

1. **Data source.** Find the `dojo.*` table. Is it already populated (the daemon federates it) or does
   it need a table + a federation writer? This is the cost driver (see the readiness map).
2. **Store logic** — `dojo/src/lib/server/<surface>-data.ts`: a pure/injectable read fn over
   `dojoDb()` + a `<Surface>Error`. Kept out of `+server.ts` so it unit-tests without a Worker.
3. **Web read route** — `dojo/src/routes/v1/t/[origin]/[org]/<surface>/+server.ts`:
   `resolveTenantAccess(...)` at the right role floor → the store fn → `Response.json({...})`. Mirror
   `incidents/+server.ts` exactly (the `e instanceof Response` / `<Surface>Error` → `apiError` guard).
4. **Client fn** — `dojo/src/lib/<surface>-data.ts`: `list<Surface>(tenantKey, {fetch, accessToken})`
   → `GET /v1/.../<surface>` → unwrap the payload; throw `DojoApiError` on non-2xx. Mirror `listIncidents`.
5. **Mapper** — `dojo/src/lib/<surface>-map.ts`: wire rows → the Kit shape the screen already takes.
   Pure + unit-tested (the wire→kit contract).
6. **Loader flip** — `(app)/…/+page.ts`: replace `<surface>For(slug)` fixture with the client fn + mapper
   inside the existing `guardTenantScope` try/catch; **honest-empty** when the list is empty, **error
   banner** (not blank/crash) on failure. Never fall back to fixtures.
7. **Tests** — store fn (unit), mapper (unit), and the screen renders real + honest-empty. Zero-errors gate.

## Per-surface readiness map (cost order)

| Surface | `dojo.*` table | Federated? | Cost | Notes |
|---|---|---|---|---|
| **Constitution ladder** | `dojo.shared_rules` ✅ | ✅ (federation writes it) | **LOW** (no DDL) | rules carry `scope_key` + `enforcement`; group → rungs. Stance has no table → honest-empty (follow-up). **← first slice.** |
| **Rule packs** | packs/adoptions (verify which DB) | partial | MED | pack catalog + adoption state; `resolveAdoptedPackRules` exists to lean on |
| **Contributions** | promotions ledger (verify) | verify | MED | personal-scope; needs the ledger exposed on the web plane |
| **org Knowledge** | none | ✗ | HIGH | new `dojo.published_knowledge` + federation + route |
| **org Scopes** | none (policies ≠ scopes) | ✗ | HIGH | new table + federation + route |
| **Projects** (personal+org) | none | ✗ | HIGH | daemon knows projects; needs a `dojo.projects` federation + route |

Sequence cheapest-first: **Constitution → Rule packs → Contributions → Knowledge/Scopes/Projects**
(the HIGH ones each carry a DDL + federation-writer sub-slice on the daemon side).

## Worked example — Constitution ladder (the first slice)

**Data:** `dojo.shared_rules` (already populated; keyed on namespace + content_hash, NOT tenant-scoped —
tenant is the auth boundary). A rule = `{ scope_key, namespace_slug/name, rule_type, title, content,
impact, enforcement }`. `scope_key` ∈ the ladder scopes (company·client·personal·project·stack);
`enforcement` ∈ advisory·recommended·required·mandatory.

- **Store** (`lib/server/constitution-data.ts`): `resolveConstitution(db, tenantNamespaces)` — reuse
  `rules-data`'s `resolveNamespaceIds` + a rules read, returning the tenant's rules. `ConstitutionError`.
- **Route** (`v1/t/[origin]/[org]/constitution/+server.ts`): `resolveTenantAccess(..., ACCESS.member)`
  → `resolveConstitution(dojoDb(), …)` → `Response.json({ rules })`.
- **Mapper** (`lib/constitution-map.ts`): `rulesToLadder(rules): KitLadderRung[]` — group by `scope_key`,
  order company→client→personal→project→stack, each rung = scope label + its rules (title · enforcement
  → tier tone · ★ when mandatory). `rulesToSections(rules): KitConstitutionSection[]` for the org shape.
- **Flip:** personal `you/[section]` (rules → `ladder`, `stance: []` honest-empty) and org
  `org/[slug]/[section]` (rules → `sections`), inside `guardTenantScope`; empty → "no rules yet",
  failure → error banner.
- **Stance** stays honest-empty this slice (no `dojo.stance` table; the daemon has the write-path but the
  web plane needs its own source) — a tracked follow-up, not fabricated dials.
- **Tests:** `resolveConstitution` (store), `rulesToLadder`/`rulesToSections` (mapper — scope order,
  enforcement→tone, mandatory lock, empty→[]), screen renders real ladder + honest-empty.

## Constitution slice — confirmed read-path depth (2026-07-27 trace)

Traced end-to-end; the slice is genuine Tier-3 backend work, **not** a loader flip. Facts:

- **Data:** `dojo.shared_rules ⋈ sensei.namespaces`. Rule = `title` · `content` · `enforcement`
  (advisory·recommended·required·mandatory) · `rule_type` · `impact` · `status`. Namespace =
  `scope_key` · `slug` · `name`. **`scope_key` is a FK to `sensei.scopes.key`** — a data-driven
  scopes TABLE (seeded `general` + others), **not a fixed enum**. So the `scope_key`→kit-ladder
  mapping (Company 社 · Client 客 · Personal 己 · Project 件 · Stack 技; sections group Company/
  Teams 組/Stacks) must be built against the **confirmed** `sensei.scopes` keys — guessing
  mislabels the ladder.
- **The existing `/v1/rules*` routes are the daemon FEDERATION plane** (device-token
  `resolveApiKeyAccess`; pull-deltas / pack-union), **not** a JWT web read the screen can call.
- **Tenant's display constitution = rules at the tenant's namespaces**, resolved via the
  **adoptions model** (`resolveAdoptedPackRules(db, nsIds)` + `effectivePackRuleTier`/`maxTier`).
  shared_rules is NOT tenant-scoped — the tenant→namespaces resolution is the real work.

**Build-ready steps (do these, in order, next session):**
1. **Confirm the scope vocabulary** — read `database/ddl/table/sensei/scopes.ddl` + the scopes seed;
   list the real `key`s → fix the `scope_key`→{label, kanji, group, order} map.
2. **Tenant→namespaces (the real design step — DO NOT shortcut).** Resolve the tenant's FULL
   namespace set across scopes, not just its org namespace. A constitution spans
   `(organization, <org>)` + `(team, <team>)` + `(technology, <stack>)` — **different slugs per
   scope**, not derivable from the org slug alone. Filtering by `slug = <org>` would render ONLY the
   Company section and silently drop real team/stack rules → a misleadingly-partial constitution
   (the acme-fallback lesson). Needs the **ownership/adoption lookup** (which namespaces this tenant
   owns/adopted) — a small `constitution-data.ts` store fn composing that set → `resolveNamespaceIds`
   + a `shared_rules` read + `resolveAdoptedPackRules`. Best built + **browser-verified against a
   real seeded dōjō** (the wrangler recipe — F6-adjacent), since a partial read fails silently.
3. **Web read route** `v1/t/[origin]/[org]/constitution/+server.ts` (`resolveTenantAccess`, member
   floor) → `{ rules }`.
4. **Mapper** `constitution-map.ts`: `rulesToLadder` (KitLadderRung[]) + `rulesToSections`
   (KitConstitutionSection[], grouped Company/Teams/Stacks) — pure, unit-tested (scope order,
   `enforcement==='mandatory'`→`hard`, empty→[], unknown-scope fallback).
5. **Flip** the personal `you/[section]` (rules→ladder; `stance: []` honest-empty) + org
   `org/[slug]/[section]` (rules→sections) loaders; honest-empty + error banner.

Stance stays honest-empty (no `dojo.stance` table; the daemon has the write-path but the web plane
needs its own source) — a tracked follow-up, not fabricated dials.

## Guardrails carried from F4/F5

Never fall back to another tenant's or fixture data (the acme-fallback lesson); unknown/empty → honest
empty. No fabricated stance/counts. Every mapper is pure + unit-tested; the loader degrades to an error
banner, never a blank or crash.
