---
type: design
---
# Mockup design brief

How we design sensei / dōjō mockups so they translate to the app with no visual
re-derivation. This is the durable half of the old `docs/mockups/DESIGN-BRIEF.md`. The
live review of outstanding screens is separate: [`../mockups/dojo2-review.md`](../mockups/dojo2-review.md).

Styling detail — the tokens, the type scale, spacing, and component classes — is not
repeated here. It lives in the canonical sources:

- [`../architecture/frontend-svelte-guidelines.md`](../architecture/frontend-svelte-guidelines.md) — the enforced in-app rules.
- [`../mockups/Sensei/CLAUDE.md`](../mockups/Sensei/CLAUDE.md) — the same system for claude.ai artifacts (the `.zs` no-rokkit drop-in) plus the pre-delivery self-check.

## The loop

We iterate section by section:

1. Discuss a section of the review, or a new ask.
2. Turn it into a designer task using the template below — the guardrails, the component
   vocabulary, the specific ask, and the target screens.
3. Run it through the designer LLM (claude.ai Artifact → the `.zs` layer, since artifacts
   can't use rokkit).
4. Drop the mockup back into `docs/mockups/`, review it against the guardrails, mark it done.

The output is a mockup a build agent ports to Svelte with no re-derivation — it already
uses the tokens, the scale, and named components.

**Two delivery contexts, one system.** claude.ai mockups use the `.zs` scoped layer (no
rokkit); the app uses rokkit (`presetRokkit`). Same tokens, same 8-stop scale, same
component shapes — only the delivery differs. A mockup built to the guardrails maps 1:1 to
a rokkit or kit component.

## Guardrails

Every design task inherits these. They exist to prevent drift, redundancy, and
un-portable output. The token/scale/spacing specifics live in the styling sources above;
these are the principles.

1. **Tokens, named only.** Use the 24 named tokens (paper / ink / accent / status), never a
   hex, `oklch()`, or `rgb()` literal, never a numbered z-scale token. Don't add a parallel
   color for an intent a token already covers. Dark mode is automatic via the token flip —
   never write a per-mode color in markup, and verify the dark render (borders need a real
   `-soft` / `-edge` token or they vanish in dark).
2. **Type, the 8-stop scale.** Named sizes only (`text-xs`…`text-4xl`), never a literal px.
   Headers use the eyebrow-over-title pattern; step section headings down so the page title
   stands alone.
3. **Color is meaning, not decoration.** `accent` for the brand beat (rationed), status
   tokens for state, ink for text, paper for surface. Hairlines over shadows; air over
   density. Status is always a status token, never a raw green or red.
4. **Spacing, the 4px grid.** Grid stops only, never a literal px and never a new stop —
   "need 18, use 16 or 20." Radii come from tokens.
5. **Responsive, mobile-first.** Phone is the base; `md:` / `lg:` widen. Every screen stacks
   on a phone; a wide table becomes one card per row, never a sideways-scrolling page.
6. **Componentize — map, don't hand-roll.** Every recurring shape is a named component: a
   Rokkit component or a dojo2-kit component. A shape used twice or more is a component —
   name it so the build maps it once. Reuse before adding; flag a genuinely new primitive
   as `NEW primitive: …` so it enters the kit.
7. **Data-driven.** A row / card / list renders from a shape (an object) configured by
   props — one `ProjectRow` or `RuleRow` driven by data, not a variant per instance. Design
   the shape alongside the component.
8. **Separate presentation, content, and logic.** Presentation is props in, markup out.
   Content is the data shape it renders. Logic — derivations, selection, status-mapping — is
   a pure function over the data, not baked into the markup. This is what lets the build put
   presentation in `.svelte`, content in `-data.ts`, and logic in `*-view.ts`.
9. **Honesty.** Never show an unbuilt feature as shipped: roadmap items get a status badge,
   empty surfaces get an honest empty state (not fabricated rows). Copy must match reality —
   no privacy over-claims, no naming tools or tiers that don't exist.

## Component vocabulary

The designer composes from two surfaces. Name the components so the build maps each once.

**Rokkit (in-app):** `List` · `Tree` · `Select` · `MultiSelect` · `Menu` · `Table` · `Tabs`
· `Toggle` — all data-first (`items` / `options` + `bind:value` + `fields` remap +
snippets). Use `List` for always-visible nav, `Select` when space is tight, `Toggle` for
2–5 modes, `Table` for columns, `Tabs` for panelled sections.

**dojo2 kit (custom, already built):**

- chrome — `AppShell` · `TopBar` · `NavPane` · `OrgSwitcher` · `ContextHeader` · `TabBar` · `MobileShell`
- primitives — `SectionHead` · `Banner` · `Chip` · `ClassChip` · `RoleTag` · `PhasePill` · `StatBadge` · `EmptyState` · `Btn` · `ListSection` · `ProjectRow` · `MyDojoRow` · `KanjiToken` · `Icon` (`i-solar:*`)
- domain — `LadderRung` · `RuleRow` · `ConflictCard` · `StanceDial` · `RunCard` · `GateCard` · `NeedsYouBand` / `NeedsRow` · `DecisionCard` · `ChatThread` · `InboxRow` · `PlanOutline`

## Task template

The text handed to the designer LLM:

```
DESIGN TASK — <screen / section>

GUARDRAILS: docs/design/mockup-brief.md — named tokens only (dark via token flip, no
  literals); 8-stop type scale; 4px spacing; mobile-first; every recurring shape a named
  component (reuse before adding, flag NEW); data-driven props; presentation / content /
  logic separated; honest empty states.
DELIVERY: claude.ai Artifact → inline the Zen-Sumi CSS, wrap in .zs (see
  mockups/Sensei/CLAUDE.md); dark check via data-theme="dark".

COMPOSE FROM: <the kit / Rokkit components this screen uses>
THE ASK: <the screen's blocks, the data shape, the states incl. empty>
STATES: default · empty · loading? · error? · mobile
RETURN: the artboard + the data shape it renders + any NEW primitive flagged.
```
