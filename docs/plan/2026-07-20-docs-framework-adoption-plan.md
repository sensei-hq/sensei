# Docs Framework Adoption + Front-Door Exemplar — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Adopt the operating-model canonical doc structure + per-feature dossier, make stages×audiences explicit in `docs/README.md`, and write `docs/features/front-door/` as the first complete, validatable feature spec (encoding the current partial implementation state + the intake model gaps).

**Architecture:** A docs-only effort. Use the shipped, idempotent `sensei scaffold` tooling to create the canonical skeletons, then fill/move content. No code, no automated tests — **verification per task is a concrete doc check** (a file exists / reads complete / a grep is clean).

**Tech Stack:** Markdown; the `sensei` CLI (`scaffold`, `scaffold feature`); `git mv`; `grep`/`ls` for verification.

**Design:** `docs/plan/2026-07-20-docs-framework-adoption-design.md`.

**Conventions:**
- **GIT HYGIENE:** the pre-commit hook stages broadly — always `git status` then explicit `git add <paths>`; per-task commit to `develop` (approach A); leave a clean tree. `--no-verify` only if the hook fails for a reason unrelated to the change (docs commits shouldn't trip it).
- **No content invention:** the front-door dossier *consolidates* existing sources (cited per task) — do not re-derive facts; pull them from the named files.
- Voice: lowercase "sensei", no emoji/marketing, sentence case (per `docs/architecture/frontend-svelte-guidelines.md` §4 / the repo's doc voice).

---

### Task 1: Project scaffold + rewrite `docs/README.md` as the stages × audiences map

**Files:**
- Run: `sensei scaffold` (creates `docs/{vision,personas,roadmap,design,features}` + `docs/features/README.md` + `docs/decisions.md`; idempotent — leaves existing `journeys/mockups/architecture` untouched)
- Modify: `docs/README.md` (replace the "six folders" map)

- [ ] **Step 1: Run the project scaffold.**
  Run: `cd /Users/Jerry/Developer/sensei-hq/sensei && sensei scaffold`
  Expected: a `[created]/[exists]` report; new **`docs/vision.md`** (a single FILE — the canonical vision doc), dirs `docs/personas/`, `docs/roadmap/`, `docs/design/`, `docs/features/` (+ `docs/features/README.md`), `docs/decisions.md`; existing folders reported `[exists]`. (Empty dirs personas/roadmap/design won't show in `git status` until they get files in Task 3.)
  Verify: `ls docs/vision.md && ls -d docs/personas docs/roadmap docs/design docs/features` all exist.
  **NOTE:** the canonical "vision tier" is top-level FILES — `docs/vision.md` (why) + `docs/objectives.md` (measurable objectives, added in Task 2) — NOT a `docs/vision/` folder. The scaffold defines vision as a single file; adapt to it.

- [ ] **Step 2: Inspect what scaffold created** so you don't overwrite its READMEs blindly: `cat docs/features/README.md docs/decisions.md`. If scaffold created stub READMEs in `vision/personas/roadmap/design`, keep them; you will fill `vision/` and `roadmap/` in Tasks 2–3.

- [ ] **Step 3: Rewrite `docs/README.md`** as the map. It MUST contain, in this order:
  1. A one-paragraph intro (sensei = the OS for AI-assisted work; observe-learn-improve; north-star FTR).
  2. A **stages × audiences table** with exactly these rows (Purpose + Primary audience + note), matching the design's Section 1:
     `vision.md` + `objectives.md` (why + measurable objectives; PO+all — top-level files), `personas/` (who we serve; PO+designer), `journeys/` (end-to-end flows; PO+designer+dev), `roadmap/` (phases+status; PO), `design/` (design system + cross-cutting UX; designer+dev), `mockups/` (system-wide mockup bundle; designer+dev), **`features/<name>/` (the complete per-feature spec — the source of truth; PO validates + agent grounds + dev builds)**, `architecture/` (technical how, per surface; dev+agent), `spec/` (legacy per-screen build specs — **transitional**, folding into `features/*/design.md` or `architecture/` as touched), `plan/*` (dated **transient** build plans).
  3. A **reading paths** subsection: **PO** → `vision.md` + `objectives.md` → `features/<name>/{brief,design}` → `journeys/`; **Designer** → `personas/` → `journeys/` → `mockups/` + `design/` → `features/<name>/mockup-ref.md`; **Dev/agent** → `features/<name>/` → `architecture/` → `plan/*`.
  4. A **source-of-truth declaration**: "A feature's truth lives in its `features/<name>/` dossier. `plan/operating-model.md` is the strategy (why + system); `plan/*` dated docs are transient build detail; `spec/` is transitional."
  5. Keep the existing "Monorepo structure" table (below the fold) — don't delete it.
  Preserve the mermaid-flow idea but update nodes to the new stages (or drop it — do not leave the OLD six-folder mermaid).

- [ ] **Step 4: Verify.** `grep -c "features/<name>/" docs/README.md` ≥ 1; `grep -iE "audience|reading path|source of truth|transient" docs/README.md` returns hits; the old `requirements/ → journeys/ → ... → plan/` six-folder mermaid is gone (`grep -n "1 · requirements" docs/README.md` → empty).

- [ ] **Step 5: Commit.**
  ```bash
  git add docs/README.md docs/vision docs/personas docs/roadmap docs/design docs/features docs/decisions.md
  git commit -m "docs: adopt canonical doc structure + rewrite README as the stages×audiences map"
  ```
  (Only add the paths scaffold created + the README. `git status` clean after.)

---

### Task 2: Move vision + objectives to top-level `docs/vision.md` + `docs/objectives.md`; fix inbound links

**Files:**
- Replace/move: `docs/requirements/vision.md` content → `docs/vision.md` (overwrite the scaffold stub); `git mv docs/requirements/objectives.md` → `docs/objectives.md` (top-level FILES, not a `vision/` folder)
- Modify (link fixes): `docs/plan/README.md`, `docs/plan/operating-model.md`, `docs/plan/decisions.md`, `docs/spec/pipeline/insights.md`, `docs/spec/screen/dojo-developer-console.md`, `docs/spec/screen/relay-security.md`, `docs/spec/screen/relay-respond.md`, `docs/spec/screen/relay-task-detail.md`

- [ ] **Step 1: Move the content into the top-level files.** The scaffold made a stub `docs/vision.md`; replace it with the real vision content and move objectives up:
  ```bash
  # vision: real content replaces the stub (git mv can't overwrite → rm stub first)
  rm docs/vision.md
  git mv docs/requirements/vision.md docs/vision.md
  # objectives: a sibling top-level file
  git mv docs/requirements/objectives.md docs/objectives.md
  ```
  (If `docs/vision.md` was already committed as the stub in Task 1, use `git rm docs/vision.md` instead of `rm`, then `git mv`. Confirm the resulting `docs/vision.md` is the 193-line requirements content, not the stub.)

- [ ] **Step 2: Find every inbound link** (both `requirements/vision` and `requirements/objectives`):
  `grep -rn "requirements/vision\|requirements/objectives" docs/`
  Expected references in the files listed above (the design doc + this plan legitimately *describe* the move — leave their "current → target" mentions alone).

- [ ] **Step 3: Rewrite the links.** In each referencing file, replace `requirements/vision.md` → the correct relative path to `docs/vision.md` and `requirements/objectives.md` → `docs/objectives.md`. Mind relative depth: from `docs/plan/*` it's `../vision.md` / `../objectives.md`; from `docs/spec/screen/*` and `docs/spec/pipeline/*` it's `../../vision.md` / `../../objectives.md`. Edit each explicitly (do not blanket-sed across differing depths).

- [ ] **Step 4: (no vision/ README needed — vision is a top-level file.)** Ensure `docs/vision.md` opens with a clear title + a one-line link to `objectives.md` ("Measurable objectives per segment → [objectives.md](objectives.md)"), so the two read as the vision tier.

- [ ] **Step 5: Verify.** `grep -rn "requirements/vision\|requirements/objectives" docs/ | grep -vE "2026-07-20-docs-framework-adoption-(design|plan)"` → **empty** (no stale links). `ls docs/vision.md docs/objectives.md` → both exist. `ls docs/requirements/vision.md 2>&1` → "No such file". `wc -l docs/vision.md` → ~193 (real content, not the ~20-line stub).

- [ ] **Step 6: Commit.**
  ```bash
  git add docs/vision.md docs/objectives.md docs/requirements/vision.md docs/requirements/objectives.md docs/plan/README.md docs/plan/operating-model.md docs/plan/decisions.md docs/spec/pipeline/insights.md docs/spec/screen/dojo-developer-console.md docs/spec/screen/relay-security.md docs/spec/screen/relay-respond.md docs/spec/screen/relay-task-detail.md
  git commit -m "docs: move vision + objectives to top-level vision.md/objectives.md; fix inbound links"
  ```
  (`git add` the moved-from paths too so the deletions are staged.)

---

### Task 3: Seed `roadmap/`; stub `personas/` + `design/`

**Files:**
- Create/modify: `docs/roadmap/README.md`, `docs/personas/README.md`, `docs/design/README.md`

- [ ] **Step 1: Write `docs/roadmap/README.md`** — do NOT copy the whole plans; write a short roadmap index that (a) states purpose + audience ("phases, sequencing, and current status; audience: product owner"), (b) summarizes the current phase status in 5–8 bullets distilled from `docs/plan/README.md` (the living gap-analysis: surfaces built ✅, learning loop closing, FTR loop) and `docs/spec/EXECUTION-PLAN.md`, and (c) links to both as the detailed sources. Mark it "living — updated as workstreams land."

- [ ] **Step 2: Write `docs/design/README.md`** — purpose + audience ("the design system + cross-cutting UX rules; audience: designer + developer"), linking to the canonical `../architecture/frontend-svelte-guidelines.md` (the 24 tokens, type scale, spacing) and `../mockups/STYLING.md`. Note: this folder points at those until design-system content is consolidated here (incremental).

- [ ] **Step 3: Write `docs/personas/README.md`** — a short stub: purpose + audience ("who sensei serves and their goals; audience: product owner + designer") and a one-line note "to be filled — the current audience segments are described in `../objectives.md` (per-segment) meanwhile." (Do not invent personas.)

- [ ] **Step 4: Verify.** Each of the three READMEs exists and names its audience: `grep -il "audience" docs/roadmap/README.md docs/design/README.md docs/personas/README.md` lists all three. `docs/roadmap/README.md` links to both `plan/README.md` and `spec/EXECUTION-PLAN.md`.

- [ ] **Step 5: Commit.**
  ```bash
  git add docs/roadmap/README.md docs/personas/README.md docs/design/README.md
  git commit -m "docs: seed roadmap index + stub personas/design stages"
  ```

---

### Task 4: Scaffold the front-door dossier + write `brief.md`

**Files:**
- Run: `sensei scaffold feature front-door` (creates `docs/features/front-door/{brief.md,design.md,plan.md,tests/,decisions.md,mockup-ref.md}`)
- Modify: `docs/features/front-door/brief.md`

- [ ] **Step 1: Scaffold the dossier.**
  Run: `sensei scaffold feature front-door`
  Verify: `ls docs/features/front-door/` shows `brief.md design.md plan.md tests decisions.md mockup-ref.md`.

- [ ] **Step 2: Write `brief.md`** (Intent — the goal + the axes it's built on; NOT layout). Source: `docs/requirements/front-door.md` §1–§2 + `operating-model.md` §3.3 + `crates/senseid/src/playbook.rs` (the enums). Content:
  - **Purpose:** the front door is sensei's adaptive-process entry — at the start of a work chunk it classifies the chunk on three axes and **recommends a playbook (recommend-and-confirm)**, so rigor is proportional to risk. It is the "start here" surface.
  - **The three axes** (the "three tiers"), each with values + **how each is inferred**:
    - `lifecycle`: `greenfield` | `stable` — inferred from spine/drift (existing code+docs → stable; empty/new → greenfield).
    - `intent`: `explore` | `ux` | `feature` | `enhancement` | `bug` — the goal of the chunk.
    - `risk`: `low` | `high` — blast-radius from the code graph (callers/community reach).
  - **Audiences of this feature:** the individual developer (runs intake), product owner (validates the process), agent (executes the recommended playbook).
  - A one-line pointer: "Full behavior + surfaces → `design.md`; what works today → `tests/acceptance.md`."

- [ ] **Step 3: Verify.** `grep -iE "greenfield|explore|ux|feature|enhancement|bug|low|high|recommend-and-confirm" docs/features/front-door/brief.md` returns the axis values + the interaction contract. Reads as one coherent "what is this + why" without needing another file.

- [ ] **Step 4: Commit.**
  ```bash
  git add docs/features/front-door/brief.md docs/features/front-door/design.md docs/features/front-door/plan.md docs/features/front-door/decisions.md docs/features/front-door/mockup-ref.md docs/features/front-door/tests
  git commit -m "docs(features): scaffold front-door dossier + write brief (intent + the three axes)"
  ```

---

### Task 5: Write the front-door `design.md` (behavior, surfaces, rule matrix, data)

**Files:**
- Modify: `docs/features/front-door/design.md`

Sources to consolidate (do not re-derive): `docs/requirements/front-door.md` (surfaces S1–S5, journeys, states), `operating-model.md` §3.3 + §9, `docs/plan/2026-07-19-frontdoor-intake-design.md`, `docs/plan/2026-07-19-app-intake-form-design.md`, `docs/plan/2026-07-19-learning-loop-design.md`, `docs/plan/2026-07-19-auto-select-on-trust-design.md`, and the seed data `database/import/staging/{playbooks,playbook_rules}.jsonl`.

- [ ] **Step 1: Write `design.md`** with these sections:
  1. **Intake flow** — freeform description → classify (gateway `reasoning` + heuristic fallback) → recommend → **recommend-and-confirm** (high-risk always human; low-risk may auto-select once trusted). The five UI states: describe / loading / recommended / recorded / error.
  2. **Two surfaces:**
     - **CLI / agent** — `/sensei:intake` (conversational; the primary in-session surface).
     - **App** — the `/intake` Observatory screen (structured twin; freeform → classify → recommend → confirm-persist; **session-less by design** — advisory, not FTR-attributed).
  3. **Playbook catalog** — the 6 playbooks (name → title → when → opening tone) from `playbooks.jsonl`: `vibe`, `mockup_first`, `spec_driven`, `gsd`, `change_flow`, `debug_flow`.
  4. **Axes → playbook rule matrix** — render as a table from `playbook_rules.jsonl`, and **mark uncovered cells** (this makes the coverage legible + surfaces the gap):

     | # | lifecycle | intent | risk | → playbook | priority |
     |---|---|---|---|---|---|
     | 1 | * | * | high | `spec_driven` | 100 |
     | 2 | greenfield | explore | * | `vibe` | 60 |
     | 3 | greenfield | ux | * | `mockup_first` | 60 |
     | 4 | stable | bug | * | `debug_flow` | 60 |
     | 5 | stable | enhancement | * | `change_flow` | 50 |
     | 6 | * | feature | low | `gsd` | 40 |
     | — | *(no match)* | | | **default → `gsd`** | — |

     Add a one-line note: cells not covered by rules 1–6 fall through to the default (`gsd`) — e.g. `stable+ux+low`, `greenfield+feature+low`, `greenfield+enhancement+*`. (Full analysis lives in `tests/acceptance.md` "known gaps".)
  5. **Learning loop + auto-select** — FTR attribution (`playbook_run.outcome_ftr` from session FTR via the analyzer `LearnPlaybooks` global pass) re-weights rules (bounded, off `base_priority` vs target FTR) + proposes `source='learned'` rules; **auto-select-on-trust** (low-risk + n≥10 + ftr≥0.8 → auto-confirm + announce, reversible).
  6. **Data contract** — `GET /api/playbook/guide` → `{frame, axes[], playbooks[]}`; `POST /api/playbook/recommend` → `{playbook, rationale, lifecycle, intent, risk, opening_tone, when_to_use, auto_select, trust{n,ftr}}` (+ `preview` flag skips persistence; `confirm` records one run). Note the session-less app path.
  7. **Depth note:** this is shipped-but-partial; the built/partial/gap breakdown is in `tests/acceptance.md`.

- [ ] **Step 2: Verify.** The rule-matrix table renders (6 rows + default). `grep -iE "vibe|mockup_first|spec_driven|gsd|change_flow|debug_flow" docs/features/front-door/design.md` → all 6. `grep -iE "/sensei:intake|/intake|/api/playbook/(guide|recommend)|auto_select|preview" docs/features/front-door/design.md` returns the surfaces + endpoints. A reader can follow flow → surfaces → catalog → matrix → data without opening `operating-model.md` or `plan/*`.

- [ ] **Step 3: Commit.**
  ```bash
  git add docs/features/front-door/design.md
  git commit -m "docs(features): front-door design — flow, both surfaces, rule matrix, learning/auto-select, data contract"
  ```

---

### Task 6: Write `tests/acceptance.md` (partial state + gaps) + `decisions.md` + `mockup-ref.md`

**Files:**
- Create/modify: `docs/features/front-door/tests/acceptance.md`
- Modify: `docs/features/front-door/decisions.md`, `docs/features/front-door/mockup-ref.md`

- [ ] **Step 1: Write `tests/acceptance.md`** — the checklist that encodes the **current partial state**, in three groups:
  - **Built + verified** (checkbox ✅): recommend/preview/confirm round-trip on both surfaces; the app `/intake` route + rail anchor + the freeform→recommend→confirm→recorded flow (e2e `app/e2e/tests/intake.spec.ts`, 4/4); `preview` writes no row, `confirm` writes exactly one; classified axes returned in the response.
  - **Shipped, not released** (checkbox ☑, note "on `develop`, post-`v0.6.0`"): the §9 learning loop (attribution + reweight + proposals + accept path); auto-select-on-trust; the nudge hook (activated on `develop`).
  - **Known gaps / open** (checkbox ☐ — RECORDED, not fixed here; each with a one-line "fix = separate chunk"):
    - **Intent taxonomy is code-only** — no "documentation / product-definition" intent; a docs/vision chunk is forced to `ux`. (Dogfood: this docs effort classified `stable/ux/low`.)
    - **Rule-matrix holes** — combos not covered by rules 1–6 silently default to `gsd` (e.g. `stable+ux+low`, `greenfield+feature+low`, `greenfield+enhancement+*`); no explicit rule + no "defaulted" signal to the user beyond `rationale: "no rule matched"`.
    - **App form is session-less** → not FTR-attributed, so app-initiated intakes don't feed the learning loop.
  - A short "how to verify now" note: `curl :7744/api/playbook/{guide,recommend}` + `make test-app-e2e` (intake spec).

- [ ] **Step 2: Write `decisions.md`** (append-only) — carry forward the decisions already made, one line each with the "why": persist-on-confirm (app form records the run); session-less app form (advisory; CLI/agent path feeds §9); auto-select thresholds (Low + n≥10 + ftr≥0.8); reweight target FTR = 0.5. Add the open decisions: whether to add a documentation/definition intent; whether to fill the rule-matrix holes or keep the `gsd` default.

- [ ] **Step 3: Write `mockup-ref.md`** — state that the designer's front-door screens (intake / playbook catalog / recommendation card) are **pending** (the three-tiers/axes approach was passed to the designer 2026-07-20); until they land in `docs/mockups/Sensei/lib/` + `docs/spec/MOCKUP-INDEX.md`, this references the surface requirements in `design.md` S1–S3. Leave a clear "update this when screens land" line.

- [ ] **Step 4: Verify.** `tests/acceptance.md` has the three groups + names the two model gaps. `grep -iE "documentation|definition|stable\+ux\+low|no rule matched|session-less" docs/features/front-door/tests/acceptance.md` returns the gaps. `decisions.md` lists ≥4 decisions with a why.

- [ ] **Step 5: Commit.**
  ```bash
  git add docs/features/front-door/tests docs/features/front-door/decisions.md docs/features/front-door/mockup-ref.md
  git commit -m "docs(features): front-door acceptance (partial-state + known gaps), decisions, mockup-ref"
  ```

---

### Task 7: Supersede `requirements/front-door.md`; retire `requirements/` to pointers; migration policy

**Files:**
- Modify/remove: `docs/requirements/front-door.md`, `docs/requirements/README.md`
- Modify: `docs/README.md` (add a short migration-policy section)

- [ ] **Step 1: Supersede `requirements/front-door.md`.** Its content now lives in the dossier. Confirm nothing unique was lost: `diff`-read it against `features/front-door/{brief,design}.md` mentally — anything present in `requirements/front-door.md` but missing from the dossier (e.g. the designer-facing S4/S5 history + learning-review screens, the deferred/open-questions) must be folded into `design.md` (surfaces) or `tests/acceptance.md` (open) FIRST. Then replace `requirements/front-door.md` with a one-line pointer: `> Moved. The front-door feature spec now lives in [\`../features/front-door/\`](../features/front-door/README.md) (or brief.md). This file is retained as a redirect.` (If `features/front-door/` has no `README.md`, point at `brief.md`.)

- [ ] **Step 2: Retire `requirements/README.md` to a pointer** — since `vision.md`/`objectives.md` moved to `vision/` and `front-door.md` moved to `features/front-door/`, replace the body with: purpose is superseded → see the top-level map (`../README.md`); vision + objectives now at top level (`../vision.md`, `../objectives.md`); feature specs in `../features/`. (Keep the folder as a redirect rather than deleting, per the incremental policy.)

- [ ] **Step 3: Add a "Migration policy" section to `docs/README.md`** (short): the canonical structure is adopted; migration is **feature-by-feature as each is touched**; `requirements/` is fully absorbed (vision/objectives → top-level `vision.md`/`objectives.md`, front-door → `features/front-door/`) and left as redirects; `spec/` folds into `features/*/design.md` or `architecture/` as each screen is next worked; dated `plan/*` are transient. Nothing is deleted until its content is migrated; history is in git.

- [ ] **Step 4: Verify.** `cat docs/requirements/front-door.md` is a one-line redirect (not the full doc). `grep -in "migration policy" docs/README.md` → hit. No doc links to `requirements/front-door.md` for *content* anymore: `grep -rn "requirements/front-door" docs/ | grep -v "redirect\|2026-07-20-docs-framework"` → empty (or only the redirect file itself).

- [ ] **Step 5: Commit.**
  ```bash
  git add docs/requirements/front-door.md docs/requirements/README.md docs/README.md
  git commit -m "docs: supersede requirements/front-door → features dossier; retire requirements/ to redirects; migration policy"
  ```

---

## Final verification (whole plan)

- [ ] **Structure:** `ls docs/vision.md docs/objectives.md` (files) + `ls -d docs/{personas,journeys,roadmap,design,mockups,features,architecture}` all exist; `docs/features/front-door/` has `brief.md design.md plan.md tests/acceptance.md decisions.md mockup-ref.md`.
- [ ] **Map:** `docs/README.md` names every stage's audience + reading paths + source-of-truth + migration policy; the old six-folder mermaid is gone.
- [ ] **No stale links:** `grep -rn "requirements/vision\|requirements/objectives\|requirements/front-door" docs/ | grep -v "redirect\|2026-07-20-docs-framework"` → empty.
- [ ] **PO check (read-aloud):** open only `docs/features/front-door/` — can you state what the front door does, the three axes, both surfaces, which playbook a `stable+bug+low` chunk gets, and what's built vs not? (Yes = the dossier is self-complete.)
- [ ] **Agent-grounding check:** the front-door dossier + `docs/README.md` answer "how does the front door work + what's built" with no reference to `operating-model.md` or `plan/*`.
- [ ] **Gaps recorded:** `tests/acceptance.md` names the intent-taxonomy gap + the rule-matrix hole.
- [ ] **Whole-plan review** (subagent): structure coherence, no stale links, the dossier is genuinely self-contained, voice consistent.

## Self-review notes (author)

- **Spec coverage:** map + stages×audiences ✓ T1; vision move ✓ T2; roadmap/personas/design ✓ T3; dossier + brief ✓ T4; design (flow/surfaces/matrix/learning/data) ✓ T5; acceptance (partial+gaps) + decisions + mockup-ref ✓ T6; supersede requirements + migration policy ✓ T7.
- **No automated tests** (docs) — each task's verification is a `grep`/`ls`/read check; the whole-plan PO + agent-grounding checks are the real acceptance.
- **Incremental:** only `requirements/` is fully migrated; `spec/`, `plan/*`, other features are left in place with the policy documenting how they fold in later. No deletions (redirects only).
- **Link-depth care (T2):** relative paths differ by folder depth (`../vision.md` from `plan/`, `../../vision.md` from `spec/screen/`) — edit per file, no blanket sed.
