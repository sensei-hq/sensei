# My dōjōs — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.

- Route: `/you/dojos` = `(app)/you/[section]/+page.svelte` (`section === 'dojos'` branch) + `[section]/+page.ts`
- Mockup: dojo2-app.jsx `ScrMyDojos` (L1463) — board "1c · My dōjōs — register / view memberships"
- Access axis: **user/membership-primary** — canonical `entity-access-model.md` §1/§3: a user has many memberships, one per tenant; this screen lists exactly the user's memberships across ALL their tenants. Keyed off the user (`memberships` = `listUserOrgs(user.id)`), never a single tenant.
- Status: **PARTIAL** — the screen (`ScrMyDojos.svelte` + `personal-view.groupDojos`) is built and binds REAL memberships from the shared console context. But `tenantToOrg` hardcodes `kind`, `kanji`, `members`, `pending`, so grouping and every count are wrong/stubbed.

## Elements → data (contract)
Live: `(app)/+layout.server.ts` → `loadConsoleContext` → `listUserOrgs(user.id)` (`dojo.memberships` ⋈ `dojo.tenants`) → `[section]/+page.ts` `toKitDojos(memberships)` → `ScrMyDojos.svelte` → `groupDojos` → `MyDojoRow`.

| Element | Mockup field | Source (loader/API/table.field) | Status | Realtime? |
|---|---|---|---|---|
| SectionHead eyebrow/title | `You · membership` / `My dōjōs` | static | have | — |
| header count | `D2.dojos.length` | `dojos.length` (real membership count) | have | — |
| "Create or join" btn | `add-circle` | static, **not wired** (`onCreateOrJoin` undefined here) | plumb | — |
| banner "you belong to N" | `D2.dojos.length` | `dojos.length` | have | — |
| group sections | `employer / client / community` (empty dropped) | `groupDojos` on `KitDojo.kind` — **but `kind` is hardcoded `Community`** in `tenantToOrg` → every dōjō falls into "Communities"; employer/client groups never render | plumb | — |
| group icon/label | per-kind | `DOJO_GROUPS` (`buildings-2`/`case-round`/`users-group-two-rounded`) | have | — |
| row: dōjō kanji | `d.kanji` | `KitDojo.kanji` — **hardcoded `群`** in `tenantToOrg` (real glyph would derive from kind/tenant) | plumb | — |
| row: name | `d.name` | `dojo.tenants.name ?? tenants.org` | **have** | — |
| row: role tag | `d.role` | `dojo.memberships.role` → `ROLE_LABEL` → `roleKey` | **have** | — |
| row: route | `d.route` | `dojo.tenants.key` (the tenant key/discovery path) | **have** | — |
| row: members count | `d.members` | **hardcoded 0** in `tenantToOrg` (`toKitDojo` passes it through) | plumb | — |
| row: projects count | `d.projects` | **hardcoded 0** in `toKitDojo` | plumb | — |
| row: needs badge | `d.needs` | `DojoOrg.pending` → **hardcoded 0** in `tenantToOrg` | plumb | — |
| row open → org | `onOpen(d)` | `[section]/+page.svelte` — **`ScrMyDojos onOpen` NOT passed** on this route; row is inert (mockup switches org context) | plumb | — |
| empty state | `空 No dōjōs yet` + Create/join | `EmptyState` when no memberships (DJ1) | have | — |

## APIs / loaders
- **load()**: server `loadConsoleContext` runs `listUserOrgs(userId)` — `dojo.memberships` (`role`, `disabled_at is null`) embedding `dojo.tenants` (`id, key, org, name, self_hosted`). Page `+page.ts` maps `toKitDojos(memberships)`. This is the ONE real personal-data binding among the five screens.
- **mutations**: none. "Create or join" is unwired; no invite/join endpoint bound here.
- **realtime**: none (membership changes need a reload).
- **Data quality gap**: `tenantToOrg` (`server/dojo-orgs.ts`) fills `kanji:'群'`, `kind:'Community'`, `members:0`, `pending:0` — none come from the tenant/membership rows. `TENANT_COLS` doesn't select a kind/origin or member/pending counts. So the real fields are name + role + route only.

