# Triage — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.
- Route: `/org/[slug]/triage` — served by `(app)/org/[slug]/[section]` (`section === 'triage'`); read API `GET /v1/t/[origin]/[org]/triage`.
- Mockup: dojo2-app.jsx `ScrTriage` (L1009)
- Access axis: **tenant-primary** — org-console governance surface. Canonical `docs/architecture/entity-access-model.md` §3: "Governance: rules · ladder/scopes …" and "Org console (`/org/[slug]`) → **Tenant** → `tenant_id`". The endpoint filters `dojo.triage_queue.eq('tenant_id', …)` at the MAINTAINER role floor.
- Status: **PARTIAL** — the ranked candidate list is REAL (`/v1/.../triage` → `dojo.triage_queue ⋈ dojo.artifacts`); the right-pane candidate detail is a best-effort projection, `conflicts` is always 0, and the Approve/Revise/Decline + "My scopes" affordances are unwired (the decide endpoint + client exist but the screen never calls them).

## ⚠ Fabricated-data debt — MUST fix on build (2026-07-29 fallback audit)
`(app)/org/[slug]/[section]/+page.ts:129` seeds `candidateDetail = candidateDetailFor(slug)` (fixture), and the catch at `+page.ts:145` sets `triageError` but does **NOT** reset `candidateDetail`. **Impact:** on a triage-fetch error the maintainer sees a fabricated candidate learning + fake evidence in the right pane instead of an error/empty state. **Fix on build:** drive every field from the real `/v1` read; on a fetch error render an explicit error state — NEVER the fixture; honest-empty only when genuinely empty. (Ties the daemon-side fabrication fixes + the "no fabricated fallbacks" rule.)

## Elements → data (contract)
| Element | Mockup field | Source (loader/API/table.field) | Status: have/bind/plumb | Realtime? |
|---|---|---|---|---|
| Section count | `groups.flatMap(items).length` | derived `triage.all.length` in `ScrTriage.svelte` from `data.triage` | have | no |
| "My scopes" button | `right={<K2Btn>My scopes</K2Btn>}` | none — static button, no handler/route | plumb | no |
| Banner (門) copy | literal | literal in component | have | no |
| Scope group header | `g.scope` | `KitTriageGroup.scope` ← `toKitTriageGroups` → `groupByScope(rows)` → `scopeLabel(row.owner_scope)`; `owner_scope` = `dojo.triage_queue.owner_scope` (opaque JSON) | have | no |
| Candidate kanji | `c.kanji` | `kindKanji(row.kind)`; `kind` ← embedded `dojo.artifacts.kind` | have | no |
| Candidate title | `c.title` | `TriageRow.title` ← embedded `dojo.artifacts.title` | have | no |
| Candidate origin | `c.origin` | `originLine()` = `contributor_count` + `relativeAge(created_at)`; `dojo.triage_queue.contributor_count` / `.created_at` | have | no |
| Conflict chip | `c.conflicts` | **hardcoded 0** in `toKitCandidate` — list route carries no ladder-conflict count | plumb | no |
| Dup chip | `c.dups` | derived: `similarity ≥ 0.75 && nearest_artifact_id` → 1; `dojo.triage_queue.similarity` / `.nearest_artifact_id` | bind | no |
| Impact chip | `c.impact` | `impactForConfidence(confidence)` (≥0.90 ⇒ `high`, else `normal`); `dojo.triage_queue.confidence` — **derived, not a real safety/impact tag** | bind | no |
| Confidence bar / enso | `c.conf` | `TriageRow.confidence` = `dojo.triage_queue.confidence` (0..1, null→0) | have | no |
| Detail · Learning | `d.learning` | `toKitCandidateDetail(topRow).learning` = **`row.title`** (projection — no rich read) | plumb | no |
| Detail · Cause | `d.cause` | **empty** — no source on the list route | plumb | no |
| Detail · Evidence | `d.evidence[]` | **empty** — supporting sessions live on the cluster, not the queue row | plumb | no |
| Detail · Conflict (loser→winner) | `d.conflict` | **empty** `{loser:'',winner:''}` — never rendered (gated on `cur.conflicts>0`, always 0) | plumb | no |
| Detail · Distribution scope chips | `d.scopes[]` | `[scopeLabel(owner_scope)]` — single own-scope chip, not a real distribution picker | plumb | no |
| Approve / Revise / Decline | 3 `K2Btn` | endpoint `POST /v1/.../triage/{signature}/decide` + client `decideTriage()` EXIST; **`ScrTriage.svelte` renders bare `<Btn>` with no `onclick`** | plumb | no |
| Second-approval note | `cur.impact==='high'\|'safety'` | `needsSecondApproval(cur.impact)`; impact derived from confidence | bind | no |

