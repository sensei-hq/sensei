# Knowledge — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.
- Route: `/org/[slug]/knowledge` — served by `(app)/org/[slug]/[section]` (`section === 'knowledge'`). No `/v1` read wired.
- Mockup: dojo2-app.jsx `ScrKnowledge` (L1123)
- Access axis: **tenant-primary** — org-console governance (the published library the dōjō has adopted). Canonical `docs/architecture/entity-access-model.md` §3 (Governance / Org console → `tenant_id`). The real backing table `dojo.artifacts` is tenant-scoped.
- Status: **STUB** — 100% fixture. Loader returns `knowledgeFor(slug)` from `components/kit/fixtures.ts` (only `acme` authored; every other slug → empty `{prunePolicy:'', active:[], pending:[], catalog:[]}`). No endpoint, no loader fetch, no mutations. The real source (`dojo.artifacts` via the federation route) exists but is NOT read by this screen.

## ⚠ Fabricated-data debt — MUST fix on build (2026-07-29 fallback audit)
`(app)/org/[slug]/[section]/+page.ts:230` (`knowledge: knowledgeFor(slug)`) renders a fabricated knowledge library for real users (active/pending/catalog authored only for `acme`). **Impact:** a real maintainer sees an invented published/adopted library as if the dōjō had adopted it. **Fix on build:** drive every field from the real `/v1` read; on a fetch error render an explicit error state — NEVER the fixture; honest-empty only when genuinely empty. (Ties the daemon-side fabrication fixes + the "no fabricated fallbacks" rule.)

## Elements → data (contract)
| Element | Mockup field | Source (loader/API/table.field) | Status: have/bind/plumb | Realtime? |
|---|---|---|---|---|
| Prune-policy readout | `k.prunePolicy` | `KitKnowledge.prunePolicy` — **fixture literal** "Prune after 90 days unused"; static, not a control | plumb | no |
| Banner (蔵) copy | literal | literal in component | have | no |
| Active · count | `k.active.length` | fixture `knowledge[slug].active` | plumb | no |
| Active row kanji/title | `r.kanji` / `r.title` | fixture `KitKnowledgeRow` — real source `dojo.artifacts.kind`(→kanji) / `.title` where state=published/adopted | plumb | no |
| Active row meta | `r.scope · r.adopted · r.age` | fixture — real: `owner_scope` / adoption reach (downstream telemetry `adopted_count`) / age from `created_at` | plumb | no |
| Active · Edit button | `<K2Btn>Edit` | none — static, no handler/route | plumb | no |
| Pending prune · count | `k.pending.length` | fixture `knowledge[slug].pending` | plumb | no |
| Pending row title/meta | `r.title` / `r.scope · r.age` | fixture — real: artifacts unused past the prune window (`age` from last-adoption/telemetry) | plumb | no |
| Pending · Keep button | `<K2Btn>Keep` | none — static, no handler (would cancel the prune) | plumb | no |
| Catalog · count | `k.catalog.length` | fixture `knowledge[slug].catalog` | plumb | no |
| Catalog row icon | `K2_EXT_ICON[c.kind]` | `EXT_ICON` map (agent/command/skill) — presentational | have | no |
| Catalog row title/scope | `c.title` / `c.scope` | fixture — real: `dojo.artifacts` of `kind ∈ {agent, command, skill}` (marketplace extensions) | plumb | no |
| Catalog kind chip | `c.kind` | fixture `KitCatalogItem.kind` = `dojo.artifacts.kind` | plumb | no |

## APIs / loaders
- **Loader** `(app)/org/[slug]/[section]/+page.ts` L230: `knowledge: knowledgeFor(slug)` — pure fixture, no fetch, no `guardTenantScope`. Explicitly flagged Tier-3/unbuilt in the loader header comment ("knowledge … still render off kit fixtures — their routes aren't built").
- **No `GET …/knowledge` endpoint exists.** Nearest real data:
  - `dojo.artifacts` — the federated library (`POST/GET /v1/.../artifacts`, `$lib/server/artifacts-data.ts`: `publishArtifact` / `pullArtifactsSince` / `promoteCluster`). This is the **daemon plane** (`resolveApiKeyAccess`, device-token) delta-pull for machines, NOT a JWT-plane console read. A console "list adopted artifacts" read would be a new endpoint at MAINTAINER floor.
  - Prune policy would live on `dojo.policies.retention_days` (per-scope; `admin-data.ts` already reads/writes `policies` for the admin surface).
- **No mutations** — Edit / Keep / prune-policy change have no write path.

## Interactions & states
- Fully presentational; no state store, no empty-state handling in the component (an empty fixture just renders three empty ListSections with count 0). A non-`acme` org renders an all-empty Knowledge screen honestly (no fake fallback).
- Buttons (Edit, Keep, prune-policy dropdown-looking readout) are inert.

## Gap / to-do (vs mockup)
1. **Build a console read endpoint** `GET /v1/.../knowledge` (or `…/artifacts?view=library`) at MAINTAINER floor over `dojo.artifacts` (tenant-scoped): published/adopted rows → Active, unused-past-prune rows → Pending prune, `kind ∈ {agent,command,skill}` → Catalog. Map wire→`KitKnowledge` with a new `*-map`; fetch it via `guardedFor('knowledge', …)` in the loader (mirror triage/health).
2. **Adoption reach + age** need downstream telemetry (`adopted_count`, last-adoption ts) — the same "metrics flow back within 14d" the maintainer-console done-gate calls for. Absent that, `adopted`/`age` stay approximate.
3. **Prune policy = real control.** Bind the readout to `dojo.policies.retention_days` for the scope and make it editable (admin write path already exists for `policies`).
4. **Wire Edit / Keep.** Edit → artifact edit (revise/republish, bumping `seq`). Keep → cancel the pending prune (reset the unused-since clock / exempt flag).
5. Published artifacts are source-dereferenced on the publish path (universal invariant, always-on); `attribution_mode = named | anonymous` (credit only) — if the library shows a contributor, honor the mode.

