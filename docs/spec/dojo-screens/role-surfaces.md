# Role surfaces (Members / Policies / Audit) — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.
- Route: `(app)/org/[slug]/[section]` where `section ∈ {members, audit}` → `ScrRoleSurfaces` (`+page.svelte` L125-134; `roleTab = tabForSection(section)` opens Members vs Audit). The **Policies** tab is a third in-screen tab with no own URL.
- Mockup: dojo2-app.jsx `ScrRoleSurfaces` (L775) + `MemberRow` (L758)
- Access axis: tenant-primary — org console members/audit, `docs/architecture/entity-access-model.md` §3 ("Org console … members · … audit → Tenant → tenant_id"). All three read routes are ADMIN-floor gated (`ACCESS.admin`).
- Status: PARTIAL — Members and Audit tabs bind **real** `/v1` data (`dojo.memberships` / `dojo.audit_events`) and set-role writes live; but member display names are shortened uuids (no identity join), "you" never resolves (self undefined), scopes are "—", and the Policies tab is a hardcoded ladder rather than the tenant's `dojo.policies`.

## Elements → data (contract)
| Element | Mockup field | Source (loader/API/table.field) | Status: have/bind/plumb | Realtime? |
|---|---|---|---|---|
| **Members tab** | | `GET /v1/t/[origin]/[org]/members` → `server/admin-data.ts listMembers` → `dojo.memberships` (eq `tenant_id`, order created_at desc); mapped `admin-map.ts toKitMembers` | — | no |
| Member · name | `m.name` | `toKitMember` → `shortId(user_id)` = **first 8 of the uuid**, not a person's name (name lives on `dojo.identities.display_name`, not joined) | plumb | no |
| Member · "you" chip | `m.you` | `toKitMembers(m.value, { self: undefined })` — **self is hardcoded undefined** in `[section]/+page.ts:162`; chip never shows though `data.user` is available | bind | no |
| Member · git line | `git: {m.git} · {m.scopes}` | `accessLine(memberships.authenticated_via)` → "GitHub"/"SSO"/"Device code" (real, but auth-method label, not a git handle) | have | no |
| Member · scopes | `m.scopes` | `toKitMember` → constant `'—'` (per-scope ownership is a separate read; see `scopeOwnersFor`) | plumb | no |
| Member · active | `m.active` | `relativeAge(memberships.last_heartbeat_at)`; `'disabled'` if `disabled_at`, `'never'` if null | have | no |
| Member · role tag | `m.role` | `dojo.memberships.role` (real) | have | no |
| Member · set-role picker | (mockup: static tag) | `onSetRole` → `PATCH /v1/t/[origin]/[org]/members/{userId}/role` → `admin-data setMemberRole` → `memberships.role` (validated `dojo.member_role`; audited). Skipped for own row | have | no |
| Invite CTA | "Invite" | no handler — inert (real `POST …/members` + `addMember` exist, unwired) | plumb | — |
| **Policies tab** | `Object.values(D2.roles)` | `admin-map.ts toKitRolePolicies()` returns the **constant `ROLE_LADDER`** (developer/maintainer/lead/admin); tenant `dojo.policies` rows are fetched (`listPolicies`) but **ignored for the grid** | plumb | no |
| Policy row · kanji/label/note | `r.kanji/label/note` | constant `ROLE_LADDER` (士/掟/客/任) | have (constant) | — |
| "Edit policy" / "New policy" CTA | labels | no handler — inert (real `POST/PATCH/DELETE …/policies` + `upsert/patch/deletePolicy` exist, unwired) | plumb | — |
| **Audit tab** | `D2.chat.thread` | `GET /v1/t/[origin]/[org]/audit` → `admin-data listAudit` → `dojo.audit_events` (eq `tenant_id`, order ts desc, limit≤500); mapped `toKitAuditThread` | have | no |
| Audit row · text | `x.who · x.text` | `toKitChatTurn` → `who='sensei'`, `text = action + " · " + target` (`audit_events.action`/`.target`) | have | no |
| Audit row · actor label | `D2.me.name` | `me` = `data.user?.name ?? email ?? 'You'` (viewer). Note: mapper hardcodes `who='sensei'` for **every** event, so the viewer branch never renders | have/plumb | no |
| Audit row · when | `x.when` | `relativeAge(audit_events.ts)` | have | no |
| Export CTA | "Export" | no handler — inert | plumb | — |