## APIs / loaders
- **Loader** `(app)/org/[slug]/[section]/+page.ts` L132–147: when `section ∈ {triage, approvals}`, one `guardTenantScope(tenantKey, …, listTriage)` fetch feeds BOTH screens. `tenantKey = org.url`. Degrades to empty list + `triageError` on failure/403/dev-404.
- **Client** `$lib/triage-data.ts::listTriage()` → `GET {DOJO}/v1/t/{tenant}/triage` → `{ queue: TriageRow[] }`.
- **Endpoint** `routes/v1/t/[origin]/[org]/triage/+server.ts` — `resolveTenantAccess(..., ACCESS.maintainer)` then `$lib/server/triage-data.ts::listTriage(db, tenantId)`: `dojo.triage_queue` where `state ∈ {queued, in_review}`, `select` embeds `dojo.artifacts(kind, title)`, ranked strongest-first (`rankTriageRows`).
- **Mappers** `$lib/triage-map.ts`: `toKitTriageGroups`, `toKitCandidateDetail` (top-ranked row), `toKitApprovals` (feeds the approvals screen).
- **Decide (built, unwired to UI)** `POST /v1/.../triage/{signature}/decide` → `$lib/server/triage-data.ts::decideTriage` writes `dojo.decisions` (+ `maintainer_id`, reason, `distribution_scope`) and flips `triage_queue.state`. `approve` requires `distribution_scope` (400 without); `decline` requires `reason` (400 without). Client `decideTriage()` ready.

## Interactions & states
- Selection: `createTriage(groups)` rune store (`$lib/triage-state.svelte`), seeded once via `untrack` at mount (a different org re-mounts). Detail pane is desktop-only + sticky; mobile drops to a one-column list.
- Empty: honest EmptyState (静, "The triage queue is clear.") when `triage.all.length === 0` — covers both a genuinely-clear queue and a degraded/`triageError` fetch (loader returns `[]`).
- Errors: loader swallows to empty + `triageError`; the shared `+page.svelte` surfaces console-action failures via `actionError` toast (not used by triage today since no action is wired).

## Gap / to-do (vs mockup)
1. **Wire Approve/Revise/Decline** — call `decideTriage()` on the selected candidate's `signature`; Approve must collect a `distribution_scope`, Decline a `reason` (server 400s otherwise), then `invalidateAll()`.
2. **Rich candidate detail read** — cause/evidence/conflict/context require a per-artifact/cluster read the list route doesn't make; add a detail endpoint over `dojo.artifacts` + evidence (sessions). Evidence lines cross the machine boundary → must be **source-dereferenced** (universal invariant, `attribution.rs::dereference()`).
3. **Real conflict count** — `conflicts` is hardcoded 0; the ladder-conflict count isn't on the queue row. Needs a resolution-ladder read to populate the conflict chip + the "the ladder settles it" block.
4. **Impact = derived, not real** — `impact` is a confidence proxy (≥0.90 ⇒ high). A genuine safety/impact classification on `dojo.artifacts` would make the second-approval routing trustworthy.
5. **"My scopes" filter** — no scope-ownership filter; the queue shows every scope's rows (the maintainer-console "wrong gate" warns against exactly this). Needs the owner-scope filter from `dojo.roles`/scope ownership.
6. Distribution-scope chips are a single own-scope label, not the mockup's Company/Team/Stack picker.

## Open questions (for Jerry)
- Where does the rich candidate detail (cause · evidence · conflict) come from — a new `GET …/triage/{signature}` cluster read, or fold it into the list `select`? It drives the whole right pane.
- Should Triage filter to the viewer's **owned** scopes by default (mockup "My scopes"), or show the whole tenant queue? The maintainer-console spec treats cross-scope leakage as a bug.
- `impact`/second-approval today rides on confidence ≥0.90. Do you want a real impact/safety field on `dojo.artifacts`, or is the confidence proxy acceptable for v1?

