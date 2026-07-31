# Scopes & policies (org admin) — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.
- Route: `/org/[slug]/scopes` — `(app)/org/[slug]/[section]/+page.svelte` (section id `scopes`, Admin group, role floor `admin`) → `ScrScopes.svelte`
- Mockup: dojo2-app.jsx `ScrScopes` (L888)
- Access axis: tenant-primary (`entity-access-model.md` §3 — "Governance: rules · **ladder/scopes** · rule-packs · constitution → Tenant `tenant_id`" and "Org console (`/org/[slug]`) → Tenant"). Clean tenant scope.
- Status: STUB — component built; loader feeds `scopeOwnersFor(slug)` **fixture** (only `acme` authored → any real org renders the honest-empty state). No scopes/policies endpoint; `onAssign` unwired. Note the mockup's scope-**ownership** model (owner + queue + SLA per scope) has **no backing table** in the DDL; `dojo.policies` (the "& policies" half) is modeled but not rendered.

## ⚠ Fabricated-data debt — MUST fix on build (2026-07-29 fallback audit)
`(app)/org/[slug]/[section]/+page.ts:246` (`scopeOwnersFor`) + `:234` (`confidentialityFor`) render fabricated scope owners (Keiko Tanaka, Marco Diaz…), queues/SLAs, and a fake confidentiality example. **Impact:** an org admin sees invented scope owners + a fake confidentiality row as if they were real governance state. **Fix on build:** drive every field from the real `/v1` read; on a fetch error render an explicit error state — NEVER the fixture; honest-empty only when genuinely empty. (Ties the daemon-side fabrication fixes + the "no fabricated fallbacks" rule.)

## Elements → data (contract)
| Element | Mockup field | Source (loader/API/table.field) | Status: have/bind/plumb | Realtime? |
|---|---|---|---|---|
| Header "Scopes & policies" (eyebrow "{org} · admin") | `org.name` | `data.orgName` (from `orgBySlug` on real memberships) | have | no |
| "New scope" button | — | no handler (noop) | have (noop) | no |
| Banner 規 (warns when any unowned) | derived from `owners` | **FIXTURE** `fixtures.ts:scopeOwners` | have | no |
| Group sections Company / Teams / Stacks | `rows.filter(group)` | **FIXTURE**. Canonical ladder = `sensei.scopes` (`key`/`name`/`level`); a scope instance = `sensei.namespaces` at that scope. | bind→plumb | no |
| Row: scope label | `r.scope` | `sensei.scopes.name` / a `sensei.namespaces` instance | plumb | no |
| Row: "{queue} in queue · SLA {sla}" | `r.queue` / `r.sla` | **No source** — queue depth / SLA not modeled in DDL | plumb (new schema) | no |
| Row: owner Avatar + name + RoleTag | `r.owner` / `r.role` | **No source** — no scope-owner table; nearest is `dojo.memberships.role`, but no scope↔owner binding exists | plumb (new schema) | no |
| Row: "unowned · fallback" chip | `r.owner == null` | derived | have | no |
| Assign / Reassign button | `onAssign(r)` | Write → **no endpoint, no table** | plumb (new schema) | no |

## APIs / loaders
- load(): `(app)/org/[slug]/[section]/+page.ts` returns `scopeOwners: scopeOwnersFor(slug)` from fixtures. The loader's guarded-`/v1`-fetch pattern is deliberately **not** applied to scopes — its own comment (L86-87) says scopes "still render off kit fixtures — their routes aren't built (Tier 3: scopes/projects/stance need further wiring)".
- mutations: none. `onAssign` is forwarded from the page but bound to nothing.
- realtime: none.

## Interactions & states
- Assign/Reassign → `onAssign?.(r)` reaches `+page.svelte` — but the page does not wire it (no console mutation).
- Banner tone flips to `warning` when `owners.filter(!owner).length > 0`.
- Empty (real org): `scopeOwnersFor` returns `[]` → every group `if (items.length)` drops → screen shows only header + neutral banner (honest empty).

## Gap / to-do (vs mockup)
- **Scope ownership has no schema.** Owner / queue / SLA / assign are pure fixtures — a new model is required (e.g. `scope_owner{ tenant_id, scope_key|namespace_id, owner_membership_id, sla }` + queue count derived from triage routing) before any of it can be real.
- **The "& policies" half is not rendered.** `dojo.policies` (per-scope `attribution_default`, `confidentiality`, `retention_days`, unique on `(tenant_id, scope_key)`) is the actual tenant policy grid; the mockup shows only ownership.
- **DEREFERENCE CONFLICT (must follow canon).** `dojo.policies.attribution_default` is typed `dojo.attribution_mode` whose enum **still carries `dereferenced`**, and the column comment reads "named | anonymous | dereferenced". Per `entity-access-model.md` §5 + `data-model-fix-impact-register.md` §1B: `attribution_mode = named | anonymous` only; **dereference is a universal, always-on transform, not a policy value**. Any policies UI added here must offer `named|anonymous` and present dereference as a non-editable invariant — never a selectable attribution option.

## Open questions (for Jerry)
1. Scope ownership (owner, queue depth, SLA, assign) is absent from the schema. New table, or derive from triage-queue routing (`triage` + `memberships`)? This screen cannot be real without a source.
2. Should this screen also surface the `dojo.policies` grid (attribution / confidentiality / retention per scope)? The title says "& policies" but the mockup renders only ownership.