## APIs / loaders
- **Loader:** `dojo/src/routes/(app)/org/[slug]/[section]/+page.ts`. For `section ∈ {members, audit}` runs `Promise.all` of three guarded fetches on `org.url` (tenantKey): `listMembers`, `listPolicies`, `listAudit` (each behind `guardTenantScope` → empty + surfaced error on 403/404/failure). Returns `members`, `rolePolicies`, `auditLog`, `roleTab`, plus `tenantKey`/`accessToken` for the write.
- **Read routes** (`dojo/src/routes/v1/t/[origin]/[org]/`): `members/+server.ts` (GET, ADMIN) · `policies/+server.ts` (GET, ADMIN) · `audit/+server.ts` (GET, ADMIN). Store logic in `lib/server/admin-data.ts`.
- **Write route:** `members/[userId]/role/+server.ts` (PATCH, ADMIN, audited via `recordAudit`). Client `admin-data.ts setMemberRole`; page `setRole()` → `act()` → `invalidateAll()`.
- **Client:** `dojo/src/lib/admin-data.ts` (fetchers + wire types). **Mappers:** `admin-map.ts`, view config `role-surfaces-view.ts`.

## Interactions & states
- **Tab switch** — URL drives `members`↔`audit` (prop `tab`); an in-screen tab click is an ephemeral `$derived` override that yields back to the URL on navigation. Policies is override-only (no URL). Correct.
- **Set role** — native `<select>` per row (a11y label), own row locked to a read-only tag (can't self-demote). On failure → dismissible `actionError` toast (no silent no-op). Wired.
- **Degraded** — any read failure → empty list + `membersError` (loader-surfaced). Honest-empty.
- **Non-admin** — Worker returns 403 → guard degrades to empty (screen still renders). Note: the nav should already gate admin sections by role.

## Gap / to-do (vs mockup)
1. **Member names are uuids.** Join `dojo.identities` (`display_name`/`email`) on `user_id` (or have `listMembers` embed it) so `MemberRow` shows a person, not `a1b2c3d4`.
2. **"you" never resolves** — pass the viewer id as `self` to `toKitMembers` in the loader (`data.user` is present; needs the user_id, not just name).
3. **Scopes column is "—"** — join scope ownership (`scopeOwnersFor` is still fixture; needs a real per-member scope read).
4. **Policies tab shows a constant ladder, not tenant policies.** The real per-scope `dojo.policies` (attribution_default · confidentiality · retention_days) are fetched but dropped; either surface them or add a separate Policies screen. CRUD endpoints already exist.
5. **Audit actor is always "sensei"** — `toKitChatTurn` hardcodes `who='sensei'`; map `audit_events.actor_id` → viewer/other so the `me` label path works.
6. **Invite / New policy / Export / Edit policy** CTAs inert — wire to the existing `POST …/members`, policy CRUD, and an audit CSV export.
7. **Register conflict (Rule B):** `attribution_default` on the membership/policy still uses the 3-value `AttributionMode` incl. `dereferenced` (`admin-data.ts:76`, `server/admin-data.ts:93` `ATTRIBUTION_MODES`, `parseUpsertPolicy`/`parsePatchPolicy` 400 messages). Canon = `named | anonymous` only (`docs/plan/2026-07-27-data-model-fix-impact-register.md` §1B). Not surfaced on this screen yet, but the write validators must drop `dereferenced` when Policies is wired.
8. **Register conflict (Rule C):** `addMember` writes `dojo_url` on the membership (`server/admin-data.ts:326`); canon drops it (derive via `tenant_id → tenants.dojo_url`, §1C).

## Open questions (for Jerry)
- Member display name: join identities in `listMembers` (one query, DRY), or a dedicated `/members?embed=identity`? Which identity provider wins when a user has several?
- Should the **Policies** tab render the tenant's real `dojo.policies` grid (attribution/retention/confidentiality per scope), or stay the read-only additive-ladder explainer with policies living on their own screen?
- Audit **Export** format — CSV or the compliance-PDF the lead console's done-gate implies (`dojo-lead-console.md`)? Must respect universal dereference on export.

### Resolved design (2026-07-30)
- **Q1 identity → dedicated `GET /members?embed=identity`** (separate composition joining `dojo.identities`, WS-1). When a user has several identities, prefer the **primary/verified** one (fallback order github > sso > email).
- **Q2 Policies tab → read-only additive-ladder EXPLAINER.** The editable `dojo.policies` grid lives on the **scopes** screen (no duplication).
- **Q3 Audit Export → CSV** (source-ref-free; universal always-on dereference respected).
- **Register fixes ride here:** policy write validators use `named | anonymous` (Rule B — shipped); `addMember` stops writing `dojo_url`, derives `tenant_id → tenants.dojo_url` (Rule C).
- **Depends on:** WS-1 (`dojo.identities`) + the `?embed=identity` endpoint + WS-0 Rule C (dojo_url derivation).

## Components & state (three-layer, per `sensei:ui-state-pattern`)

**Domain types** (UI-shaped; the Load layer maps wire → these):
```ts
type RoleSurfaces = { members: Member[]; policies: RolePolicy[]; audit: AuditEntry[];
  self: string | null; roleTab: 'members' | 'audit' }
type Member = { userId: string; name: string; you: boolean; git: string;
  scopes: string[]; active: string; role: MemberRole }
type RolePolicy = { scope: string; kanji: string; label: string; note: string;
  attributionMode: 'named' | 'anonymous'; confidentiality: string; retentionDays: number | null }
type AuditEntry = { id: string; actor: { id: string; name: string; self: boolean };
  action: string; target: string; when: string }
```
The types encode the gap fixes: `Member.name` is a real display name (identity join, not `shortId(user_id)`), `Member.you` is resolved (`self` passed in, not hardcoded `undefined`), `Member.scopes` is a real per-member list (not `'—'`), `RolePolicy[]` is the tenant's `dojo.policies` (not the constant `ROLE_LADDER`), `AuditEntry.actor` is mapped from `actor_id` (not hardcoded `who='sensei'`). **`attributionMode` is `named | anonymous` only** — `dereferenced` dropped (canon Rule B, register §1B).

**State** — `role-surfaces-state.svelte.ts` → `roleSurfacesState`
- data: `members`, `policies`, `audit`, `self`, `tab` (ephemeral URL override)
- `$derived`: `activeTab` (URL ∨ override, yields back on nav), `canSetRole(m)` = `m.userId !== self`
- methods: `load(data)`, `setTab(t)`, `setRole(userId, role)`, `invite(input)`, `upsertPolicy(p)`, `export()` (each mutation → Load call → `invalidateAll`)

**Load** — `role-surfaces.ts` → `loadRoleSurfaces(tenantKey, self)`
- mock-first: members (incl. a `you` row + a disabled/never row), a real policy grid, an audit thread with self/other actors; empty/degraded variants
- real (later, body-swap only): the three existing ADMIN GETs on `org.url` (`listMembers`/`listPolicies`/`listAudit`, see APIs above) mapping `dojo.memberships` **joined to `dojo.identities`** for the name (WS-1), `dojo.policies`, `dojo.audit_events` (actor via identity). Mutations: existing set-role PATCH, members POST (invite), policy CRUD, audit export. Two register fixes ride here: policy write validators drop `dereferenced` (attribution = `named | anonymous`, §1B); `addMember` stops writing `dojo_url` (derive `tenant_id → tenants.dojo_url`, §1C).

**Components** (pure, semantic, own styles + `md:` — no `K2*`)
- `RoleSurfaces` shell — Members / Policies / Audit tab switch, reads `roleSurfacesState`
- `MemberList` + `MemberRow` — one `Member` (name · you-chip · git line · scopes · active · role); role picker = accessible native `<select>` (own row locked to a read-only tag); `onSetRole → state.setRole`. Invite CTA wired via `@rokkit/forms`.
- `PolicyGrid` + `PolicyRow` — tenant `RolePolicy[]` (kanji · label · note · attribution · retention · confidentiality); Edit/New via `@rokkit/forms` (attribution field = `named | anonymous`, no `dereferenced` option)
- `AuditThread` + `AuditRow` — actor (self vs other) · action · target · when; Export CTA
- Kanji role marks (士/掟/客/任) = `KanjiToken` (brand); CTA glyphs = Solar icons

**Copy** (paraglide `m.<key>()`): tab/role labels, invite/export/edit copy, degraded-empty copy. Kanji ladder stays `KanjiToken`, not messages.

**Realtime = State**: none (admin reads are load-time). **Test seams:** state methods (`activeTab` override→URL yield, `canSetRole` self-guard, `setRole`); `MemberRow`/`PolicyRow`/`AuditRow` with mock props (incl. `you`/self-actor paths); Load mock → shape.
