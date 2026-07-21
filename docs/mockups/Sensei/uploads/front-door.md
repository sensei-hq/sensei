# Front door & adaptive playbooks — product requirements

> **Audience:** a designer (human or LLM) producing/updating screens.
> **The *why* and *what*, not the *how*.** The reframed vision lives in
> [`../plan/operating-model.md`](../plan/operating-model.md); the build specs in
> [`../plan/2026-07-19-frontdoor-intake-design.md`](../plan/2026-07-19-frontdoor-intake-design.md)
> and the learning-loop / auto-select designs alongside it. This doc is the
> bridge: the surfaces these concepts require, and what each screen must show.
>
> **What to produce:** see [§9 Deliverables](#9-deliverables-requested). Generate
> against the design system (see [§8](#8-design-system-constraints)); drop mockups
> into `docs/mockups/Sensei/lib/` and register them in `docs/spec/MOCKUP-INDEX.md`.

---

## 1. The reframe in one page

Sensei was a **retrospective** loop — it watched a pairing (you + your assistant)
and surfaced insights *after* the fact (memories, patterns, FTR). The reframe
adds a **prospective** loop: sensei now also helps at the **start** of a chunk of
work, not just the review of it.

The organizing idea: **sensei is the OS for AI-assisted work — it drives, the
human brings intent.** The discipline that used to need a whole team (analyst,
architect, reviewer) is carried by sensei's roles, so one person gets a team's
rigor at ~zero overhead. Two things follow:

- **Adaptive process.** Not every chunk deserves the same ceremony. A one-line
  bug fix and a high-blast-radius rewrite need different rigor. Sensei reads the
  chunk and **recommends a playbook** — a way of working — proportional to the
  risk. Depth follows risk; the human is never made to over-plan a trivial task
  or under-plan a dangerous one.
- **The front door.** Every chunk of work starts with a short **intake**: sensei
  classifies the chunk on three axes, recommends a playbook, and confirms with
  the human before work begins. This is the new "start here" surface.

This document is about that front door and the playbook system around it.

---

## 2. Core vocabulary

| Term | Meaning |
|---|---|
| **Chunk** | One unit of work a developer is about to start (a bug fix, a feature, a spike). The atom the front door operates on. |
| **Axes** | Three dimensions sensei classifies a chunk on (below). Together they select a playbook. |
| **Playbook** | A *way of working* for a chunk — the recommended method (e.g. debug-flow, spec-driven). Six today; the catalog is runtime-extensible. |
| **Intake** | The short front-door interaction: describe the chunk → sensei classifies + recommends → human confirms. |
| **Recommend-and-confirm** | The interaction contract: sensei proposes a playbook with a one-line reason; the human accepts or overrides. Never silent for risky work. |
| **Auto-select-on-trust** | For **low-risk** chunks whose recommendation has a proven track record, sensei skips the confirm, records it, and announces — reversible. High-risk always keeps the human in the loop. |
| **Learning loop** | Sensei attributes each recorded chunk's outcome (did the first turn land?) back to the playbook, and over time re-weights recommendations + proposes new rules. |
| **FTR** | First-turn resolution — the fraction of sessions whose first attempt landed without a correction. The quality signal the learning loop rides. |

### The three axes (exact values)

| Axis | Values | The question the user answers |
|---|---|---|
| **Lifecycle** | `greenfield` · `stable` | "Is this a new effort, or a change to a stable product?" |
| **Intent** | `explore` · `ux` · `feature` · `enhancement` · `bug` | "What's the goal — explore, a UX-heavy surface, a new feature, an enhancement, or a bug fix?" |
| **Risk** | `low` · `high` | "How much does this touch — is the blast-radius high?" |

Sensei **infers** these where it can (from the code graph, docs, and drift) and
only asks what it can't infer. The design should treat the axes as *sensei's
reading of the chunk, shown back to the user for a sanity check* — not a form the
user is forced to fill from scratch.

---

## 3. The playbook catalog

Six playbooks today. Each has a short `when_to_use` and an `opening_tone` (the
posture sensei adopts once it's chosen). The design should be able to render this
as a browsable catalog and as the label/description on a recommendation.

| Playbook | Title | When it's recommended | Opening tone |
|---|---|---|---|
| `vibe` | Vibe / spike | Greenfield, objective fuzzy — explore then extract learnings (discardable) | "Explore fast and loose; capture what you learn, keep nothing you cannot justify." |
| `mockup_first` | Mockup-first | Greenfield, UX-heavy — design the surface before the spec | "Start from the mockup; let the UI shape the spec." |
| `spec_driven` | Spec-driven | Clear objective + high blast-radius — force a deep design first | "Slow down: write the design, enumerate edge cases, before any code." |
| `gsd` | Get stuff done | Known feature, low risk — lean plan then build | "Lean plan, then build; keep it tight." |
| `change_flow` | Change-flow | Stable product enhancement — impact analysis then targeted design | "Map impact first; design the smallest change that lands the value." |
| `debug_flow` | Debug-flow | Stable product bug — reproduce, fix, add a regression test | "Reproduce first; fix; lock it with a regression test." |

---

## 4. Surfaces & screens

Two surfaces run the front door. The design work is almost entirely the **app**
surface — the CLI already exists and is text-only.

- **CLI / agent surface (exists, no design needed):** `/sensei:intake` — a
  conversational intake inside the coding assistant. Sensei asks only what it
  can't infer, recommends, and confirms in prose. This is the *primary* surface
  during actual coding.
- **Sensei app surface (the design work):** the desktop app's structured twin of
  that conversation — a screen where the user describes a chunk and sees the
  recommendation as a card. Lives in the **Observatory** (the per-user home), as
  the **front-door anchor** (first item in the rail).

Screens the designer should produce or update, by priority:

### S1 — Intake screen (P0, new, no mockup exists)

The core deliverable. Route `/intake`, reached from the "Intake" rail anchor.

**Goal:** the user is about to start a chunk; they want sensei's read + a method.

**Flow & elements:**
1. **Frame / intro** — a one-line grounding prompt above the input (the intake
   "frame": *"Describe the chunk's lifecycle, intent, and risk…"* — reworded for
   an end user, e.g. "Describe the work you're about to start.").
2. **Freeform input** — a single multi-line text box. The user types a plain
   description ("fix the crash when the token refreshes"). No axis pickers up
   front — sensei classifies.
3. **Recommend action** — a primary button ("Recommend a playbook"); shows a
   working/loading state while sensei classifies.
4. **Recommendation card** — see S3, rendered inline once sensei responds.
5. **Confirm** — a primary action on the card ("Use this playbook") that records
   the choice; the card then shows a **recorded** confirmation + a way to start a
   new intake.

**States the screen must express** (these are real — the app implements them):
`describe` (empty, ready) · `loading` (classifying) · `recommended` (card shown,
awaiting confirm) · `recorded` (confirmed + saved) · `error` (classification/record
failed — show the message, let them retry).

### S2 — Playbook catalog (P1)

A browsable view of the six playbooks (title, when-to-use, opening tone). Purpose:
let a user understand the methods sensei can pick, and see which one they're
getting. Could be its own reference screen, a panel on the intake screen, or a
section in settings — designer's call. Should scale gracefully as the catalog
grows (it's runtime-extensible).

### S3 — Recommendation card (P0 component, part of S1)

The recommend-and-confirm unit. Must show:

- **Playbook title** (from the catalog) + the one-line **rationale** (why this
  chunk got this playbook).
- **The inferred axes** — three chips: lifecycle, intent, risk — so the user sees
  *what sensei read*. This is the sanity-check surface; consider making it obvious
  these are sensei's inference (and, as a future enhancement, correctable).
- **Opening tone** — the posture line, shown as a quiet secondary line.
- **Trust / auto-select badge** — when the recommendation is trusted (see S1
  auto-select), a badge: "trusted — FTR 0.9 over 12 runs." When auto-selected,
  the card is already in the recorded state and says so.
- **Confirm affordance** — "Use this playbook." For **high-risk** chunks the
  confirm must be explicit (never auto). For a mis-read, a way to change/override
  (future: pick a different playbook / edit an axis).

### S4 — Intake / run history (P2, future)

Recorded chunks over time: what was started, which playbook, and — once the
learning loop attributes it — how it turned out (FTR). A per-user log that feeds
the sense that sensei is learning your patterns. Not built yet; mock it so we can
see where it belongs (likely a tab on the intake screen or under the project).

### S5 — Playbook learning review (P2)

Where a human reviews what the learning loop produced: per-combo FTR stats, and
**proposed new rules** (sensei suggests "for stable+enhancement+low, prefer
change_flow" based on outcomes) to accept or reject. This is a governance-flavored
surface — it may belong in **Dōjō** (the team/org control plane) more than the
individual Sensei app. Flag both options in the mockup; note the open question in
[§10](#10-open-questions).

---

## 5. User journeys (concrete)

**J1 — Stable bug fix (recommend-and-confirm).** Dev opens Intake, types "fix the
null deref when the session token refreshes." Sensei reads it: lifecycle=stable,
intent=bug, risk=low → recommends **Debug-flow** ("Reproduce first; fix; lock it
with a regression test"). Dev glances at the axis chips, agrees, clicks **Use this
playbook**. Recorded. Dev goes to their assistant and works in debug-flow.

**J2 — Greenfield UX spike (design-first).** Dev types "prototype a new onboarding
wizard, still figuring out the shape." Sensei: greenfield, ux, low →
**Mockup-first** ("Start from the mockup; let the UI shape the spec"). Dev
confirms.

**J3 — High-blast-radius change (forced rigor).** Dev types "rename the session
store used across the app." Sensei: stable, enhancement, **high** → **Spec-driven**
("Slow down: write the design, enumerate edge cases, before any code"). Because
risk is high, sensei **requires an explicit confirm** — no auto-select, ever.

**J4 — Trusted routine chunk (auto-select).** Dev types another small bug fix.
Debug-flow has landed cleanly 12 times for this kind of chunk (FTR 0.9), so sensei
**auto-selects**, records it, and announces: "Auto-selected Debug-flow — reliable
for this kind of chunk (FTR 0.9 over 12 runs). Say 'change' to pick another." The
card opens already in the recorded state with the trust badge.

---

## 6. States & rules the design must honor

- **Depth follows risk.** Low-risk → light touch, auto-select eligible. High-risk
  → always explicit confirm, always human-in-the-loop. The visual weight of the
  confirm should reflect this (a high-risk recommendation should *feel* like it
  wants a decision).
- **Sensei's read is visible and challengeable.** The axes are shown so the user
  can catch a mis-read. Auto-select is announced and reversible, never silent.
- **Graceful empty/quiet states.** A fresh install has no trust history → no
  auto-select, plain recommend-and-confirm. The catalog is never empty (six
  built-ins). A daemon hiccup degrades to a calm state, never a broken screen.
- **No dead ends.** After recording, there's always a clear next step (new intake;
  or go work). Errors are actionable (retry).

---

## 7. Data the screens read (contract, for reference)

The screens are driven by two daemon endpoints (already built). Field names the
design copy can rely on:

- **Guide** (`GET /api/playbook/guide`): `frame` (the intro line), `axes[]` (the
  per-axis prompt + help), `playbooks[]` (`name`, `title`, `when_to_use`,
  `opening_tone`).
- **Recommendation** (`POST /api/playbook/recommend`): `playbook` (name),
  `rationale`, the inferred `lifecycle` / `intent` / `risk`, `opening_tone`,
  `auto_select` (bool), `trust` (`{ n, ftr }`).

The design doesn't need to implement these — it just needs to know every piece of
information a screen can show is one of the above.

---

## 8. Design-system constraints

Generate against the existing system — no hand-rolled primitives:

- **Tokens & scale:** `docs/architecture/frontend-svelte-guidelines.md` (the 24
  canonical tokens; the Zen-Sumi 4px spacing scale). Colors/surfaces use the
  `paper`/`ink`/`accent`/`success`/`warning`/`danger` families (e.g. cards =
  `bg-paper-soft` + `border-paper-edge`); never invent `bg-surface`-style tokens.
- **Existing screens to match:** the Observatory (`docs/mockups/Sensei/lib/observatory.jsx`)
  and its cards — the intake screen is a new sibling in that rail and should read
  as part of it.
- **Mockup conventions:** `docs/mockups/Sensei/MOCKUP-INDEX.md` and `docs/mockups/STYLING.md`.
- **Delivery:** system-wide, repo-resident bundle — drop the generated mockups
  into `docs/mockups/Sensei/lib/` and register each in `docs/spec/MOCKUP-INDEX.md`
  (per the operating-model design subsystem — not claude.ai/design on the fly).

---

## 9. Deliverables requested

From the designer, in priority order:

1. **P0 — Intake screen** (S1) with the **recommendation card** (S3), covering all
   five states (describe / loading / recommended / recorded / error) and both the
   recommend-and-confirm and auto-selected variants, including the high-risk
   explicit-confirm treatment.
2. **P1 — Playbook catalog** (S2) — a browse of the six methods.
3. **P2 — Intake/run history** (S4) and **learning review** (S5) — lower fidelity;
   enough to place them and settle where S5 lives (Sensei vs Dōjō).

Each mockup: consistent with the Observatory chrome, responsive, and registered in
the mockup index.

---

## 10. Open questions (for the designer + Jerry)

- **Where does S5 (learning review) live** — the individual Sensei app, or Dōjō
  (team/org governance)? Leaning Dōjō for accept/reject of learned rules; Sensei
  for "your own" per-combo stats. Mock both placements if cheap.
- **Axis correction:** v1 shows the inferred axes read-only. Should the card let
  the user *override* an axis before confirming (which would re-recommend)? Design
  the affordance even if the build defers it.
- **Catalog placement:** standalone reference screen vs a panel on intake vs
  settings. Designer's recommendation welcome.

## 11. Out of scope

The learning-loop internals (attribution math, rule re-weighting), the governance
resolution model, and the daemon/DDL — all covered in the plan docs. This doc is
only the surfaces and what they show.
