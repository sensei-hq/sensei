# 具 · Project window · Instruments

**Segment:** 04 · The project window
**Route:** `/project/[id]/instruments`
**Source mockup:** [`lib/instruments-simple.jsx`](../../mockups/Sensei/lib/instruments-simple.jsx) → project variant of the three-tab shell
**App file:** `app/src/routes/project/[id]/instruments/+page.svelte`

## Purpose

Instruments scoped to a single project. Same three-tab structure
(Playground / Replay / Health) as
[[screen/observatory-instruments-health]] + siblings, but every
tab is scoped to this project's tool activity, sessions, and MCP
usage. Useful for auditing what the assistants have actually done
inside one project's boundary.

Kanji is 具 — *instrument*.

## Data invariants

- All three Instruments endpoints accept `?project=<id>` and
  return scoped data.
- The MCP L1 grid on Health shows which MCPs were called within
  this project's sessions, not the global inventory.
- Sub-nav placement follows the Instruments rule (below the hero
  via `subNav` prop) — same as the Observatory Instruments.

## Signals shown

Same as the Observatory Instruments tabs, filtered:

- **Playground** — the same tool tree; execute round-trips log
  against this project as attribution.
- **Replay** — session picker shows only this project's
  sessions.
- **Health** — MCP grid + signal strip scoped to this project's
  usage.

## Done gate

- Every tab correctly scopes to the project id.
- Playground executions are attributed to this project.
- Health L2 signals scope by both MCP AND project.
- Sub-nav renders below the hero (not above).

## Wrong gate

- **A tab loads with global data despite `?project=<id>`.** Scope
  filter regressed.
- **Playground execution attributed globally.** Attribution
  regression.
- **Health L1 grid identical to Observatory.** Not project-scoped.
- Every failure mode inherited from the sibling Instruments specs.

## Related

- [[screen/observatory-instruments-playground]]
- [[screen/observatory-instruments-replay]]
- [[screen/observatory-instruments-health]]
- [[pipeline/mcp-surface]]
