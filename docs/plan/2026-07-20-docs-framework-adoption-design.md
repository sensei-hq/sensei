---
name: Docs framework adoption + front-door exemplar — design
date: 2026-07-20
status: design — approved in brainstorm 2026-07-20; plan next
spec: docs/plan/operating-model.md §5 (canonical doc structure) + §3.2 (per-feature dossier)
audience: product owner (validate) · designer (build screens) · developer/agent (ground + build)
---

# Docs framework adoption + front-door exemplar — design

## Problem

The docs feel thin/partial from two perspectives: the **product owner** has no readable,
validatable feature-specification document; the **agent/LLM** grounds itself by synthesizing
from scattered sources (`plan/operating-model.md` + dated `plan/*` + `journeys/` + code + memory)
rather than one complete picture.

Root cause — **a designed framework that was never adopted**, leaving two competing structures:

- The operating model already specifies the target: §5 canonical folders
  (`vision/personas/journeys/roadmap/design/mockups/features/`) **and** §3.2 a per-feature
  **dossier** (`docs/features/<name>/{brief, design, plan, tests, decisions, mockup-ref}`), and the
  `sensei scaffold feature <name>` tooling exists.
- But `docs/features/` **does not exist** — no feature was ever written as a dossier. The real
  detail stayed in `operating-model.md` + dated `plan/*`; the *current* folders
  (`requirements/architecture/spec/plan`) are an older, different structure.
- Result: the "complete feature spec" unit (the dossier) exists in design + tooling but has **zero
  instances**, and detail lives in the wrong *stage* (strategy/plan, not a durable feature layer).

A dogfood of `/sensei:intake` on this very docs chunk classified it **stable / ux / low → no rule
matched → defaulted `gsd`**, confirming the front-door taxonomy is code-centric (no
documentation/definition intent) and the rule matrix has holes — findings this effort must record.

## Decisions (brainstorm 2026-07-20)

1. **Framework first + front-door as the exemplar** — establish the documentation *system*, then
   write one complete feature spec against it (not a full-vision rewrite).
2. **Adopt the operating-model §5 canonical + the per-feature dossier**; reconcile the current
   folders into it **incrementally** (no big-bang migration).
3. **`architecture/` stays** alongside the canonical (technical "how," per surface).
4. The front-door dossier's **acceptance criteria encode the current *partial* state** — what is
   built vs not — so it doubles as the checklist to verify `/sensei:intake` behaves correctly now.

## Section 1 — Target structure + stages × audiences map

`docs/README.md` becomes the **map**: it names each stage, its purpose, and its primary audience
(this is the missing piece — today nothing states who a folder is for). Target + reconciliation:

| Stage (folder) | Purpose | Primary audience | Current → target |
|---|---|---|---|
| `vision/` | why sensei exists; north star | product owner, all | ← `requirements/vision.md` + `objectives.md` |
| `personas/` | who we serve + their goals | PO, designer | **new** (stub now, fill later) |
| `journeys/` | end-to-end flows | PO, designer, dev | exists (thin) → enrich over time |
| `roadmap/` | phases, sequencing, status | PO | ← `plan/README` + `spec/EXECUTION-PLAN` |
| `design/` | design system + cross-cutting UX | designer, dev | ← `architecture/frontend-svelte-guidelines` + `mockups/STYLING` |
| `mockups/` | system-wide mockup bundle | designer, dev | exists (keep) |
| **`features/<name>/`** | **complete per-feature spec (dossier)** | **PO (validate) + agent (ground) + dev (build)** | **new — the key layer** |
| `architecture/` | technical "how," per surface | dev, agent | keep |
| `spec/` | legacy per-screen/relay build specs | dev | **transitional** — content folds into the relevant `features/<f>/design.md` or `architecture/` as each is touched; not the feature source-of-truth |
| `plan/*` (+ per-feature `plan.md`) | dated, **transient** build plans | dev/agent | keep, **marked transient** in the map |

**Reading paths (stated in the map):**
- **Product owner** → `vision/` → `features/<name>/brief.md` + `design.md` (validate) → `journeys/`.
- **Designer** → `personas/` → `journeys/` → `mockups/` + `design/` → `features/<name>/mockup-ref.md`.
- **Developer / agent** → `features/<name>/` (the complete truth) → `architecture/` → `plan/*` (current build).

The map explicitly declares: **the `features/<name>/` dossier is the source of truth for a
feature**; `operating-model.md` is the *strategy* (why + system); `plan/*` is *transient* build
detail. This is what lets the agent ground on one place and the PO validate one place.

## Section 2 — The feature dossier is the complete-spec unit; front-door is the first

**Dossier shape** (already produced by `sensei scaffold feature <name>`, per §3.2):
`brief.md` (intent — user objective + the axes the feature touches) · `design.md` (behavior, states,
data contract, depth-by-risk, cross-layer contract, gates) · `plan.md` (tasks) · `tests/`
(Outcomes/Signals = acceptance) · `decisions.md` (append-only) · `mockup-ref.md` (optional link into
the system mockup). Constraints/governance stay **live** (`get_rules`) — never a folder.

