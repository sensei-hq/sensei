# Constitution (personal) — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.
- Route: `/you/rules` — `(app)/you/[section]/+page.svelte` (section id `rules`; mobile tab `rules`) → `ScrConstitution.svelte`
- Mockup: dojo2-app.jsx `ScrConstitution` (L133)
- Access axis: tenant-primary for the RULES half (`entity-access-model.md` §3 — "Governance: rules · ladder/scopes · rule-packs · constitution → Tenant `tenant_id`"). **Exception:** the STANCE half is **user-scoped, daemon-local** (`sensei.stances.user_key`, DDL: "stance follows the user, not a tenant … NOT tenant-shared"). So this personal-zone screen mixes a user-scoped stance with the user's inherited namespace/tenant governance — it is the *personal projection*, not a tenant surface.
- Status: PARTIAL — component + `constitution-map.rulesToLadder` mapper built & unit-tested, but the loader feeds **fixtures** (`stance`, `ladder`); the mapper is unwired, there is no `/v1` read endpoint, and the stance dials forward `onChange` to nowhere (the page passes no `onStance`, no write path).

## Elements → data (contract)
| Element | Mockup field | Source (loader/API/table.field) | Status: have/bind/plumb | Realtime? |
|---|---|---|---|---|
| Header "Your constitution" (eyebrow "You · standing rules") | static | — | have | no |
| "Rule packs →" button | `onGoPacks` | `+page.svelte` `goPacks()` → `goto(youHref('packs'))` = `/you/packs` | have | no |
| Banner 静 ("stays on your machine …") | static copy | — | have | no |
| Stance dials, 3-up grid | `D2.stance` → `stance` prop | **FIXTURE** `components/kit/fixtures.ts:stance`. Real source `sensei.stances` (`autonomy` `stance_autonomy` · `sharing` `stance_sharing` · `review` `stance_review`), keyed `user_key` + `namespace_id`. **No dōjō endpoint** (daemon-local). | bind→plumb | no |
| Dial change | `StanceDial onChange(id,value)` | Write path is daemon-side: `POST /api/stance` + MCP `set_stance`/`upsert_stance` (see memory *Stance WRITE path*). Dōjō has **no** stance route; `+page.svelte` wires no `onStance`. | plumb | no |
| "Your effective constitution" rungs (personal + stack) | `personalRungs(D2.ladder)` | **FIXTURE** `fixtures.ts:ladder` filtered to `id∈{personal,stack}` (`personal-view.personalRungs`). Real → `constitution-map.rulesToLadder(ConstitutionRule[])` over `dojo.shared_rules ⋈ sensei.namespaces` resolved to the user's namespaces, ordered by `sensei.scopes.level`. | bind→plumb | no |
| LadderRung rule ★ hard-lock | `rung.rules[].hard` | `constitution-map.isHardRule` ⇐ `sensei.enforcement = 'mandatory'` | have (mapper) / plumb | no |

## APIs / loaders
- load(): `(app)/you/[section]/+page.ts` — returns `stance`, `ladder`, `rulePacks` verbatim from `$lib/components/kit/fixtures`; **no fetch**. `rulesToLadder` exists but nothing calls it. Comment in the loader (L20-22) explicitly flags governance as "still fixture-backed pending its route".
- mutations: none in the dōjō. Stance edits belong to the daemon (`POST /api/stance`, MCP `set_stance`) — a cross-plane action with no dōjō route.
- realtime: none.

## Interactions & states
- Stance dial click → local `$state` only (`StanceDial` seeds from `dial.value`, forwards `onChange`); no persistence.
- "Rule packs →" → client nav to `/you/packs`.
- Empty state: `rulesToLadder([]) → []` (empty-in/empty-out, honest empty) once wired; the fixture path never empties.