## Open questions (for Jerry)
- Is the console library read a new JWT-plane `GET …/knowledge`, or do we reshape the existing daemon-plane `…/artifacts` pull for console use? (Auth plane differs — device token vs Supabase session.)
- "Pending prune" needs a definition of *unused*: last adoption? last downstream pull? no telemetry at all yet — what's the v1 signal, and is `dojo.policies.retention_days` the prune window?
- Does Catalog (skills/agents/commands) come from `dojo.artifacts` of those kinds, or from the marketplace subtree? They may be different libraries.
- Edit/Keep — are these v1, or is Knowledge a read-only library for now (matching how thin the real data is)?

### Resolved design (2026-07-30)
- **Q1 read → new JWT-plane `GET /v1/t/{tenant}/knowledge`** (Supabase-session plane, matching the other console screens) reading `dojo.artifacts`. NOT the device-token daemon-plane `/artifacts` pull.
- **Q2 prune signal → "unused" = no downstream adoption/pull within `dojo.policies.retention_days`** (retention_days is the window). Telemetry-light; reuses the retention policy.
- **Q3 Catalog → `dojo.artifacts` where `kind ∈ {skill, agent, prompt}`** (the tenant's federated library), NOT the marketplace subtree.
- **Q4 mutations → READ-ONLY library for v1.** No Edit/Keep writes; edits happen via triage/authoring.
- **Build constraint:** drop the `acme` fixture; drive from the real `/knowledge` read; honest-empty (not fixture) for other slugs; error state on failure.
- **Depends on:** the new JWT-plane `/knowledge` endpoint over `dojo.artifacts` + the retention_days prune computation.

## Components & state (three-layer, per `sensei:ui-state-pattern`)

**DB** (reference §Elements→data): real source = `dojo.artifacts` (tenant-scoped) — published/adopted → Active, unused-past-prune → Pending, `kind ∈ {agent,command,skill}` → Catalog; prune window = `dojo.policies.retention_days`. STUB today (`knowledgeFor(slug)` fixture, only `acme` authored).

**API** (reference §APIs/loaders): **no console read endpoint exists** — the daemon-plane `…/artifacts` delta-pull (device-token) is NOT a JWT console read. Needs a new `GET /v1/.../knowledge` at MAINTAINER floor. Edit (revise/republish, bump `seq`) and Keep (cancel pending prune) have **no write path**; prune-policy edit can reuse the existing `dojo.policies` admin write.

**Domain types** (UI-shaped):
```ts
type KnowledgeLibrary = { prunePolicy: { retentionDays: number; label: string };
  active: Artifact[]; pending: Artifact[]; catalog: CatalogItem[] }
type Artifact = { id; kanji; title; scope; adopted: number; age: string;
  contributor?: { name; attributionMode: 'named'|'anonymous' } }
type CatalogItem = { id; icon; title; scope; kind: 'agent'|'command'|'skill' }
```

**State** — `knowledge-state.svelte.ts` → `knowledgeState`
- data: `library: KnowledgeLibrary`
- `$derived`: `activeCount`, `pendingCount`, `catalogCount`
- methods: `load(library)`, `edit(id)`, `keep(id)` (cancel prune), `setPrunePolicy(days)`

**Load** — `knowledge.ts` → `loadKnowledge()`
- mock-first: hand-crafted `KnowledgeLibrary` (mirror the `acme` fixture — active/pending/catalog +
  a prune policy) so this STUB screen **builds to fidelity NOW**; replaces the `knowledgeFor(slug)`
  fixture that rendered every non-seed org all-empty
- real (body-swap only): the new `GET …/knowledge` over `dojo.artifacts` + `dojo.policies.retention_days`;
  `adopted`/`age` from downstream telemetry (approximate until the metrics-flow-back lands). Published
  rows are **source-dereferenced** on the publish path (always-on); a shown contributor honors
  `attribution_mode = named|anonymous` (credit only)

**Components** (pure, semantic, own styles + `md:`; NO `K2*`)
- `KnowledgeLibrary` shell — banner (蔵 `KanjiToken`) + `PrunePolicyControl` + three sections.
- `ActiveArtifactList` / `PendingPruneList` — `ArtifactRow[]` (kind glyph Solar · title ·
  scope·adopted·age) with Edit / Keep affordances. **Mockup-match + `md:` here.**
- `CatalogList` — `CatalogRow[]` with a Solar icon per kind (agent/command/skill).
- `PrunePolicyControl` — retention-days editor via **`@rokkit/forms`** (bound to `retention_days`);
  Edit/Keep confirms likewise.
- Shell reuses `(app)/org/[slug]/[section]`; `+page.ts` = Load wiring → `knowledgeState.load`.

**Copy** (paraglide `m.<key>()`): section titles / prune-policy readout / button labels in
`messages/en.json`; 蔵 stays a `KanjiToken`, kind/action glyphs are **Solar icons**.

**Realtime = State**: none. **Test seams:** state methods incl. `keep`/`setPrunePolicy` (no DOM);
`ArtifactRow`/`CatalogRow` with a mock prop (fidelity); Load mock → shape.
