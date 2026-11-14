# Shared component catalogue — one clean list for the app and the dōjō

**Status:** approved by the user (2026-11-14), with one ruling: the preview covers the dōjō's components as well
(see Rule 6). Consolidates #293, #409, #410, #412, #384, #47, #406 and the relay rebuild's rokkit-first pass.

## Why

The user (2026-11-14): cleaning up and consolidating one list of shared components makes the app more stable and
consistent. The inventory says the same:

| Where | Components | Use rokkit |
|---|---|---|
| `packages/ui` (`@sensei/ui`, shared) | 11 | 3 |
| `app/src/lib/components` | 25 | 6 |
| `app/src/lib/relay` | 33 | 0 |
| `dojo/src/lib/components` (+ `kit/`) | 55 | 4 |

Hand-rolled patterns that recur (files): `<style>` blocks 34 · `overflow-auto` 22 (#293: one scroll owner per
screen) · hand-drawn SVG charts 7 · `.chip` 5 (#410) · tablists 4 · switches 3 · `<table>` 3. Names defined twice:
`Eyebrow`, `Spark`, `TabBar`. Scroll has broken repeatedly; `preview:scroll` now pins it for the Relay shell only.

## Rules

1. **rokkit first.** A shared component is a rokkit component used directly. When rokkit lacks a prop, the prop goes
   into rokkit (as Pill `tone`, StatCard `accent`, Toolbar `kind`/`count` did), never a local wrapper or CSS override.
2. **`@sensei/ui` holds only what is sensei's own** — the voice (Notice, EmptyState, StateView, Loading), the glyph
   mode (Mark), the shell's store connection (ShellScreen). App and dōjō import it; neither keeps a copy.
3. **Glyphs from one package** (`@sensei/glyphs`, repurposed per #406): no `i-solar-*` in source.
4. **Styles through tokens and utilities** (#409): a `<style>` block only for state selectors, keyframes, `color-mix`.
5. **One scroll owner per screen** (#293): the shell body. `overflow-auto` elsewhere needs a named reason.
6. **Every shared component has `/preview` fixtures, for the app's uses and the dōjō's** (user ruling 2026-11-14),
   and the scroll test covers every shell-hosted screen in both. A shared component's fixtures show each
   consumer's real cases (the props the app sends and the props the dōjō sends), so one page answers "does this
   look right everywhere it is used". A dōjō-local component that the list below replaces gets fixtures before it
   is migrated, so its before and after are compared like the relay's were.
   - **Where it runs:** the catalogue rules (`app/src/lib/preview/catalogue.ts`) and the index/component pages
     move to `@sensei/ui`, and the dōjō mounts its own dev-only `/preview` from them. The app cannot import the
     dōjō's `$lib` (a separate SvelteKit app), so each app hosts the fixtures for its own components, and the
     shared components' fixtures live once in `@sensei/ui` and show in both. The capture rig
     (`preview:capture|sheet|scroll`) takes the host as a parameter.

## The list (candidate mapping — each verified when migrated)

| Shared component | Replaces |
|---|---|
| rokkit `ScreenShell` via `@sensei/ui` `ShellScreen` | `PageHeader`, dōjō `SectionHead`, the four header/scroll patterns (#293) |
| rokkit `Pill` (tone · outline · sm) | relay `Pill`, `Tag`; dōjō `Chip`, `ClassChip`, `CheckerChip`, `PhasePill`, `RoleTag`; the 5 `.chip` copies (#410) |
| rokkit `StatCard` | `MetricCard`, `ProgressCard`, dōjō `StatBadge` — where they are a number and its label |
| rokkit `Switch` / `Toggle` | `AssistantCard`'s hand switch; app `Switch` wrapper if it adds nothing (#410) |
| rokkit `Tabs` | app `TabBar`, dōjō `TabBar`, `SubTabs` (#47) |
| rokkit `List` (item snippet) | `LaneList`, `CompletedList`, `QueueSection`, dōjō `ListSection` — rows with their own buttons need a non-selecting mode first |
| rokkit `Timeline` (item snippet) | `TaskHistory`/`HistoryEntry`, `RelayTimeline`, dōjō `RunActivity` — fixes the off-centre dots |
| rokkit chart `Sparkline` | `FtrStrip` (#412), `MetricSparkline`, dōjō `Spark` ×2 |
| rokkit `Button` / `Toolbar` | dōjō `Btn`, `@sensei/ui` `ActionButton`, `ToolbarItem` + `toolbar-item.js` |
| rokkit `Table`, `Avatar`, `Message` | the 3 `<table>`s, dōjō `Avatar`, banner look-alikes |
| `@sensei/ui` `Mark` (+ `@sensei/glyphs`) | `Kanji`, dōjō `KanjiToken`, `Icon`; `Eyebrow` once |
| `@sensei/ui` status mark | `StatusDot`, `StatusDisc`, `StatusIndicator`, `RelayStatusBadge` |

Stays local: screen-specific compositions (LaneRow, MemberCard, DecisionRow, PlanGraph, …) — built from the above.

## Order

1. Relay first — **done 2026-11-14:** app + dōjō on rokkit 1.15.4 (`3cd4a8cac`); toolbar via `kind`/`count`,
   `relay-toolbar.css`, `relay-filters.css`, `@sensei/ui` ToolbarItem deleted (`ac4640baf`); Working accent
   (`035b8aeac`); relay `Pill`/`Tag` → rokkit Pill (`942488387`). **Remaining:** Timeline; List where rows have no
   buttons; the mockup's host tag ("in Cursor", needs a Pill `title`).
2. Guards before mass migration: specs that fail on a new hand chip, a `<style>` colour, an `i-solar-*` class, an
   `overflow-auto` outside the shell — each with today's count as an allowlist that only shrinks.
3. The dōjō's `/preview` (Rule 6), before the first family that touches the dōjō.
4. One component family per change, app and dōjō together, each with `/preview` fixtures in both: Pill → Tabs →
   StatCard → Sparkline → Button → Table → status mark → shell (#293, the largest).
5. `@sensei/glyphs` repurposed (#406) alongside step 4.

## Open questions

- Where a rokkit component is a control (List, Pill with `removable`) and our use is display-only, confirm the
  display mode exists before migrating.