## Gap / to-do (vs mockup)
- Unwire fixtures: feed `ladder` from `rulesToLadder(...)` off a real read; feed `stance` from a real stance read.
- No `/v1` (or daemon) endpoint returns a *resolved personal constitution* (the user's most-specific rungs across their namespaces) — needs a new read.
- No write path for the stance dials (the dials are interactive but inert).
- **Rung-id mismatch (real bug):** `personal-view.personalRungs` filters `r.id === 'personal' || 'stack'`, but `rulesToLadder` emits `id = scope_key` (`user`, `technology`, …). When the mapper is wired, `personalRungs` returns `[]`. The scope-key vocabulary must be reconciled.

## Open questions (for Jerry)
1. Stance is user-scoped + daemon-local (`sensei.stances`, no `tenant_id`). Should the dōjō web app **edit** stance at all, or read-only (edited only in the desktop app / via MCP)? If read-only, where does the dōjō read it from given there is no cross-DB link to the daemon?
2. `/you/rules` shows the user's effective constitution — but the user can span several dōjōs. Which tenant's governance resolves here, and via which endpoint (daemon-resolved vs a new tenant `/v1/rules/resolved` for the user's namespaces)?
3. Confirm the canonical rung-id vocabulary so `personalRungs` matches `rulesToLadder` output (`personal|stack` vs `user|technology`). Fix one side.

## Components & state (three-layer, per `sensei:ui-state-pattern`)

Makes the three layers explicit; references the **Elements → data** + **APIs / loaders**
sections above for the tables/endpoints (not restated).

- **DB** — `dojo.shared_rules ⋈ sensei.namespaces` (the ladder half — **tenant**-scoped,
  ordered by `sensei.scopes.level`, `enforcement='mandatory'` → hard ★) · `sensei.stances`
  (`user_key` + `namespace_id`; `stance_autonomy`/`stance_sharing`/`stance_review`) — the
  stance half is **user-scoped, daemon-local, no `tenant_id`, no cross-DB link to a dōjō read**.
- **API** — loader `loadConstitution` (mock → real); real = a resolved-personal-constitution
  `/v1` read for the ladder + a daemon stance read for the dials (neither exists yet); the
  W1 `constitution-map.rulesToLadder` mapper is the ladder transform. No realtime.
- **UI** — `ScrConstitution` shell composing `StanceDial` + `LadderRung`/`RuleRow`, reading
  `constitutionState`.

**Domain types** (UI-shaped; the Load layer maps wire → these):
```ts
type PersonalConstitution = { stance: KitStanceDial[]; ladder: KitLadderRung[] }
// KitStanceDial { id; kanji; label; caption; levels: string[]; value }  ← sensei.stances rows
// KitLadderRung { id; kanji; scope; name; caption; rules: KitRule[] }    ← rulesToLadder(ConstitutionRule[])
// KitRule       { kanji; text; hard; level }                             ← hard = isHardRule(enforcement==='mandatory')
```
The ladder half is mapped by `constitution-map.rulesToLadder` (built + unit-tested, currently
unwired); the stance half maps `sensei.stances` rows → `KitStanceDial[]`.

**State** — `constitution-state.svelte.ts` → `constitutionState` (**new** — today the screen
reads fixtures through `personal-view.personalRungs`, with local `$state` living inside `StanceDial`)
- data: `stance: KitStanceDial[]`, `ladder: KitLadderRung[]`
- `$derived`: `rungs` = `personalRungs(ladder)` (the personal + stack rungs that apply to every project)
- methods: `load(c: PersonalConstitution)`, `setDial(id, value)` (moves a dial locally; the single
  seam a write path hooks onto)
- **no realtime** — governance is a pull, stance is daemon-local

**Load** — `constitution.ts` → `loadConstitution()` (wired in `(app)/you/[section]/+page.ts`, section `rules`)
- mock-first: hand-crafted `PersonalConstitution` exercising a full ladder, a mandatory ★ lock, and
  empty (`rulesToLadder([]) → []`) → build UI + tests to fidelity NOW
- real (body-swap only): read the resolved personal ladder (`shared_rules ⋈ namespaces` for the
  user's namespaces, ordered by `scopes.level`) → `rulesToLadder`; read stance from the daemon
  (`sensei.stances`) → `KitStanceDial[]`. **Two planes** — the tenant `/v1` ladder read and the
  daemon-local stance read are distinct backends (see open questions on cross-DB access).

**Components** (pure, semantic, own styles + `md:`)
- `ScrConstitution` — shell: header + "Rule packs →" nav + 静 banner + 3-up `StanceDial` grid +
  "Your effective constitution" `LadderRung[]`. Reads `constitutionState`; dial change → `setDial`.
- `StanceDial` — one `KitStanceDial`: labelled discrete slider snapping across `levels`; emits
  `(id, value)`. **Mockup-match + `md:` live here.**
- `LadderRung` + `RuleRow` — a rung and its rules; `RuleRow` surfaces the mandatory ★ (`rule.hard`).
  Already semantic — kept.

**Copy** (paraglide `m.<key>()` from `$lib/paraglide/messages` — no inline literals): `m.constitution_title()`,
`m.constitution_eyebrow()` "You · standing rules", the 静 stays-local banner, per-dial label/caption,
empty copy. Kanji (静/己/技/守) stay `KanjiToken` brand marks, not messages. (The stance `sharing`
dial is a sharing *posture*, NOT `attribution_mode` — keep the two vocabularies separate.)

**Realtime = State**: none — a refetch re-runs `load`, not a live `patch`. **Test seams:**
`constitutionState` methods (no DOM); `StanceDial`/`LadderRung` with a mock prop (fidelity);
`loadConstitution` mock → `PersonalConstitution` shape.

**Rung-id carry (blocks the real ladder):** `personalRungs` filters `id ∈ {personal, stack}` but
`rulesToLadder` emits `id = scope_key` (`user`, `technology`, …) → wiring the mapper today yields `[]`.
`$derived rungs` must key off the reconciled vocabulary (Gap/to-do + Open question 3) before the mock→real swap.

**New open question (three-layer):** should `loadConstitution` fan out to two planes in one call
(tenant `/v1` ladder + a daemon-local stance read), or — given the dōjō has no cross-DB link to the
daemon — should the web screen render **ladder-only** and drop the stance dials until a stance read
route exists (keeping stance edits desktop/MCP-only)?