### Resolved design (2026-07-30)
- **Q1 ownership → NEW `dojo.scope_owner` table** `{ tenant_id, scope_key|namespace_id, owner_membership_id, sla }`; **queue depth derived from triage routing**; assign/reassign writes this table. WS-3 schema addition.
- **Q2 → render BOTH ownership + the `dojo.policies` grid** (attribution / confidentiality / retention per scope) — the full "Scopes & policies" screen.
- **New-Q state → SPLIT into two states/loads:** the **policies** grid ships NOW (backed by the existing `dojo.policies`); **ownership** is a separate state/load that ships later once `scope_owner` lands — honest-empty until then (NO fabricated owners).
- **Build constraint (fabricated-data debt):** owner/queue/SLA are pure fixtures today — render honest-empty (not the `acme` fixture) until real; error state on fetch failure.
- **Depends on:** new `dojo.scope_owner` DDL (WS-3) + a policies CRUD endpoint over `dojo.policies` + triage-routing queue derivation + role checks (admin).
3. Confirm the policies UI (if added) drops the `dereferenced` attribution option (canon: `named|anonymous`) and shows dereference as always-on — and confirm the enum/comment fix in `attribution_mode.ddl` + `policies.ddl` is in scope for this work.

## Components & state (three-layer, per `sensei:ui-state-pattern`)

Makes the three layers explicit; references the **Elements → data** + **APIs / loaders** sections
above for the tables/endpoints (not restated).

- **DB** — `sensei.scopes` (`key`/`name`/`level` ladder) · `sensei.namespaces` (a scope instance) ·
  `dojo.policies` (per-scope `attribution_default` typed `dojo.attribution_mode`, `confidentiality`,
  `retention_days`; unique `(tenant_id, scope_key)`) — the "& policies" half. **No scope-owner /
  queue / SLA table exists** — owner/queue/SLA/assign are pure fixtures (new schema, see Gap/to-do).
  **Tenant**-primary.
- **API** — loader `loadScopes` (mock → real); real ownership = **no endpoint, no table**; real
  policies = a `dojo.policies` read (`/v1` endpoint doesn't exist yet); `assign`/`setPolicy`
  mutations don't exist. No realtime.
- **UI** — `ScrScopes` shell composing scope-owner rows + a (new) `PolicyGrid`, reading `scopesState`.

**Domain types** (UI-shaped; the Load layer maps wire → these):
```ts
// KitScopeOwner { scope; group; owner: string|null; role: string|null; queue; sla }  ← NO backing table
type Policy = { scopeKey; attributionDefault: 'named' | 'anonymous';                   // ← dojo.policies
                confidentiality; retentionDays: number }
```
**Attribution canon (load-bearing here):** `Policy.attributionDefault` is `named | anonymous` **only**
— `dereferenced` is NOT a mode; universal source-dereference is an always-on, non-editable publish
transform (`entity-access-model.md` §5). The (new) policy control offers named/anonymous and presents
dereference as a fixed invariant — never a selectable option.

**State** — `scopes-state.svelte.ts` → `scopesState` (**new** — today the screen renders the
`scopeOwnersFor(slug)` fixture directly, no state module)
- data: `owners: KitScopeOwner[]`, `policies: Policy[]`
- `$derived`: `groups` (owners bucketed Company/Teams/Stacks) · `anyUnowned` (drives the 規 banner tone)
- methods: `assign(scope, membershipId)` (needs the new ownership table) · `setPolicy(scopeKey, mode)`
  (`named|anonymous` only)

**Load** — `scopes.ts` → `loadScopes()` (wired in `(app)/org/[slug]/[section]/+page.ts`, section `scopes`)
- mock-first: hand-crafted `KitScopeOwner[]` + `Policy[]` exercising owned / **unowned-fallback** /
  each group / empty (real org → `[]`, honest empty) → build UI + tests to fidelity NOW
- real (body-swap only): ownership blocked on the new `scope_owner` schema; policies map `dojo.policies`
  rows → `Policy[]` once a read endpoint exists

**Components** (pure, semantic, own styles + `md:`)
- `ScrScopes` — shell: header + "New scope" + 規 banner (warns when `anyUnowned`) + Company/Teams/Stacks
  group sections. Reads `scopesState`.
- scope-owner row — one `KitScopeOwner`: scope label · "{queue} in queue · SLA {sla}" · owner
  `Avatar` + name + `RoleTag`, or "unowned · fallback" chip · Assign/Reassign → `assign`.
  **Mockup-match + `md:` live here.**
- `PolicyGrid` + `PolicyRow` (**new**) — the "& policies" half: per-scope `attribution_default`
  (`named|anonymous` control via `@rokkit/forms`) · confidentiality · retention; dereference rendered
  as an always-on, non-editable invariant.

**Copy** (paraglide `m.<key>()` — no inline literals): `m.scopes_title()`, `m.scopes_eyebrow({org})`,
the 規 banner, "in queue"/"SLA"/"unowned · fallback", Assign/Reassign, policy labels, empty copy.
Kanji (規) stays a `KanjiToken` brand mark.

**Realtime = State**: none — assign/policy is a mutation + refetch. **Test seams:** `scopesState`
methods (no DOM); the scope-owner row + `PolicyRow` with a mock prop (owned/unowned; named/anonymous);
`loadScopes` mock → `{owners, policies}` shape.

**New open question (three-layer):** should one `scopesState` carry **both** the (new-schema)
ownership rows and the `dojo.policies` grid behind a single `loadScopes`, or split policies into its
own state/screen — since ownership has no backing table today while policies does, one Load blocks the
other from ever going real?