**`docs/features/front-door/` (the exemplar)** is written completely, consolidating today's scattered
detail into the dossier slots:

- `brief.md` — the front door's purpose (adaptive process; recommend-and-confirm a playbook at the
  start of a chunk) + the three axes it is built on (lifecycle `greenfield|stable`, intent
  `explore|ux|feature|enhancement|bug`, risk `low|high`) and **how each is inferred**.
- `design.md` — the intake flow + **both surfaces** (CLI `/sensei:intake` conversational; the app
  `/intake` screen — states describe/loading/recommended/recorded/error); the **playbook catalog**
  (6 playbooks) + the **axes→playbook rule matrix** (as a table, so the coverage is legible); the
  learning loop (FTR attribution) + auto-select-on-trust; the data contract (guide + recommend
  endpoints, session-less-by-design for the app). Depth is dialed to the fact that this is
  shipped-but-partial.
- `mockup-ref.md` — link into the designer's landing screens (intake / catalog / recommendation
  card) once they land; until then, note "pending" + reference `requirements/front-door.md` content.
- `tests/acceptance.md` — acceptance criteria that **encode the current partial state**: what is
  built + verified (recommend/preview/confirm round-trip, both surfaces, the e2e), what is stubbed
  (learning loop shipped-not-released; nudge OFF-by-default→now-activated), and what is a **known
  gap** (below).
- `decisions.md` — carries forward the key decisions (persist-on-confirm, session-less app form,
  auto-select thresholds) + the model gaps as decisions-to-make.

**Model gaps the dossier records** (surfaced by the dogfood — the docs must reflect where the
implementation is incomplete/incorrect):
- **Intent taxonomy is code-only** — no "documentation / product-definition" intent; a docs/vision
  chunk is forced to `ux`.
- **Rule-matrix holes** — e.g. `stable+ux+low` matches no rule → silent default to `gsd`. The
  dossier includes the full 2×5×2 matrix and marks the uncovered cells.

`requirements/front-door.md` (the designer-facing requirements I wrote earlier) is **superseded**:
its content moves into `features/front-door/{brief,design}.md`; the file is removed or left as a
one-line pointer to the dossier (to avoid a third competing copy).

## Section 3 — Scope now vs incremental

**In scope now:**
1. `docs/README.md` rewritten as the stages × audiences map (with reading paths + the
   source-of-truth + transient declarations).
2. Canonical folders created: `vision/`, `personas/`, `roadmap/`, `design/`, `features/` (+ existing
   `journeys/`, `mockups/`, `architecture/`). Missing stages get a short stub README stating purpose
   + audience + "to be filled" (so the structure is legible even where empty).
3. `vision/` populated from the existing `requirements/vision.md` + `objectives.md` (moved, not
   rewritten); `roadmap/` seeded from `plan/README` + `spec/EXECUTION-PLAN` (pointer + summary).
4. **`docs/features/front-door/` written completely** as the exemplar dossier (Section 2).
5. A short **migration policy** in `docs/README.md`: how the old folders map, that migration is
   feature-by-feature as each is touched, and that `spec/` + dated `plan/*` are transitional/transient.

**Not now (incremental, follow-on):** migrating every other feature/plane into a dossier;
enriching `personas/` + `journeys/`; folding all of `spec/` into dossiers. These happen as each area
is next worked.

## Units & interfaces (isolation)

| Unit | Responsibility | Depends on |
|---|---|---|
| `docs/README.md` (map) | declare stages, audiences, reading paths, source-of-truth, migration policy | the target structure |
| canonical folder stubs | make each stage legible (purpose + audience) even when empty | — |
| `vision/`, `roadmap/` seed | move existing high-level content into the right stage | current `requirements/`, `plan/README`, `spec/EXECUTION-PLAN` |
| `features/front-door/` dossier | the first complete, validatable feature spec | `scaffold feature`, operating-model §3.3/§9, current code + `requirements/front-door.md`, plan/* |

## Testing / how we know it worked

- **Product-owner check:** Jerry can open `docs/features/front-door/` and read the complete feature
  (purpose → axes → flow → surfaces → playbooks → acceptance) without cross-referencing five files.
- **Agent-grounding check:** the front-door dossier + the map answer "how does the front door work +
  what's built" from one place (no synthesis from `operating-model.md` + `plan/*`).
- **Structure check:** `docs/README.md` names every stage's audience; every canonical folder exists
  (stub or filled); no folder's purpose is ambiguous.
- **Intake-verification check:** `features/front-door/tests/acceptance.md` lists the current built vs
  partial vs gap state, so `/sensei:intake` can be validated against a written expectation (incl. the
  intent-taxonomy + rule-matrix gaps as known-open).

## Scope / deferred

**In:** the map, canonical folder adoption (+ stubs), vision/roadmap seeding, the complete front-door
dossier, the migration policy. **Out (incremental):** dossiers for other features/planes; personas +
journeys enrichment; full `spec/` fold-in; **fixing** the intake model gaps (intent taxonomy +
rule-matrix — recorded here, fixed as a separate feature chunk); deleting legacy folders (kept until
their content is fully migrated).
