# Identity & SSO — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.
- Route: `(app)/org/[slug]/[section]` where `section = identity` → `ScrIdentity` (`+page.svelte` L137-138). (`identities` is the underlying table/endpoint; the URL section is `identity`.)
- Mockup: dojo2-app.jsx `ScrIdentity` (L1269)
- Access axis: tenant-primary — org admin console; identity mappings are per-tenant (`docs/architecture/entity-access-model.md` §3, org console → `tenant_id`). Read route is ADMIN-floor gated.
- Status: PARTIAL — the identity **mappings** and the IdP header are derived from real `dojo.identities` rows (count per provider), but the IdP protocol/status/domain and the SCIM state are synthesized placeholders, the mapping "→ target" is a constant string, and every CTA is inert.

## Elements → data (contract)
| Element | Mockup field | Source (loader/API/table.field) | Status: have/bind/plumb | Realtime? |
|---|---|---|---|---|
| IdP · name | `id.idp.name` | `admin-map.ts toKitIdentity` → `providerLabel(dominant provider)` among `dojo.identities.provider`, else "No provider" (**derived, not a real IdP-connection row**) | plumb | no |
| IdP · protocol | `id.idp.protocol` | `toKitIdentity` → `'SAML / OIDC'` if dominant=`sso` else `'OAuth'` (**synthesized constant**) | plumb | no |
| IdP · status | `id.idp.status` | `toKitIdentity` → `'connected'` if any identity else `'not connected'` (**derived from existence**, not a health check) | plumb | no |
| IdP · domain | `id.idp.domain` | `toKitIdentity` → `` `${identities.length} mapped` `` or `'—'` (**not a real domain**) | plumb | no |
| Configure CTA | (label) | no handler — inert | plumb | — |
| SCIM · enabled | `id.scim` | `toKitIdentity` → **hardcoded `false`** (no SCIM/provisioning source on the wire) | plumb | no |
| Manage CTA | (label) | no handler — inert | plumb | — |
| Mappings count | `id.mappings.length` | number of distinct `dojo.identities.provider` values (real) | have | no |
| Mapping · source | `m.source` | `providerLabel(provider)` — real `dojo.identities.provider` grouped | have | no |
| Mapping · → target | `m.to` | constant `'role from directory access'` for every row (**not a real mapping rule**) | plumb | no |
| Mapping · count | `m.count` | `count(dojo.identities) group by provider` (real) | have | no |
| Add mapping CTA | `onAddMapping` | component accepts `onAddMapping`, but `+page.svelte` passes **only `identity`** → inert (real `POST …/identities` exists, unwired) | plumb | — |
| Edit mapping CTA | (label) | no handler — inert (real `PATCH …/identities/{id}` exists, unwired) | plumb | — |

## APIs / loaders
- **Loader:** `dojo/src/routes/(app)/org/[slug]/[section]/+page.ts`, `guardedFor('identity', …)` → `listIdentities(tenantKey)` → `toKitIdentity`. Degrades to `toKitIdentity([])` + `identityError` on failure. Returns `identity`.
- **Read route:** `dojo/src/routes/v1/t/[origin]/[org]/identities/+server.ts` (GET, ADMIN) → `server/admin-data.ts listIdentities` → `dojo.identities` (eq `tenant_id`, order created_at desc). Cols: `id, user_id, provider, subject, email, display_name, created_at, last_login_at`.
- **Write routes (exist, unwired to UI):** `identities/+server.ts` (POST) · `identities/[id]/+server.ts` (PATCH email/display_name, DELETE). Client `admin-data.ts createIdentity/updateIdentity/deleteIdentity`.
- **Fixture reference** (unused by the live path): `fixtures.ts identity` (Okta/OIDC/SCIM-on/3 mappings) shows the intended shape.

## Interactions & states
- **Degraded** — read failure → empty identity (`No provider` / no mappings) + `identityError`. Honest-empty (DJ1).
- **Non-admin** — 403 → guard degrades to empty; screen renders.
- **Mobile** — 2-col config grid collapses to 1-col (`md:grid-cols-2`). Wired.
- **All CTAs (Configure/Manage/Add mapping/Edit)** — presently inert.

## Gap / to-do (vs mockup)
1. **No IdP-connection model.** The mockup's IdP card (Okta · OIDC · connected · acme.okta.com) and SCIM toggle have **no backing table** — `dojo.identities` is per-user subject mappings, not an org SSO config. Either add a tenant IdP/SCIM settings row or relabel the card to "what we infer from mapped identities" and stop synthesizing protocol/domain.
2. **Mapping target is a constant.** `m.to = 'role from directory access'` for all rows; the mockup implies a real rule (e.g. "GitHub org · acme → role from repo access"). Needs a real mapping-rule source (or derive from provider + policy).
3. **CTAs inert** — wire Add/Edit/Delete to the existing `identities` write endpoints (pass `onAddMapping` etc. from `+page.svelte` → `act()` → `invalidateAll()`, mirroring role-surfaces set-role).
4. **`subject`/`email`/`display_name`/`last_login_at`** are fetched but unused — the mockup's per-mapping detail could surface them.

