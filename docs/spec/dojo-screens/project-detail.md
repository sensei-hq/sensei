# Project detail (constitution preview) — dōjō screen spec

> **Before building this screen, follow the [Screen-build runbook](./SCREEN-BUILD-RUNBOOK.md)** — render + compare the mock, uno utilities (not var-space), rokkit components, tokens via rokkit.config, independent scroll + sticky headers, zero-errors gate.

- Route: `/you/projects/[id]` = `(app)/you/projects/[id]/+page.svelte` + `+page.ts`
- Mockup: dojo2-app.jsx `ScrProjectPreview` (L244) — boards "3 · Project preview · company (lumen-auth)" + "3b · Project preview · client (globex-portal) — client rung on"
- Access axis: **user/membership-primary** for the *surface* (a personal drill-in from the user's projects list), but the *content* it renders — the resolved constitution ladder + discarded conflicts — is **tenant/governance-derived** (canon §3 row 3: "Governance: rules · ladder/scopes · rule-packs · constitution → Tenant `tenant_id`"). The preview composes the user's personal rungs with the project's bound dōjō's rungs; the resolution is per-project, read by the user.
- Status: **STUB** — the screen (`ScrProjectPreview.svelte` + `preview/state.svelte`) is fully built and faithful, but `+page.ts` resolves the project + ladder + conflicts entirely from `kit/fixtures`. No `/v1` governance-resolve endpoint feeds it.

## Elements → data (contract)
Live: `projects/[id]/+page.ts` → `projects.find(id)`, `ladder`, `conflicts` (ALL from `$lib/components/kit/fixtures`) → `ScrProjectPreview.svelte`. `hasMembership===false` or unknown id → redirect to `/you/projects`.

| Element | Mockup field | Source (loader/API/table.field) | Status | Realtime? |
|---|---|---|---|---|
| back header | `Back to projects` | `onBack`→`youHref('projects')` | have | — |
| SectionHead kanji/eyebrow | `件` / `Before you start · {p.repo}` | `project.repo` (**fixture**) | bind→plumb | — |
| SectionHead title | `p.name` | `project.name` (**fixture**) | bind→plumb | — |
| classification chip | `p.classification` | `project.classification` (**fixture**) — drives `previewRungs` composition | bind→plumb | — |
| phase pill | `p.phase` | `project.phase` (**fixture**) | bind→plumb | — |
| classification banner | client vs "most-specific wins" | `project.classification` + counts | have (from above) | — |
| banner counts | `{scopes} scopes · {rules} apply · {locks} locked · {discarded} discarded` | `preview.rungs/effective/locks/discarded` (derived in `preview/state`) | have (over fixtures) | — |
| by-layer / consolidated toggle | `view` | `preview.view` rune state | have | — |
| ladder rungs (by-layer) | `previewRungs(p)` per classification | `preview.rungs` from `ladder` fixture, composed by classification: personal→[personal,project,stack]; client→[company,client,personal,project,stack]; else→[company,personal,project,stack] | bind→plumb | — |
| rung rules / ★ locks | `r.rules[].{text,hard,kanji}` | `KitLadderRung.rules` (**fixture**) — the actual resolved rules for THIS project | plumb | — |
| consolidated rule rows | `effective` flatMap, discarded filtered | `preview.effective` (rungs minus `conflicts[].loser.text`) | have (over fixtures) | — |
| rule-row jump | `onJump`→focus rung | `preview.jumpTo(level)` | have | — |
| discarded/conflicts section | `showConflicts` (non-personal) | `conflicts` (**fixture**) → `ConflictCard` | bind→plumb | — |
| conflict card | `c.{topic,loser,winner,why,locked}` | `KitConflict` (**fixture**) | plumb | — |

## APIs / loaders
- **load()** (`projects/[id]/+page.ts`): guard on `hasMembership`; `projects.find(p => p.id === params.id)` from fixtures (unknown → `redirect(307, /you/projects)`); returns `{ project, ladder, conflicts }` all fixture. Comment: "presentational — real `/v1` wiring is a later chunk."
- **mutations**: none (read-only preview).
- **realtime**: none.
- **Would-be source**: a governance-resolve endpoint that, given a project + the user's memberships, returns the composed ladder (company/client/personal/project/stack rungs), the effective rule set, the ★ locks, and the conflicts the ladder discarded. The daemon owns rule resolution (`render_rules_tiers` / `resolve_local_pack_raws` per project memory), but the **dōjō has no `/v1` route** exposing it, and the project identity itself is unresolved (see `projects.md`).

## Interactions & states
- **By-layer ↔ Consolidated** toggle (`preview.setView`); by-layer taps a rung to focus (`preview.setActive`); consolidated rule-row `onJump` switches back to by-layer focused on that level. State seeded ONCE at mount via `untrack(createProjectPreview(...))` — opening a different project re-mounts (`/you/projects/[id]` nav).
- **Conflicts section** hidden for `classification === 'personal'` (`preview.showConflicts`).
- **Dead-ends guarded**: no-membership / unknown id → redirect to the projects list (never renders a fabricated project).
- **Responsive**: `ScrProjectPreview` hard-codes `p-8 gap-6` (no `mobile` prop threaded from the route); the by-layer grid is single-column already. Verify phone layout via the shell's MobileShell.

## Gap / to-do (vs mockup), ranked
1. **Real resolution source** — expose the daemon's per-project constitution resolution (composed rungs + effective rules + ★ locks + discarded conflicts) as a dōjō-readable `GET /v1/…` keyed by project + the user's memberships. Everything on this screen is fixture until then.
2. **Project identity** — blocks on `projects.md` Gap 1 (there is no real `project.id` to resolve). The preview can't be real before the projects list is.
3. **Classification/phase** must be real (they drive `previewRungs` composition and the banner). Confirm these federate from the daemon.
4. **Client dereference banner** — for `classification === 'client'` the banner says "sources are dereferenced." Align copy with canon §5: dereference is **universal/always-on**, not client-only. The reassurance is fine, but don't imply personal/company work is un-stripped when it crosses a boundary.
5. **Thread `mobile`** or confirm the shell handles the phone layout.

## Open questions (for Jerry)
1. Does the dōjō web app render the resolved constitution at all, or is "before you start" a **desktop-app** surface (where the daemon already resolves rules locally)? The dōjō governance screens are tenant-authoring; a per-project *resolution* preview may belong in the app, not the web. [jt] we have projects page and each project has the detail page in mockup for this sidebar projects and click on project item.
2. If in the dōjō: one resolve endpoint returning the whole composed ladder, or does the dōjō re-resolve client-side from tenant rules + the user's personal rules + packs (which it would also need to fetch)? [jt] one resolve endpoint. 
3. Conflict resolution ("discarded by the ladder") — is the winner/loser decision computed server-side (daemon) and just displayed, or does the dōjō need the resolution algorithm too? [jt] server side
4. The client banner's dereference wording — reword to the universal invariant, or keep client-specific framing for the engagement context? (Impacts `data-model-fix-impact-register.md` Rule B doc sweep.) [jt] reword to universal invariant

### Resolved design (2026-07-30)
- **Q1 → the dōjō web DOES render the per-project constitution preview:** projects page → sidebar project item → click → this detail page (per mockup). Not app-only.
- **Q2 → ONE resolve endpoint:** `GET /v1/…/projects/[slug]/constitution` returns the WHOLE composed ladder (rungs by classification + effective rules + ★ locks + discarded conflicts). The dōjō does NOT re-resolve client-side.
- **Q3 → conflict winner/loser computed SERVER-SIDE** (the daemon's resolution); the dōjō just displays. The dōjō does not carry the resolution algorithm.
- **Q4 → client banner copy reworded to the universal always-on dereference invariant** (`ScrProjectPreview.svelte` "Client engagement — sources are dereferenced." → universal framing; small copy change).
- **Depends on:** `dojo.projects` (the slug/classification/phase source) + the daemon exposing per-project constitution resolution to a dōjō-readable `/v1` route (a governance-resolution federation seam) + WS-0 Rule A (user-membership-keyed).

## Components & state (three-layer, per `sensei:ui-state-pattern`)

**DB** tenant governance (rules · `rule_packs`) resolved by the daemon (`render_rules_tiers`/
`resolve_local_pack_raws`); the dōjō holds no resolution table, read-only · **API** Load `loadProjectPreview`
in `projects/[id]/+page.ts` (today all `kit/fixtures`); would-be `GET /v1/…/projects/[slug]/constitution`
keyed by (project + the user's memberships) · **UI** `projectPreviewState` + `ConstitutionPreview` over the
`ProjectConstitution` domain type.

**Domain types** (UI-shaped; Load maps the resolve response → these):
```ts
type ProjectConstitution = { project: Project /* shared with projects.md */;
  rungs: LadderRung[]; effective: Rule[]; conflicts: Conflict[] }
type LadderRung = { level: 'company'|'client'|'personal'|'project'|'stack'; label; rules: Rule[] }
type Rule = { id; text; hard: boolean /* ★ lock */; icon?: string /* Solar glyph */ }
type Conflict = { topic; loser: Rule; winner: Rule; why; locked: boolean }
```
Rung composition is driven by `project.classification`: personal→[personal,project,stack];
client→[company,client,personal,project,stack]; else→[company,personal,project,stack].

**State** — `project-preview-state.svelte.ts` → `projectPreviewState` (semantic rename of `preview/state.svelte`
`createProjectPreview`)
- data: `constitution: ProjectConstitution`, `view: 'by-layer'|'consolidated'`, `active: LadderRung['level']|null`
- `$derived`: `effective` (rungs flattened minus `conflicts[].loser`), banner counts
  (`scopes`/`applies`/`locks`/`discarded`), `showConflicts` (`classification !== 'personal'`)
- methods: `load(constitution)`, `setView(v)`, `setActive(level)`, `jumpTo(level)` (consolidated row →
  by-layer focus). Seeded once at mount via `untrack`; a different `[id]` re-mounts.

**Load** — `projects/[id]/+page.ts` → `loadProjectPreview(id)`
- mock-first: the existing `kit/fixtures` ladder/conflicts stay the mock (already exercise
  company/client/personal composition + discarded conflicts) → the screen is fidelity-complete NOW
- real (body-swap only): call the resolve endpoint; guard `hasMembership`/unknown id →
  `redirect(307, /you/projects)` (never a fabricated project). Depends on `projects.md` delivering a real
  `project.id` + `classification`.

**Components** (pure, semantic, own styles + `md:` — fidelity verified per component)
- `ConstitutionPreview` — shell: back header · `SectionHead` (kanji 件 brand) · classification banner + counts
  · by-layer/consolidated toggle · rungs / consolidated list · conflicts. Reads `projectPreviewState`.
  (replaces `ScrProjectPreview`)
- `LadderRung` — one rung: label + `RuleRow[]`; `onjump→state.setActive`.
- `RuleRow` — one `Rule`: text + ★-lock (Solar lock icon, not kanji) when `hard`.
- `ConflictCard` — one `Conflict`: topic · loser→winner · why · locked. (kept)
- Toggle: the binary by-layer/consolidated view is a plain Toggle (not `@rokkit/forms` — no option/answer form
  on this screen).

**Copy** (paraglide `m.<key>()`): banner variants (client vs most-specific-wins), count line
`m.preview_counts({scopes,applies,locks,discarded})`, dereference reassurance reworded to the **universal**
invariant (§5, not client-only). Kanji 件 stays a `KanjiToken` brand mark.

**Realtime = State**: none (read-only preview). **Test seams:** `projectPreviewState` view/jump/`effective` +
`showConflicts` (no DOM); `ConstitutionPreview`/`ConflictCard` with a mock `ProjectConstitution`; Load mock =
the fixtures.

**New open question (from this exercise):** `ProjectConstitution.project` reuses the shared `Project` type, so
this screen inherits projects.md's classification gap — a relay-derived project with no `classification` can't
compose the ladder. Confirm the resolve endpoint returns `classification` itself (authoritative, daemon-side)
rather than trusting the projects-list field.
