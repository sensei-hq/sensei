# 隣 · Diagrams · Neighbourhood

**Segment:** 04 · Observatory / Project window
**Route:** `/project/[id]/diagrams/neighbourhood?focus=<node id>`
**Source mockup:** `docs/mockups/Sensei/Sensei Schema and Call Graph v8.dc.html` (`view:"hood"`)
**Data:** `sensei.edges` (`calls`) + `sensei.graph_nodes` + `sensei.folder_projects`. **No new indexing.**
**Daemon:** `GET /api/projects/{id}/diagrams/neighbourhood?focus=<uuid>&depth=1..3` (`4845e435`)
**App files:** `diagrams/neighbourhood-state.svelte.ts`, `diagrams/neighbourhood/+page.svelte`
**Rokkit:** `Neighborhood` (`@rokkit/graph@1.9.0`)
**Status:** BUILT 2026-10-07

## Purpose

One symbol, read left to right: **what calls it, and what it calls.** At depth 2
and 3, the callers' callers and the callees' callees. Centre on any neighbour to
keep walking.

Kanji is 隣 (*neighbour*).

## Data invariants

- **There is no default symbol.** A missing or malformed focus is a 400, and the
  screen opens on a picker. A picture of a symbol the reader did not choose
  answers a question nobody asked.
- **Two directed walks.** The callers side follows edges in to the focus and the
  callees side follows edges out, each with its own claimed set. A mutual
  neighbour is reachable from both, and the component puts it in a column by
  its dominant direction.
- **A ring is capped at 40 per side, and the cut is counted.** Distinct callers
  on project `sensei` are 1 at the median, 6 at p90, 60 at p99, and 2,304 for
  `assert_eq`. The most-called are kept, ties broken by id so the cut is stable.
- **Weight is call sites** (`props.occurrences`), not edge rows.
- **A library or another project's symbol is drawn but never walked through.**
  Expanding `serde_json::to_string` would pull in every caller on the machine.
  Re-centring on one is refused with a reason.
- **Callers are this project's.** Scoped through `sensei.folder_projects`, as on
  every sibling screen (#211). A focus from another project is a 404.
- **What the picture cannot show sits beside it, as separate counts that are
  never summed:**
  - cards cut, per side;
  - calls the focus makes that the graph could not place;
  - unplaced calls that use the focus's NAME. This one is a "may": the callers
    column is a floor.

  Both unplaced counts go through `Reason::casts_doubt`, so `plumbing` and
  `external_boundary` verdicts don't count as gaps.

## Signals shown

- The focus: name, package / module, file:line.
- **Neighborhood** cards tinted by package, so a call crossing a crate boundary
  is visible.
- **Coverage lines**, one fact each. If nothing is missing, one line says
  everything was placed and is drawn.
- **Depth** 1 · 2 · 3, which re-asks the daemon.
- **Pick / loading / error / alone / graph**: five distinct states. *Alone* is a
  finding in words, not an empty canvas.

## Done gate

1. `GET …/neighbourhood` with no focus is a **400**. `depth=0` and `depth=4` are
   400s. A focus from another project is a **404**.
2. Depth 2 reaches a callee's callee and depth 1 does not. A library callee is
   drawn and never walked through.
3. Every edge's two ends are nodes the payload carries, and every edge is
   `kind: "dependency"`.
4. The screen opens on the picker with no request made, and the URL's `focus`
   survives a reload.
5. A failed search reads as a failure, not as "no matches".
6. A slow response for an old focus cannot overwrite a newer one.
7. `Neighborhood` MOUNTS against real data without a runtime throw.

## Wrong gate

- **A default symbol.** It looks helpful and answers nothing the reader asked.
- **A capped column with nothing beneath it.** It reads as the whole set.
- **An unplaced count folded into the drawn count.** This turns a gap into a
  relationship.
- **Plumbing counted as doubt.** A well-placed symbol would look uncertain.
- **"No matches" when the daemon is down.** It sends the reader looking for a
  symbol that exists.

## Not in scope

Drilling from Structure into a neighbourhood (the mockup's "Drill down →
neighbourhood" button). It needs a symbol-level selection that Structure, at
file grain, doesn't have.

## Related

- [#220](https://github.com/sensei-hq/sensei/issues/220) — this screen
- [#233](https://github.com/sensei-hq/sensei/issues/233) — the read cache it shares
