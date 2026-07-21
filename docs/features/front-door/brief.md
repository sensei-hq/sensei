---
name: front-door — brief
updated: 2026-07-20
---

# Front door — brief

> **Retired / folded (2026-07-20).** The front door is no longer a standalone
> feature. Its user-facing story is now the **Working style** section of the
> [Project feature](../04-project.md#working-style--the-operation-manual); its deep
> design lives in the [playbook design module](../../design/playbook.md). This
> dossier is kept for history — the two above are canonical.

> Intent. The feature's goal + the axes it's built on — not the layout or
> behavior (that's [design.md](design.md)).

## Purpose

The front door is sensei's adaptive-process entry point. At the start of a work
chunk, it classifies the chunk on three axes and **recommends a playbook**
(recommend-and-confirm) — so the rigor applied to a chunk is proportional to its
risk, instead of every chunk getting the same ceremony. It is the "start here"
surface: before a coding assistant or the sensei app dives into a chunk, it
passes through intake. Depth follows risk — a high-risk chunk always keeps the
human in the loop; a low-risk chunk may auto-select a playbook once its
recommendation has earned trust.

## The three axes (the "three tiers")

Sensei infers these where it can and only asks the user what it can't infer —
they're sensei's reading of the chunk, shown back for a sanity check, not a form
filled from scratch.

| Axis | Values | Inferred from |
|---|---|---|
| `lifecycle` | `greenfield` · `stable` | spine/drift — existing code + docs → `stable`; empty/new → `greenfield` |
| `intent` | `explore` · `ux` · `feature` · `enhancement` · `bug` | the goal of the chunk |
| `risk` | `low` · `high` | blast-radius from the code graph (callers / community reach) |

Together the three axes select a playbook — a way of working (e.g. debug-flow,
spec-driven) — from a six-playbook catalog.

## Audiences of this feature

- **The individual developer** — runs intake at the start of a chunk, via the
  CLI/agent surface or the app.
- **The product owner** — validates that the adaptive process recommends
  sensibly and that depth actually follows risk.
- **The agent** — executes the playbook the front door recommends.

## More

Full behavior + surfaces → [design.md](design.md); what works today + gaps →
[tests/acceptance.md](tests/acceptance.md).