## Interactions & states
- **Grouping**: employer → clients → communities, empty groups dropped (`groupDojos`). BROKEN today: all rows read `Community`, so only the "Communities" group ever shows.
- **Row open**: mockup `onOpen(d)` switches into the org context (`onPick(d.slug)`). On this route `ScrMyDojos` is rendered WITHOUT `onOpen`, so rows don't navigate. The shell's OrgSwitcher is the working path into an org; confirm whether the row should also route to `/org/[slug]`.
- **Empty (DJ1)**: honest-empty with a Create/join CTA (also unwired).
- **Responsive**: `ScrMyDojos` hard-codes `p-8 gap-6`; no `mobile` prop threaded from the route (shell handles phone chrome). `MyDojoRow` has its own compact behavior.

## Gap / to-do (vs mockup), ranked
1. **`kind` is fake** — derive the real membership/tenant kind (employer/client/community/personal) so grouping works. Source: `dojo.tenants` origin/kind or `dojo.memberships.kind` (canon §1 says membership carries `kind` personal/employer/client/community). Add it to `TENANT_COLS`/the select and map in `tenantToOrg`.
2. **Counts are fake** — `members`, `projects`, `needs(pending)` all hardcoded 0. Either compute (member count per tenant; pending asks per membership) or drop the count chips rather than show a fabricated 0.
3. **Row navigation** — pass `onOpen` (→ `enterOrg`/`/org/[slug]`) so a row steps into the dōjō, matching the mockup; or intentionally keep the OrgSwitcher as the only entry and simplify the row.
4. **Create-or-join** — wire the CTA (invite-accept / create-tenant flow) or mark it out-of-scope.
5. **`kanji`** — derive per kind instead of the constant `群`.

## Open questions (for Jerry)
1. Does `dojo.memberships`/`dojo.tenants` actually carry a `kind` (personal/employer/client/community)? Canon §1 says membership does; the DDL read here (`TENANT_COLS`) doesn't select it. Confirm the column so grouping can be real.
2. Member/project/pending counts per dōjō — compute them (extra queries per tenant) or omit? For a personal "which dōjōs am I in" list, role + name may be enough; the counts are org-console concerns.
3. Should a row navigate into the org context (duplicating the OrgSwitcher), or is the switcher the single entry and the row purely informational?
4. "Create or join a dōjō" — in scope pre-release? If yes, what's the flow (GitHub-org auto-join vs magic-link invite vs self-serve create)?
5. Personal-tenant treatment: canon `…rls-membership-function.md` §7 flags personal dōjō as an ad-hoc `origin='org'`. Should the user's own personal dōjō even appear in this list, or is it implicit (the `/you` context itself)?

### Resolved design (2026-07-30)
- **Q1 (factual):** `dojo.memberships.kind` (enum `membership_kind`) and `dojo.tenants.origin` (enum `tenant_origin`) **both exist** — the read just omits `kind`. Fix = add `memberships.kind` to `TENANT_COLS`, derive the glyph from `kind`. Kills the "everything → Communities" bug. No new schema.
- **Q2 counts → COMPUTE real counts:** members (per tenant), projects (from `dojo.projects` once it lands), pending (asks per membership). Extra per-tenant queries; no fabricated 0s.
- **Q3 row click → NAVIGATES into org:** clicking a dōjō row enters that dōjō's org console (`/org/[slug]`) — a nav path alongside the OrgSwitcher.
- **Q4 create/join → IN SCOPE. ALL THREE flows:** (1) self-serve create (user creates an org/client dōjō, becomes admin), (2) magic-link invite (admin invites via kavach magic link → membership on accept), (3) GitHub-org auto-join (SSO-style auto-provision from GitHub org membership).
- **Q5 personal dōjō → SHOW it as a row** (a `Personal` group).
- **Depends on:** WS-0 Rule A (user-wide membership read) + `dojo.projects` (for the projects count) + WS-1 identity (member names, if shown).

