# Sensei / Dōjō — design brief & review (single source)

> **This is the one design doc.** It collapses the prior review files
> (`DOJO-REVIEW.md`, `DOJO2-MISSING-CONSOLES.md`, `DOJO-REDESIGN-BRIEF.md`,
> `SPACING-CLEANUP.md`) into one working surface. We iterate **section by section**:
> a section here becomes an **actionable task handed to the designer LLM**, which
> produces the mockup changes; the result is dropped back into `docs/mockups/`.
> The aim is a comprehensive, consistent mockup that translates cleanly to the app.
>
> **Guardrails (Part B) are baked in** — every task we hand off carries them, so the
> designer never re-derives tokens, off-scale sizes, or one-off components.
>
> Canonical companions (do NOT duplicate here — reference them):
> in-app rules = [`../architecture/frontend-svelte-guidelines.md`](../architecture/frontend-svelte-guidelines.md);
> claude.ai/no-rokkit `.zs` drop-in = [`STYLING.md`](STYLING.md);
> screen→source map = [`../spec/MOCKUP-INDEX.md`](../spec/MOCKUP-INDEX.md);
> the design system itself = [`Zen-Sumi Design System/`](./Zen-Sumi%20Design%20System/).

---

## Part A — how we work (the loop)

1. We discuss a **section** of Part D (or a new ask).
2. I turn it into a **designer task** using the template in **Part C** — which prepends the
   guardrails (Part B) + the component vocabulary + the specific ask + the target screen(s).
