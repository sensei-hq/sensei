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

## Guardrails carried from F4/F5

Never fall back to another tenant's or fixture data (the acme-fallback lesson); unknown/empty → honest
empty. No fabricated stance/counts. Every mapper is pure + unit-tested; the loader degrades to an error
banner, never a blank or crash.
