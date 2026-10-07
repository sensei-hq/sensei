# 層 · Diagrams · Layers and Cycles

**Segment:** 04 · Observatory / Project window
**Routes:** `/project/[id]/diagrams/layers` · `/project/[id]/diagrams/cycles`
**Source mockup:** `Sensei Schema and Call Graph v8.dc.html` views `layers` and `cycles` (`…:2443`, `…:2487-2537`)
**Data:** `sensei.structure_edges` → `sensei.module_edges`, both already populated. **No new indexing.**
**Daemon:** `GET /api/projects/{id}/diagrams/layering?level=module|file&kinds=…` (#222, `2e898607`)
**App files:** `diagrams/layering-state.svelte.ts`, `diagrams/layers/+page.svelte`, `diagrams/cycles/+page.svelte`
**Rokkit:** `LayersDiagram` + `ViolationsControl` (`@rokkit/graph@1.9.0`), `Toolbar` (`@rokkit/ui`)
**Status:** SHIPPED 2026-10-06 (`d0f191d7` screens, `1690a5d7` chip fix, `89e465ae` toolbars)

## Purpose

Answer **do these calls flow downward** — and, where they do not, **which arrow
to cut**.

Two screens because they are two questions, but ONE controller and one request:
the layering, the cycles and the coverage all come out of a single `analyse`
over one read of `structure_edges`, and that read is the entire cost (1.5–74 s
per project). Two controllers would pay it twice for two answers that must agree
about which arrow is the weakest link.

Kanji is 層 — *layer* — and 環 — *ring* — for Cycles.

## Data invariants

- **The layering is DERIVED, never declared.** `layerSource: "derived"` means it
  was measured from the call graph. The screen therefore answers "do these calls
  flow downward", not "is this the architecture you intended" — the second needs
  a layering somebody wrote down (#230).
- **`skip` IS NOT A VIOLATION.** A call that skips a layer is legal under relaxed
  layering, and `@rokkit/graph` separates it from a climb in its own vocabulary
  (`layout/edges.js` filters `conformance === 'up'`, not `!== 'down'`). Measured
  on project `sensei`: 25 climbs against 15 skips, so counting both overstates by
  60%.
- **Every `up` edge is a cycle's back edge.** The feedback order chooses it, so
  the arrows highlighted on Layers and the cut named on Cycles are the same set
  of facts and cannot disagree.
- **A unit depending on ITSELF is not a cycle.** At module grain that is two of
  the module's own files referring to each other — 21 of them on `sensei` — and
  folding them into the cycle list would report a module as circular for an
  internal reference.
- **`coverage.unknownUnit` is a dependency whose endpoint owns no file**, and so
  is not a unit. 23 of 194 on `sensei`; it was 95 of 122 on the largest client
  project before #231.

## Signals shown

- **Layers** — `LayersDiagram` ranked by the daemon's `layer`, with
  `ViolationsControl` bound to `showEdges`.
- **Three lines above the picture, none decorative.** What the layering IS
  (`sourceNote`), what it found (`breaksNote`), what the picture leaves out
  (`coverageLine`).
- **Cycles** — one card per strongly-connected component, deepest-depended-on
  first: an eyebrow (`Cycle · N units · layer L`), every member, every inner
  dependency heaviest first with the cut prefixed `cut ·` in accent, and the
  sentence `Cut <a> → <b> (×n, the weakest link) and the loop opens.`
- **Depends on itself** — listed apart, with what it means at module grain.
- **Loading / empty / error**, three distinct states.

## Done gate

1. `GET …/diagrams/layering?level=module&kinds=calls` for `sensei` returns
   `depth ≥ 2`, a non-empty `nodes`, and a `conformance` histogram containing
   both `up` and `skip`. **Measured 2026-10-06: 154 units, depth 4, 91 down /
   63 level / 25 up / 15 skip.**
2. Layers renders the bands, and `breaksNote` names a non-zero climb count.
3. `coverageLine` shows `unknownUnit` and matches the payload.
4. `sourceNote` says the layering was **measured**, not declared.
5. Cycles lists `sensei`'s single 22-member component with 88 inner edges and
   names the cut `senseid/adapters → senseid/tasks ×2`.
6. A project with no cycle renders `cycles-none` WITH A SENTENCE, never a blank
   canvas.
7. Switching grain re-queries; a selection made at the old grain is dropped.
8. e2e passes against the live daemon.

## Wrong gate

- **A skip counted as a violation.** Reports a problem the architecture does not
  have, and overstates by 60% on this corpus.
- **An empty violations list under no caption.** Reads as "this architecture is
  clean" whether the layering found nothing or the request returned nothing.
- **An empty Cycles canvas.** The screen's whole subject is absence, so absence
  must be stated in words — `cycles-none`, not nothing.
- **A stale payload under an error banner.** Worse here than elsewhere: the
  arrows it highlights are the ones somebody is about to go and cut.
- **Self-dependencies folded into the cycle list.** Reports a module as circular
  because two of its files refer to each other.
- **`unknownUnit` omitted.** A diagram missing a share of its edges with no
  number beside it reads as a simple codebase.

## Not in scope

A DECLARED layering, which is a different question with a different source
(#230). Precomputing the read so the FIRST paint is fast — the cache in #233
fixes every request after it, which is where the interaction lives.

## Related

- [#222](https://github.com/sensei-hq/sensei/issues/222) — the derivation
- [#232](https://github.com/sensei-hq/sensei/issues/232) — these screens
- [#233](https://github.com/sensei-hq/sensei/issues/233) — the read cost
- [#231](https://github.com/sensei-hq/sensei/issues/231) — why `unknownUnit` fell
