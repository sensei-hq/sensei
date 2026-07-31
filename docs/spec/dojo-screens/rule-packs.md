# Rule packs (personal) — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.
- Route: `/you/packs` — `(app)/you/[section]/+page.svelte` (section id `packs`) → `ScrRulePacks.svelte`
- Mockup: dojo2-app.jsx `ScrRulePacks` (L211), row `PackRow` (L161)
- Access axis: tenant-primary per `entity-access-model.md` §3 (rule-packs = governance). **Nuance:** the pack LIBRARY (`sensei.rule_packs`) is a *shared-plane* artifact — global (`owner_namespace_id` NULL) or org-authored (`owner_namespace_id → namespaces`); ADOPTION (`sensei.rule_pack_adoptions`) binds a concrete `namespace_id`. A personal `/you/packs` adoption binds the *user's* namespace, an org adoption (on ScrOrgLadder) binds a stack/org namespace — same table, different scope instance.
- Status: PARTIAL — component + `rulepacks-state` adopt/drop built; loader feeds **fixture** `rulePacks`; adopt/drop is local `$state` only. Full DDL exists (`rule_packs` / `rule_pack_rules` / `rule_pack_adoptions`) and the daemon-facing `GET /v1/.../rules/resolved?ns=` resolves adopted packs — but there is **no list-packs or adopt/drop endpoint** for the dōjō UI.

## ⚠ Fabricated-data debt — MUST fix on build (2026-07-29 fallback audit)
The `/you` rule-packs list is fixture-backed via the shared constitution loader (`(app)/you/[section]/+page.ts:5` import, `:25` returns `rulePacks`). **Impact:** a real user sees fabricated adopted/available packs, not their real adoptions. **Fix on build:** drive the list from the real `/v1` read; on a fetch error render an explicit error state — NEVER the fixture; honest-empty only when genuinely empty. (Ties the daemon-side fabrication fixes + the "no fabricated fallbacks" rule.)

## Elements → data (contract)
| Element | Mockup field | Source (loader/API/table.field) | Status: have/bind/plumb | Realtime? |
|---|---|---|---|---|
| Header "Rule packs" (eyebrow "Adopt · not a library") | static | — | have | no |
| "Browse all" button | — | no handler (noop) | have (noop) | no |
| Banner 束 | static copy | — | have | no |
| "Adopted" ListSection | `packState.adopted` | **FIXTURE** `fixtures.ts:rulePacks` filtered `adopted` (`personal-view.splitPacks`). Real → `sensei.rule_pack_adoptions ⋈ rule_packs` for the viewer's namespace. | bind→plumb | no |
| "Available" ListSection | `packState.available` | **FIXTURE**. Real → `sensei.rule_packs` (`status='active'`, visible to viewer) minus adopted. | bind→plumb | no |
| PackRow name / by / note | `pack.name` / `pack.by` / `pack.note` | `rule_packs.name` / `.source` / `.summary` | bind | no |
| "N rules" disclosure chip | `pack.rules.length` | count of `rule_pack_rules` (ordered by `ordinal`) | bind | no |
| Expanded rule rows | `pack.rules[]` (string[]) | `rule_pack_rules.statement` (RuleRow renders `{kanji, text}`) | bind | no |
| adopted chip / Adopt · Drop button | `pack.adopted` + `adopt()` | Write → `sensei.rule_pack_adoptions` insert/delete (pins `pinned_version`, optional `enforcement` override) at the user's namespace. **No dōjō endpoint.** | plumb | no |

