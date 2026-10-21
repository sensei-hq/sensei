# PRD modules — index and spec template

29 modules across five planes. **One module = one PRD spec = one roadmap section
= one `.feature` set.** Plane pages are the reading order; this page is the index
and the spec template.

## The module index

| Plane | Modules | Plane page |
|---|---|---:|
| Foundation | M1–M6 | [`01-foundation/`](01-foundation/README.md) |
| Insight | M7–M13 | [`02-insight/`](02-insight/README.md) |
| Delivery | M14–M22 | [`03-delivery/`](03-delivery/README.md) |
| Team | T1–T5 | [`04-team/`](04-team/README.md) |
| Platform | P1–P3 | [`05-platform/`](05-platform/README.md) |
| **Total** | | **29** |

Per-module specs land here as each module is next touched (Phase 2 of
[`../../reorg.md`](../../reorg.md)), folding in the legacy `features/` dossiers
and `spec/` tree. Until a module spec exists, its status lives in
[`../../roadmap/`](../../roadmap/README.md) and its legacy docs are still at
`../../features/` and `../../spec/`.

**Phase 2 progress (by plane, priority order):**
- **Foundation — ✅ all six authored 2026-10-13.** M1 [`01-bootstrap-provisioning/`](01-foundation/01-bootstrap-provisioning/README.md) ·
  M2 [`02-setup-preferences/`](01-foundation/02-setup-preferences/README.md) · M3 [`03-capture-sessions/`](01-foundation/03-capture-sessions/README.md) ·
  M4 [`04-code-graph/`](01-foundation/04-code-graph/README.md) · M5 [`05-inference-gateway/`](01-foundation/05-inference-gateway/README.md) ·
  M6 [`06-storage-data-model/`](01-foundation/06-storage-data-model/README.md). Each owns a `scenarios/` `.feature` set.
  The folded dossiers `features/01-setup.md` and `features/02-config.md` are stamped superseded; the shared
  pipeline specs (`capture`, `code-graph`, `inferencing`, `bootstrap-resolution`) stay live until later planes fold.
- **Insight — ✅ all seven authored 2026-10-13.** M7 [`07-analyzer-and-signals/`](02-insight/07-analyzer-and-signals/README.md) ·
  M8 [`08-metrics-and-health/`](02-insight/08-metrics-and-health/README.md) · M9 [`09-memory-and-knowledge/`](02-insight/09-memory-and-knowledge/README.md) ·
  M10 [`10-patterns-and-conventions/`](02-insight/10-patterns-and-conventions/README.md) · M11 [`11-traceability-and-doc-drift/`](02-insight/11-traceability-and-doc-drift/README.md) ·
  M12 [`12-libraries/`](02-insight/12-libraries/README.md) · M13 [`13-governance-and-rules/`](02-insight/13-governance-and-rules/README.md).
  Each owns a `scenarios/` `.feature` set. The M8-owned dossiers `features/metrics/{catalog,feature,architecture-metrics}.md`
  are stamped superseded; `features/03-observatory.md` and `features/05-governance.md` stay live (they span the Delivery
  and Team planes too), as do the shared pipeline specs until those planes fold.
- **Delivery — ✅ all nine authored 2026-10-13.** M14 [`14-mcp-context-delivery/`](03-delivery/14-mcp-context-delivery/README.md) ·
  M15 [`15-observatory/`](03-delivery/15-observatory/README.md) · M16 [`16-project-window/`](03-delivery/16-project-window/README.md) ·
  M17 [`17-atlas/`](03-delivery/17-atlas/README.md) · M18 [`18-instruments/`](03-delivery/18-instruments/README.md) ·
  M19 [`19-insights-and-impact/`](03-delivery/19-insights-and-impact/README.md) · M20 [`20-planner-and-baseline/`](03-delivery/20-planner-and-baseline/README.md) ·
  M21 [`21-relay/`](03-delivery/21-relay/README.md) · M22 [`22-agentic-execution/`](03-delivery/22-agentic-execution/README.md).
  Each owns a `scenarios/` `.feature` set (M14: 3, M15: 3, M16: 2, M17: 2, M18: 2, M19: 2, M20: 2, M21: 2, M22: 1 = 19 files).
  The module-owned dossiers `features/03-observatory.md` (M15), `features/04-project.md` (M16) and `features/06-relay.md` (M21)
  are stamped superseded; the shared pipeline specs (`spec/pipeline/{mcp-surface,context-delivery,semantic-search,impact,agent-execution,insights,benchmarks,...}`)
  stay live until their modules fold.

## The module-spec template

Every module spec (`modules/<plane>/<module>/README.md`) answers the five
questions that make a module spec usable, then the gaps and the
scenarios:

```
# <Module name>  · <M#>

**Plane:** <plane> · **Status:** see ../../roadmap/<plane>.md · **Serves:**
personas (link into prd/03-personas.md)

## Purpose
One paragraph. What the user feels. Not what the module contains.

## In scope / out of scope
- In: …
- Out: …

## Surfaces
Who touches it, and how — app screen / MCP tool / daemon task / CLI / web.

## Data invariants
- Tables and joins that must hold before this module can look right.
- Stated as preconditions, not implementation.

## Signals shown
What each visible number / card / chip means, with worked examples.

## Done gate
The English claim we make when the module is finished. Prefer a
curl-shaped / test-shaped check for the non-obvious ones.

## Wrong gate
What would make this module embarrassing to demo. The failure modes
we specifically watch for.

## Scenarios
`.feature` files live in `scenarios/` beside the spec and are linked
here. See ../../reorg.md §6.

## Open gaps
Pointer to `../../roadmap/06-gaps.md` entries for this module.

## Related
Architecture · journeys · mockups · decisions
```

## Folder and naming conventions

- Plane folders are numbered: `01-foundation`, `02-insight`, `03-delivery`,
  `04-team`, `05-platform`.
- Module folders are numbered + kebab-case: `01-bootstrap-provisioning`,
  `07-analyzer-and-signals`, `14-mcp-context-delivery`, …
- Each plane carries a `README.md`; each module will carry one plus a
  `scenarios/` folder of `.feature` files.
- Diagrams referenced by specs live in [`../assets/diagrams/`](../assets/diagrams/README.md).