### Resolved design (2026-07-30)
- **Q1 detail → new `GET /v1/…/triage/{signature}`** (per-cluster read): `{ cause, evidence[] (source-dereferenced — universal invariant), conflict{loser,winner}, scopes }`. The list route stays lean.
- **Q2 impact → a REAL impact/safety field on `dojo.artifacts`** (`normal | high | safety`) that drives the 2-sig second-approval routing — not the `confidence ≥0.90` proxy. Needs a classifier (analyzer/`derive_signals`) to set it + the column.
- **Build (endpoints exist):** wire Approve/Revise/Decline → `decideTriage(signature,…)` (Approve = distribution-scope picker; Decline = reason — via `@rokkit/forms` so a click can't post an invalid decide; server 400s otherwise) → `invalidateAll`. Real conflict count needs a resolution-ladder read.
- **Depends on:** the new detail endpoint + the `dojo.artifacts.impact` field/classifier + the decide wiring + a ladder-conflict read.
- Approve needs a `distribution_scope` payload — what does the picker offer (own scope only, or promote up the ladder like the mockup's Company/Team/Stack)?

## Components & state (three-layer, per `sensei:ui-state-pattern`)

**DB** (reference §Elements→data — don't restate): list = `dojo.triage_queue ⋈ dojo.artifacts(kind,title)` (state ∈ {queued,in_review}, ranked strongest-first); decide writes `dojo.decisions` (+`maintainer_id`, reason, `distribution_scope`) and flips `triage_queue.state`. Rich detail (cause/evidence/conflict) has **no table read yet** — a per-cluster read the list route never makes.

**API** (reference §APIs/loaders): read `GET /v1/t/{tenant}/triage` (MAINTAINER floor, behind `guardTenantScope`). Mutation `POST /v1/.../triage/{signature}/decide` + client `decideTriage()` **EXIST but are unwired** — the state's `decide()` method is the wiring. No `GET …/triage/{signature}` detail endpoint yet (open Q).

**Domain types** (UI-shaped; the Load layer maps wire → these):
```ts
type TriageCandidate = { id; signature; kanji; title; origin; confidence: number;
  conflicts: number; dups: number; impact: 'normal'|'high'|'safety' }
type TriageGroup = { scope: string; candidates: TriageCandidate[] }
type TriageDetail = { learning; cause: string|null; evidence: Evidence[];
  conflict: { loser: string; winner: string }|null; scopes: string[] }
type Evidence = { id; summary; sessionRef?: string /* dereferenced on the publish path */ }
```

**State** — `triage-state.svelte.ts` → `triageState` (consolidates today's `createTriage` rune store)
- data: `groups: TriageGroup[]`, `selectedId`, `detail: TriageDetail|null`, `myScopesOnly`
- `$derived`: `all` (flatten), `selected`, `count`, `shown` (`myScopesOnly` filter)
- methods: `load(groups)`, `loadDetail(detail)`, `select(id)`, `toggleMyScopes()`,
  `decide(verdict, { distributionScope?, reason? })` → `decideTriage(signature,…)` → `invalidateAll`
  (Approve requires `distributionScope`, Decline requires `reason` — server 400s otherwise)

**Load** — `triage.ts` → `loadTriage()` (+ `loadTriageDetail(signature)`)
- mock-first: hand-crafted `TriageGroup[]` across scopes exercising normal/high/safety impact,
  dup/conflict chips, and empty (clear queue) — build UI + tests to fidelity NOW
- real (body-swap only): `listTriage` → `toKitTriageGroups`; detail = the new per-cluster read
  (cause/evidence/conflict). Evidence crosses the machine boundary → **source-dereferenced**
  (`attribution.rs::dereference()`, always-on); any shown contributor honors
  `attribution_mode = named|anonymous` (credit only)

**Components** (pure, semantic, own styles + `md:`; NO `K2*`)
- `TriageList` — left: banner (門 `KanjiToken`) + "My scopes" toggle + scope-grouped
  `TriageCandidateRow[]` from `triageState.shown`; `onselect → state.select`.
- `TriageCandidateRow` — one candidate: kind glyph (Solar) · title · origin · conflict/dup/impact
  chips · `ConfidenceEnso(confidence)`. **Mockup-match + `md:` live here.**
- `TriageDetail` — right pane (desktop-only, sticky): learning · cause · `Evidence[]` · conflict
  (loser→winner) · distribution-scope chips · Approve/Revise/Decline. Reads `triageState.selected/detail`.
- decide affordances via **`@rokkit/forms`** (schema-driven): Approve = distribution-scope picker;
  Decline = reason field — so a click can't post an invalid decide.
- `ConfidenceEnso` · `TriageBanner` (banner shared with Approvals/Knowledge/Health).
- Shell: `(app)/org/[slug]/[section]` composes `TriageList` + `TriageDetail`; `+page.ts` = Load
  wiring → `triageState.load` (a different org re-mounts / re-seeds).

**Copy** (paraglide `m.<key>()`, no inline literals): banner / empty / second-approval-note / decide
labels in `messages/en.json` (e.g. `m.triage_empty()` "The triage queue is clear.",
`m.triage_my_scopes()`). Banner kanji (門) stays a `KanjiToken` brand mark; functional kind/action
glyphs are **Solar icons**.

**Realtime = State**: none today (no channel) — `load` is a snapshot. **Test seams:** state methods
incl. the `decide` guard (no DOM); `TriageCandidateRow`/`TriageDetail` with a mock prop (fidelity);
Load mock → shape.