## Open questions (for Jerry)
- Is org SSO/SCIM a real tenant-settings record we should add (so protocol/status/domain/SCIM are truthful), or is this screen intentionally a read-only projection of the identities that already exist (in which case drop the synthesized IdP/SCIM fields)?
- The mockup's "identity mapping" (GitHub org → auto-join role) is a **provisioning rule**, distinct from `dojo.identities` (per-user subject rows). Do we model mapping rules separately, or keep this screen as the per-provider identity census?
- Should Add/Edit/Delete mapping be wired now (endpoints exist) or wait on the IdP-settings decision above?

### Resolved design (2026-07-30)
- **Q1 → ADD a tenant IdP/SCIM-settings table (WS-3).** New `dojo.idp_settings { tenant_id, protocol, domain, status, scim_enabled, … }` (+ a connection/health check) so the IdP card + SCIM toggle are truthful. Stop synthesizing protocol/status/domain from mere identity-existence; drop the hardcoded `scim=false`. Until it lands, render honestly ("inferred from N mapped identities"), never a fake Okta/OIDC/domain.
- **Q2 → wire the per-user mapping CRUD now** via the existing `dojo.identities` POST/PATCH/DELETE (Add/Edit/Delete mapping through `@rokkit/forms`).
- **Depends on:** new `dojo.idp_settings` DDL + a settings read/write endpoint (WS-3) + the existing `identities` CRUD + WS-1 (`dojo.identities`).

## Components & state (three-layer, per `sensei:ui-state-pattern`)

**Domain types** (UI-shaped; the Load layer maps wire → these):
```ts
type Identity = { idp: IdpConnection | null; scim: ScimState | null;
  mappings: IdentityMapping[] }
type IdpConnection = { name: string; protocol: string; status: string; domain: string }
type ScimState = { enabled: boolean }
type IdentityMapping = { provider: string; source: string; target: string; count: number }
```
`idp`/`scim` are **nullable on purpose**: there is no tenant IdP/SCIM-settings table, so Load returns `null` and the card renders "inferred from N mapped identities" instead of synthesizing Okta / OIDC / a fake domain (kills the `toKitIdentity` placeholders). `mappings` is the real per-provider census; `target` becomes a real rule string once a mapping-rule source exists (today the constant `'role from directory access'`).

**State** — `identity-state.svelte.ts` → `identityState`
- data: `identity: Identity | null`
- `$derived`: `mappingCount`, `hasIdp` (`idp != null`), `connected`
- methods: `load(identity)`, `addMapping(input)`, `editMapping(id, patch)`, `configureIdp()` / `toggleScim()` (no-op until a settings table lands)

**Load** — `identity.ts` → `loadIdentity(tenantKey)`
- mock-first: a mapping census + a mock `IdpConnection`/`ScimState` so the full card builds to fidelity; plus an `idp:null` variant for the honest projection
- real (later, body-swap only): existing ADMIN GET `…/identities` (see APIs above) → group `dojo.identities.provider` for the census (count per provider). IdP/SCIM = `null` until a **tenant IdP/SCIM-settings row** exists (WS-3). Mutations via the existing `identities` POST/PATCH/DELETE (add/edit/delete mapping through `@rokkit/forms`).

**Components** (pure, semantic, own styles + `md:` — no `K2*`)
- `IdentityConsole` shell — 2-col config grid collapsing to 1-col (`md:`)
- `IdpCard` — `IdpConnection` (name · protocol · status · domain) OR the "inferred from mapped identities" projection when `idp==null`; Configure CTA
- `ScimCard` — SCIM toggle; Manage CTA (disabled until backing table)
- `MappingList` + `MappingRow` — provider census (source · target · count); Add/Edit via `@rokkit/forms`
- Provider + status glyphs = Solar icons; any brand kanji = `KanjiToken`

**Copy** (paraglide `m.<key>()`): IdP/SCIM/mapping labels, the honest "inferred from mapped identities" copy, CTA labels. No inline literals.

**Realtime = State**: none. **Test seams:** state methods; `IdpCard` with both `idp` set and `idp:null`, `MappingRow` with mock props; Load mock → shape. Identity is tenant-primary (correct) — no Rule A/B/C touch; its only real blocker is the missing IdP/SCIM-settings table (WS-3).
