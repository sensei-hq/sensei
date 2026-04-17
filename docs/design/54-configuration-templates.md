---
name: Configuration & Templates
description: Config schemas, guardrails template, state file schema, and phase doc templates
date: 2026-04-17
type: design
traces:
  - ideas/03-configuration.md
  - ideas/04-cross-cutting.md
  - blueprints/01-workflow-engine.md
---

# Configuration & Templates

## Overview

The data layer files that multiple components read and write. Defines schemas for config, guardrails, state, and phase document templates.

---

## Config schema (.sensei/config.yaml)

See [ideas/03-configuration.md](../ideas/03-configuration.md) for full spec.

## Guardrails template (.sensei/guardrails.md)

See [blueprints/01-workflow-engine.md](../blueprints/01-workflow-engine.md) section 2.

## State file schema (.sensei/state.yaml)

See [blueprints/01-workflow-engine.md](../blueprints/01-workflow-engine.md) section 7.

## Phase doc templates (docs/templates/)

Templates to create:

| Template | Sections |
|----------|----------|
| `idea.md` | frontmatter (name, description, date, status), Problem, Goals, Constraints, Open questions |
| `analysis.md` | frontmatter (+ origin), Current state, Feasibility, Approaches (2-3 with tradeoffs), Recommendation |
| `blueprint.md` | frontmatter (+ origin, analysis), Overview, Architecture, Components, Interfaces, Data flow, Integration points |
| `experiment.md` | frontmatter (+ origin), Hypothesis, Approach, Findings, Recommendation (incorporate/discard) |
| `plan.md` | frontmatter (+ origin, blueprint), Features (ordered), per-feature: acceptance criteria, test scenarios, dependencies |

Existing templates to keep: `design.md`, `feature.md`.

---

## Testing

Verify templates by creating a doc from each template and checking frontmatter parsing.