3. You drop the task into the designer LLM (claude.ai Artifact tool → `.zs` per `STYLING.md`,
   since artifacts can't use rokkit).
4. You drop the produced mockup back into `docs/mockups/…`; we review it against the guardrails
   and mark the section done here.
5. The result is a mockup a build agent can port to Svelte with **zero visual re-derivation** —
   because it already uses the tokens, the scale, and named components.

**Two delivery contexts, one system:** claude.ai mockups use the `.zs` scoped layer (no rokkit,
per `STYLING.md`); the shipped app uses rokkit (`presetRokkit`). **Same tokens, same 8-stop
scale, same component shapes** — only the delivery differs. A mockup built to the guardrails
maps 1:1 to a rokkit component or a custom kit component.

---

## Part B — guardrails (the designer's standing rules — non-negotiable)

Every design task inherits these. They exist to prevent redundancy, drift, and un-portable output.

### B1 · Tokens — named only, never a literal
- Use the **canonical 24 named tokens**, never a hex / `oklch()` / `rgb()` literal, never a
  deprecated numbered/z-scale token.
- **Surface:** `paper` · `paper-soft` · `paper-mute` · `paper-edge`.
  **Ink (text):** `ink` · `ink-soft` · `ink-mute` · `ink-faint`.
  **Brand:** `accent` (朱 vermillion, **rationed**) · `accent-soft` · `primary` (= ink) ·
  `on-primary` (= paper). **Status:** `success`/`-soft` · `warning`/`-soft` · `danger`/`-soft` ·
  `info`/`-soft` · `error`/`-soft`. **Focus/shadow:** `focus-ring` · `shadow-tint`.
- **No token duplication.** If a value exists as a token, use it; do not introduce a parallel
  color/variable for the same intent.
- **Dark mode is automatic** — the tokens flip (`data-theme="dark"` on a wrapper for `.zs`;
  `[data-mode="dark"]` in-app). **Never write a per-mode color in markup.** A pill/card/button
  must read correctly in both modes *by using the tokens* — verify the dark rendering.
  *(Known trap: borders need a real `-soft`/`-edge` token, not the same color as the fill, or
  they vanish in dark mode.)*

### B2 · Typography — fixed 8-stop scale, named families
- Scale (never a literal px): `text-xs` 11 · `text-sm` 13 · `text-base` 15 · `text-lg` 17 ·
  `text-xl` 22 · `text-2xl` 28 · `text-3xl` 40 · `text-4xl` 56.
- Families: display (Fraunces) · body (Inter) · mono (JetBrains) · kanji (Mincho).
  Weights: **light / normal / medium / semibold only.**
- Headers follow the **eyebrow + title** pattern (uppercase tracked eyebrow over a display
  title, optional kanji to the left). **Step section headings down** so the hero/page title
  stands alone — never one size for everything.

### B3 · Color system — semantic, restrained
- Color carries **meaning**, not decoration: `accent` for the brand beat (rationed), status
  tokens for state, ink for text, paper for surface. **Hairlines over shadows; air over density.**
- Status always via the status tokens (`success`/`danger`/`warning`/`info` + their `-soft` fill
  + a border token) — never a raw green/red.

### B4 · Spacing & shape — 4px grid
- `p-*` `px-*` `py-*` `gap-*` `m*-*` on the stops `{1:4, 2:8, 3:12, 4:16, 5:24, 6:32, 8:64}`
  (extended set adds 10:40, 12:48, 16:64). **Never a literal px; never invent a stop** — "if you
  need 18, use 16 or 20."
- Radii from tokens: `rounded-sm` 4 · `rounded` 6 · `rounded-lg` 10 · `rounded-full`.

### B5 · Responsive — mobile-first, breakpoint-consistent
- **Phone is the base** (unprefixed); layer `md:` (768) / `lg:` (1024) to widen. In-app: **never
  `@media` for layout** — use `md:` prefixes and **swap** conflicting utilities. `.zs` mockups
  may use `@media` but **at the app's breakpoints** (`sm` 640 · `md` 768 · `lg` 1024), mobile-first.
- Every screen must stack cleanly on a phone. Wide tables → one card per row on mobile (never a
  sideways-scrolling page).

### B6 · Componentization — map to a component, don't hand-roll
- Design in **named components**, not bespoke markup. Each recurring shape must map to either
  **(a) a Rokkit component** (see the surface in Part C) **or (b) a custom reusable kit
  component** (the dojo2 kit — Part C). If a shape appears **2+ times, it is a component** — call
  it out by name so the build maps it once.
- **Reuse before adding.** Prefer an existing kit/Rokkit component; only propose a **new**
  primitive when nothing fits — and flag it explicitly ("NEW primitive: …") so it enters the kit.
- One empty-state, one section-header, one chip, one row — used everywhere. No per-screen variants
  of the same idea.

### B7 · Data-driven / property-driven components
- Components are **configured by props/data**, not duplicated per case. A row/card/list renders
  from a **shape** (an object) — one `ProjectRow`, `RuleRow`, `MyDojoRow` driven by data, not a
  hand-built variant per instance. Design the **shape** alongside the component.

### B8 · Separation — presentation vs content vs logic
- **Presentation** (the component: props in → markup out, purely visual),
  **content** (the data/fixture shape it renders),
  **logic** (derivations, selection, status-mapping, actions) — kept **separate**.
- In the mockup this means: a screen = a composition of presentational components fed by a data
  shape; any computed value (counts, tallies, resolution) is a pure function over the data, not
  baked into the markup. This is what lets the build put presentation in `.svelte`, content in a
  `-data.ts` fixture, and logic in a `*-view.ts` / `*.svelte.ts` — the app's §2 discipline.

### B9 · Honesty
- Never present an **unbuilt** feature as shipped. Roadmap items get a status badge; empty
  surfaces get an honest empty state (not fabricated rows). Copy claims must match reality
  (no privacy over-claims, no naming tools/tiers that don't exist).

---

## Part C — component vocabulary + task template

### The component surfaces the designer composes from
**Rokkit components** (in-app; the mockup should map to these where the shape fits):
`List` · `Tree` · `Select` · `MultiSelect` · `Menu` · `Table` · `Tabs` · `Toggle` — all
data-first (`items`/`options`/`data` + `bind:value` + `fields` remap + snippets). Use `List` for
always-visible nav/option lists, `Select` when space is tight, `Toggle` for 2–5 mode switches,
`Table` for columnar data, `Tabs` for panelled sections.

**The dojo2 kit** (custom reusable components already built — compose from these, name them):
chrome — `AppShell` · `TopBar` · `NavPane` · `OrgSwitcher` · `ContextHeader` · `TabBar` ·
`MobileShell`; primitives — `SectionHead` · `Banner` · `Chip` · `ClassChip` · `RoleTag` ·
`PhasePill` · `StatBadge` · `EmptyState` (bordered, centered) · `Btn` · `ListSection` ·
`ProjectRow` · `MyDojoRow` · `KanjiToken` · `Icon` (`i-solar:*`); domain — `LadderRung` ·
`RuleRow` · `ConflictCard` · `StanceDial` · `RunCard` · `GateCard` · `NeedsYouBand`/`NeedsRow` ·
`DecisionCard` · `ChatThread`. Reuse these; flag anything new.

### Task template (what I hand you for the designer LLM)
```
DESIGN TASK — <screen / section name>

GUARDRAILS (must follow): Part B of docs/mockups/DESIGN-BRIEF.md —
  named tokens only (24, dark-mode via token flip, no literals); 8-stop type scale;
  4px spacing; mobile-first (phone base, md:/lg: widen); every recurring shape is a
  named component (Rokkit or the dojo2 kit — reuse before adding, flag NEW primitives);
  data-driven props; presentation/content/logic separated; honest empty states.
DELIVERY: claude.ai Artifact → inline Zen-Sumi `colors_and_type.css`, wrap in `.zs`
  (per STYLING.md); dark check via data-theme="dark".

COMPOSE FROM: <the specific kit/Rokkit components this screen uses>
THE ASK: <what to design — the screen's blocks, the data shape, the states incl. empty>
STATES: default · empty · loading? · error? · mobile
RETURN: the artboard + the data shape it renders + any NEW primitive flagged.
```

---

## Part D — the review / outstanding design work (feed section by section)

**Design the whole system, not one plane.** The three planes are one loop — if we design
Dōjō in isolation we miss the **impact surface** (where a rule actually changes behavior) and
the supervision surface, and we hit integration gaps.

- **Sensei** (app + daemon) — where work gets **implemented** and where governance **applies**;
  the **impact surface** (did the rule change the outcome? the FTR/insight/traceability view).
- **Dōjō** (web) — where a team **configures / defines / triages / promotes** governance +
  knowledge.
- **Relay** — where a run is **executed and supervised** away from the keyboard (watch · approve ·
  decide · chat · **nudge**). Relay is also *our* operational surface — a working relay is how the
  live phase/checkpoint view + nudge replaces "are you stuck?".

The loop: **Dōjō defines → Sensei applies → the impact shows in Sensei → contributes back up to
Dōjō → Relay supervises the runs throughout.** Every design section below names its plane; a
change on one plane must show its counterpart on the others (e.g. a rule adopted in Dōjō must have
a visible effect + provenance in the Sensei impact surface).

### DS · Sensei — the impact surface (app plane)
The observatory/app is where adopted governance + learnings **land and show impact**. Design the
surfaces that close the loop back from Dōjō: **what governs this project** (the resolved
constitution, in-app, mirroring the Dōjō preview), **did it help** (rule → measured effect:
FTR/churn/correction signals, the insight copy), **traceability** (rule/decision → the code/PR it
shaped), and the **contribute-up** touchpoint (a learning formed here → shared to the Dōjō, with
the anonymize/preview flow). Ref existing app mockups (`lib/observatory/`, `lib/project/`) +
`MOCKUP-INDEX.md`; keep the app on the design system (`assistant-card.jsx` reference) and the
Rokkit migration. This is the plane most likely to be under-designed — prioritize it.

### DR · Relay — execute & supervise (the third plane, revive it)
Relay was implemented (phone UI + segment-publish + hook-gate; the daemon holds a live line to the
Dōjō over Supabase realtime) but a **UX change broke the integration** — reviving it is high value
because it *also* gives the live phase/checkpoint + nudge supervision. Design (aligned to the rebuilt
dojo2 relay kit — `RunCard`/`GateCard`/`DecisionCard`/`ChatThread`/`NeedsYouBand`): the **live run
view** (phases done/doing/next + activity, a "needs you" band), **approve** a gated command,
**decide** (options + free reply), **chat** to steer, and the **nudge** affordance — identical on
phone + console, ranked by what's blocked on you. Confirm the data path (daemon → Worker
`/v1/t/{tk}/relay/*` → phone/console) end-to-end; the break is likely where the dojo2 relay UI meets
the daemon publish/gate wiring. *(This is as much an integration/build task as a design one — see
the plan.)*

### Dōjō plane — the sections below (D1–D5)

### D1 · dojo2 IA (the spine — mostly built, keep the mockup in sync)
Work-first personal landing (`/you`); a **"my dōjōs"** list (org + role, empty state); click an
org → **`/org/[slug]`** context with its own nav pane (projects-in-jurisdiction + constitution
ladder + role-scoped surfaces). Two navs: **NAV_YOU** (Work · Govern · Relay · Dōjōs) + role-scoped
**NAV_ORG** (Overview · Govern[maintainer] · Clients[lead] · Admin[admin]). Ref:
`lib/dojo2/dojo2-app.jsx` + `dojo2-kit.jsx`. *Status: built in the app; mockup is the source — keep
it the canonical reference as we refine.*

### D2 · Governance — rule packs + the ladder (in flight — the current design focus)
- **Rule pack shape** (aligned with Jerry): each pack carries **area** (7-set: principles ·
  architecture · security · compliance · tech-stack · design · process), **scope** on the ladder
  where **"organization"** replaces company/client (company-vs-client is the *viewer's*
  relationship, resolved per-membership) — i.e. `organization · team · project · stack · personal`,
  **enforcement** (advisory/recommended/required/mandatory — drives both precedence *and* whether a
  rule is always-injected vs on-demand), real **source** (Robert C. Martin, OWASP, PCI SSC,
  **Rokkit** for Zen-Sumi, Gang of Four…), and **rules[]** each `{ text, detail?, hard?, checker?,
  skill? }`. Pack row = at-a-glance summary (kanji · name · **area chip** · **scope chip** · source
  · "N rules ▾") that **expands** to the rules (with hard/non-negotiable marker + checker badge).
- **Constitution / ladder preview** — the resolved ladder for a project (org → … → stack), conflicts
  settled, mandatory locks shown, provenance link per rule → its authoring scope.
- Ties to the instruction-delivery model (separate architecture doc, in progress) — the pack's
  **enforcement** is the delivery discriminator.
- **Open bug:** the **adopted pill** doesn't match the mockup in dark mode → needs a real
  `success-edge` border token (dōjō lacks the `-edge` tokens today) + match the mockup's
  `check-circle` + success tone/soft/edge chip. *(Independent, small — slot in anytime.)*

### D3 · The re-added org consoles (bring into the dojo2 IA + kit)
Eight consoles dropped in the dojo2 pass, to re-skin to the kit and role-scope in NAV_ORG:
**Triage · Approvals · Knowledge** (maintainer) · **Engagements · Incidents · Client-audit** (lead)
· **Identity & SSO · Health/Monitor** (admin). Each already exists (routes + backend); the design
job is the dojo2-kit re-skin + role-grouped nav destination. (Members/Scopes/Audit already in the
admin nav.)

### D4 · Evergreen guardrail fixes (carried from the old review — apply across surfaces)
- **Design-system adoption** — no hand-rolled inline styles over numbered tokens; snap off-scale
  sizes to the scale; replace any raw color literal with a `-soft`/`-edge` token. *(This is what
  made old-dōjō dark mode unreadable — the same class of bug as the transparent-button + adopted-pill
  issues.)*
- **Danger affordances** — destructive actions (Decline/Retract/Delete) use the **danger** style,
  not a neutral button.
- **Real data, not hardcoded** — verdicts/ladders/counts read the real order/data, not a fixed
  literal (nav badges, precedence ladder).
- **Shared primitives, not local copies** — one `SectionHead`/`ListSection`/`EmptyState`/`Panel`,
  never a per-screen re-definition.
- **Responsive stacking** — every screen takes a mobile treatment; wide tables → one card per row.

### D5 · Website (verify — may be stale/done)
The old review flagged website copy accuracy (privacy over-claims, export/assistants/instruments
claims), a roadmap+waitlist beat, and responsive. **Verify current state before actioning** — much
may be shipped. Keep the honesty rule (B9). *(Lower priority than the dōjō/governance work.)*

---

## Part E · Coverage directives (2026-07-24 — from the coverage audit)

Full audit: [`../analysis/2026-07-24-coverage-audit.md`](../analysis/2026-07-24-coverage-audit.md).
Triggered by the logout 404 (a designed-but-uncovered critical path). Four directives:

- **E1 · Cover the five missing critical paths.** Design them as first-class Zen-Sumi
  surfaces, not framework fallbacks: **404 / not-found**, **`+error.svelte` boundary**
  (failed load → calm page + retry), **permission-denied** (direct URL to a role-gated
  section), **session-expired / re-auth** (ended session → cue + return-to-where-you-were),
  **rate-limit (429)**. None exist today in spec or code.
- **E2 · `EmptyState` is the law for empties.** The shared `kit/EmptyState.svelte` (空 ·
  "Still listening.") is already adopted by 7+ screens — **always compose it, never
  hand-roll an empty branch.** Audit `ScrYourWork` / `ScrRelayApprove` for uncovered empties.
- **E3 · Consolidate duplicated shells into named kit primitives** (data-driven props, no
  inline `padding`/`letter-spacing` literals — B2/B4): `Card` (the `bg-paper-soft` shell,
  ~23 copies), `ListItem` (border-bottom row, 8+), `FieldLabel` (uppercase section label,
  10+), `LabelWithIcon` (6+); fold `Eyebrow` into `SectionHead`'s eyebrow slot. Batch these,
  browser-verify by computed style (B1). *(Big-touch — do NOT sweep blind.)*
- **E4 · Shared logic → a util, not a copy.** `getInitials` (`kit/initials.ts`, shipped
  `0967f0f7`) is the template: a 2nd copy of any computation is a refactor signal.

---

## Superseded — collapsed into this doc
`DOJO-REVIEW.md` (2026-07-16, pre-dojo2) · `DOJO2-MISSING-CONSOLES.md` · `DOJO-REDESIGN-BRIEF.md` ·
`SPACING-CLEANUP.md`. Their live content lives here now; the originals can be retired.