## Components & state (three-layer, per `sensei:ui-state-pattern`)

**DB** `dojo.memberships` ⋈ `dojo.tenants` via `listUserOrgs(user.id)` — needs `kind` added to `TENANT_COLS`;
member/project/pending counts would need extra per-tenant queries. Read-only (create/join unwired) · **API**
Load = server `loadConsoleContext`→`listUserOrgs` in `(app)/+layout.server.ts`, mapped by `[section]/+page.ts`
(Supabase-direct, no `/v1`) · **UI** `myDojosState` + `DojoList`/`DojoGroup`/`DojoRow` over the `Dojo` domain type.

**Domain types** (UI-shaped; Load maps `memberships ⋈ tenants` → these):
```ts
type Dojo = { id; slug; name; role: string; route: string;
  kind: 'personal'|'employer'|'client'|'community'; members?: number; projects?: number; needs?: number }
```
`members`/`projects`/`needs` are **optional** — a row renders a count chip only when the loader actually
computed it (no fabricated 0, unlike today's `tenantToOrg`).

**State** — `my-dojos-state.svelte.ts` → `myDojosState`
- data: `dojos: Dojo[]`
- `$derived`: `groups` (group by `kind`, empty groups dropped — replaces `groupDojos`/`personal-view`), `total`
- methods: `load(dojos)`, `open(slug)` (→ `enterOrg`/`/org/[slug]`)

**Load** — `my-dojos.ts` → `loadMyDojos()`
- **already real**: wraps `listUserOrgs(userId)` → maps to `Dojo[]` (the one real personal-data binding of the
  five personal screens). The mock-first seam here is only for **empty (DJ1) / error / multi-kind** test
  fixtures, not to build the screen to fidelity.
- gap it must close: select the real `kind` (from `memberships.kind` or `tenants.origin`) instead of the
  hardcoded `Community`; leave counts absent unless computed.

**Components** (pure, semantic, own styles + `md:` — fidelity verified per component)
- `DojoList` — `SectionHead` + "you belong to N" banner + Create/join CTA + `DojoGroup[]` from
  `myDojosState.groups` + `EmptyState` (DJ1). (replaces `ScrMyDojos`)
- `DojoGroup` — one kind group: Solar group icon + label (`DOJO_GROUPS`
  buildings-2/case-round/users-group-two-rounded) + `DojoRow[]`.
- `DojoRow` — one `Dojo`: Solar kind icon (drops the constant kanji 群) · name · role tag · route · optional
  member/project/needs chips; `onopen→state.open`. **Mockup-match + `md:` live here.** (replaces `MyDojoRow`)

**Copy** (paraglide `m.<key>()`): `m.dojos_title()`/`m.dojos_eyebrow()`, `m.dojos_belong({n})`, group labels,
role labels (`ROLE_LABEL`→messages), Create/join CTA, `m.dojos_empty()`. Kanji (空) stays a `KanjiToken` brand
mark; kind glyphs are Solar icons, not kanji.

**Realtime = State**: none (a membership change = reload). **Test seams:** `myDojosState.groups` grouping +
empty-drop (no DOM); `DojoRow` with a mock `Dojo` (counts present/absent); Load mock for DJ1/multi-kind.

**New open question (from this exercise):** because Load is *already real* (not a stub mocked into fidelity),
the mock-first pattern degenerates to test fixtures only — yet the screen still can't reach fidelity until the
`kind` column ships. Is adding `memberships.kind`/`tenants.origin` to `TENANT_COLS` in scope this pass (it
unblocks the whole grouping), or does my-dojos stay single-group ("Communities") until the WS-0/WS-1 pass?
