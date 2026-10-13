# PRD modules — index and spec template

29 modules across five planes. **One module = one PRD spec = one roadmap section
= one `.feature` set.** Plane pages are the reading order; this page is the index
and the spec template.

## The module index

| Plane | Modules | Plane page |
|---|---|---:|
| Foundation | M1–M6 | [`foundation/`](foundation/README.md) |
| Insight | M7–M13 | [`insight/`](insight/README.md) |
| Delivery | M14–M22 | [`delivery/`](delivery/README.md) |
| Team | T1–T4 | [`team/`](team/README.md) |
| Platform | P1–P3 | [`platform/`](platform/README.md) |
| **Total** | | **29** |

Per-module specs land here as each module is next touched (Phase 2 of
[`../../reorg.md`](../../reorg.md)), folding in the legacy `features/` dossiers
and `spec/` tree. Until a module spec exists, its status lives in
[`../../roadmap/`](../../roadmap/README.md) and its legacy docs are still at
`../../features/` and `../../spec/`.

**Phase 2 progress (by plane, priority order):**
- **Foundation — ✅ all six authored 2026-10-13.** M1 [`bootstrap-provisioning/`](foundation/bootstrap-provisioning/README.md) ·
  M2 [`setup-preferences/`](foundation/setup-preferences/README.md) · M3 [`capture-sessions/`](foundation/capture-sessions/README.md) ·
  M4 [`code-graph/`](foundation/code-graph/README.md) · M5 [`inference-gateway/`](foundation/inference-gateway/README.md) ·
  M6 [`storage-data-model/`](foundation/storage-data-model/README.md). Each owns a `scenarios/` `.feature` set.
  The folded dossiers `features/01-setup.md` and `features/02-config.md` are stamped superseded; the shared
  pipeline specs (`capture`, `code-graph`, `inferencing`, `bootstrap-resolution`) stay live until later planes fold.
- **Insight — ✅ all seven authored 2026-10-13.** M7 [`analyzer-and-signals/`](insight/analyzer-and-signals/README.md) ·
  M8 [`metrics-and-health/`](insight/metrics-and-health/README.md) · M9 [`memory-and-knowledge/`](insight/memory-and-knowledge/README.md) ·
  M10 [`patterns-and-conventions/`](insight/patterns-and-conventions/README.md) · M11 [`traceability-and-doc-drift/`](insight/traceability-and-doc-drift/README.md) ·
  M12 [`libraries/`](insight/libraries/README.md) · M13 [`governance-and-rules/`](insight/governance-and-rules/README.md).
  Each owns a `scenarios/` `.feature` set. The M8-owned dossiers `features/metrics/{catalog,feature,architecture-metrics}.md`
  are stamped superseded; `features/03-observatory.md` and `features/05-governance.md` stay live (they span the Delivery
  and Team planes too), as do the shared pipeline specs until those planes fold.

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

- Module folders are kebab-case: `code-graph`, `dojo-project-planning`, …
- Each plane carries a `README.md`; each module will carry one plus a
  `scenarios/` folder of `.feature` files.
- Diagrams referenced by specs live in [`../assets/diagrams/`](../assets/diagrams/README.md).