## APIs / loaders
- load(): `(app)/you/[section]/+page.ts` returns `rulePacks` from fixtures; no fetch.
- mutations: none. Adopt/drop = `rulepacks-state.toggle` (local, non-persistent). To plumb: `GET /v1/t/{origin}/{org}/packs` (library + the viewer-namespace's adoptions) and `POST`/`DELETE /v1/.../packs/{id}/adoption` — **none exist**.
- realtime: none.

## Interactions & states
- "N rules" chip → local expand/collapse (`SvelteSet`), a11y `aria-expanded`.
- Adopt / Drop → `packState.toggle` moves the pack between the two ListSections (derived off the live `adopted` flag); not persisted.
- Empty state: `splitPacks([])` yields two empty ListSections (honest empty) — fixture path never empties.

## Gap / to-do (vs mockup)
- No list endpoint and no adopt/drop mutation.
- **Adoption scope-picker missing:** the banner promises "adds its rules at the scope you choose", but `adopt()` is a bare boolean toggle with no scope/namespace choice — DDL adoption is `namespace_id`-keyed, so a picker is required.
- **Per-rule guard tone lost:** mockup `PackRow` renders a `guard` chip for `r.tone === 'guard'`; the Svelte `RuleRow` only receives `{kanji, text}`. `rule_pack_rules.enforcement` (esp. `mandatory`) should drive a guard/★ marker.
- `KitRulePack.rules` is `string[]`; the DDL rule is structured (`statement`/`body`/`rationale`/`enforcement`/`verification`/`checker_ref`/…) — the (unbuilt) wire→kit mapper must reduce to `statement` and decide what else surfaces.
- "Browse all" is inert.

## Open questions (for Jerry)
1. For a personal `/you/packs` adoption, which `namespace_id` is "you"? Is there a per-user (`user`-scope) namespace row to bind adoptions to, or does personal adoption need a different mechanism than `rule_pack_adoptions`?
2. Visibility filter for the "Available" list — global packs (`owner_namespace_id` NULL) plus packs owned by the user's dōjō namespaces only? Confirm.
3. This personal adopt vs the org "Adopt pack" on ScrOrgLadder both write `rule_pack_adoptions` at different `namespace_id`s — confirm the two affordances stay distinct (individual vs tenant-wide) and which roles may adopt at an org scope.

### Resolved design (2026-07-30)
- **Q1 + new-Q → personal adoption = the user's single personal namespace (`scope_key='personal'`); NO scope-picker on `/you/packs`.** The bare toggle becomes real `adopt(userNS)` / `drop` — `POST`/`DELETE` `sensei.rule_pack_adoptions` keyed on the user's personal namespace. The scope-picker is an **org-only** affordance (ScrOrgLadder).
- **Q2 → Available visibility = global packs (`owner_namespace_id` NULL) + packs owned by the user's dōjō namespaces.**
- **Q3 → org-scope adoption requires `maintainer` or `admin`;** contributors cannot adopt tenant-wide. Personal adoption (any user, own namespace) stays a **distinct** affordance.
- **Depends on:** dōjō list-packs + adopt/drop endpoints (`GET /v1/…/packs`, `POST`/`DELETE …/packs/adoptions`) + the user's personal namespace row + WS-0 Rule A/role checks.

## Components & state (three-layer, per `sensei:ui-state-pattern`)

Makes the three layers explicit; references the **Elements → data** + **APIs / loaders** sections
above for the tables/endpoints (not restated).

- **DB** — `sensei.rule_packs` (library — global `owner_namespace_id` NULL | org-authored
  →`namespaces`) · `sensei.rule_pack_rules` (`statement`, `ordinal`, `enforcement`) ·
  `sensei.rule_pack_adoptions` (adoption binds a concrete `namespace_id`, `pinned_version`,
  optional `enforcement` override). **Tenant**-primary; a personal adoption binds the *user's* namespace.
- **API** — loader `loadRulePacks` (mock → real); real list = `GET /v1/…/packs` (library +
  the viewer-namespace's adoptions), adopt/drop = `POST`/`DELETE /v1/…/packs/{id}/adoption`
  — **none exist**. No realtime.
- **UI** — `ScrRulePacks` shell (Adopted + Available `ListSection`s) composing `PackRow` +
  `RuleRow`, reading `rulePacksState`.

**Domain types** (UI-shaped; the Load layer maps wire → these):
```ts
// KitRulePack { id; kanji; name; by; rules: string[]; adopted; note }
//   name ← rule_packs.name · by ← .source · note ← .summary · rules ← rule_pack_rules.statement[]
//   (ordered by ordinal) · adopted ← an adoption row exists for the viewer's namespace
```
`KitRulePack.rules` is `string[]`; the wire rule is structured (`statement`/`body`/`rationale`/
`enforcement`/`verification`/`checker_ref`) — the (unbuilt) wire→kit mapper reduces to `statement`
and must surface a guard/★ marker from `enforcement` (esp. `mandatory`), which the kit currently drops.

**State** — `rulepacks-state.svelte.ts` → `createRulePacks(seed) → RulePacksState` (**exists**; convention
target singleton `rulePacksState`)
- data: `packs: KitRulePack[]` (copied from seed so toggling never mutates fixtures)
- `$derived`: `adopted` / `available` = `splitPacks(packs)` (off the live `adopted` flag)
- methods: `isAdopted(id)`, `toggle(id)` (local only) — **real adopt is `namespace_id`-keyed, so
  `toggle` must grow to carry a chosen scope** (the missing scope-picker; see Gap/to-do)

**Load** — `rule-packs.ts` → `loadRulePacks()` (wired in `(app)/you/[section]/+page.ts`, section `packs`)
- mock-first: hand-crafted `KitRulePack[]` exercising adopted / available / a mandatory-guard rule /
  a long rule list / empty (`splitPacks([])` → two empty sections) → build UI + tests to fidelity NOW
- real (body-swap only): `GET /v1/…/packs` → map `rule_packs ⋈ rule_pack_rules` (+ the viewer's
  adoptions) → `KitRulePack[]`; `toggle` → `POST`/`DELETE …/adoption` at the chosen namespace

**Components** (pure, semantic, own styles + `md:`)
- `ScrRulePacks` — shell: header + "Browse all" + 束 banner + Adopted/Available `ListSection`s.
  Reads `rulePacksState`; adopt/drop → `toggle`.
- `PackRow` — one `KitRulePack`: name · by · note · "N rules" disclosure chip (`SvelteSet` expand,
  `aria-expanded`) · adopted chip / Adopt·Drop. **Mockup-match + `md:` live here.** Needs the guard
  chip restored on expanded `RuleRow`s (mockup `r.tone==='guard'`).
- `RuleRow` — one statement; must receive `{kanji, text, hard}` so `enforcement='mandatory'` renders
  the guard/★ marker (today only `{kanji, text}`).
- **Scope-picker** (new) — the adoption's `namespace_id` choice the banner promises ("adds its rules
  at the scope you choose"); via `@rokkit/forms`.

**Copy** (paraglide `m.<key>()` — no inline literals): `m.packs_title()`, `m.packs_eyebrow()`
"Adopt · not a library", the 束 banner, `m.packs_adopted()`/`m.packs_available()`, adopt/drop labels,
empty copy. Kanji (束/守) stay `KanjiToken` brand marks.

**Realtime = State**: none — adopt/drop is a mutation + refetch, not a live `patch`. **Test seams:**
`RulePacksState` methods (no DOM); `PackRow`/`RuleRow` with a mock prop (fidelity, incl. adopted +
guard); `loadRulePacks` mock → `KitRulePack[]` shape.

**New open question (three-layer):** should `rulePacksState.toggle(id)` become
`adopt(id, namespaceId)` — the State carrying a chosen adoption scope — or is a personal `/you/packs`
adoption always the user's single namespace, so the scope-picker is an **org-only** affordance (ScrOrgLadder)
and the personal state stays a bare boolean toggle?
