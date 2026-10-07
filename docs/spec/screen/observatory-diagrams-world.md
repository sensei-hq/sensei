# 界 · Diagrams · World

**Segment:** 04 · Observatory / Project window
**Route:** `/project/[id]/diagrams/world`
**Source mockup:** `docs/mockups/Sensei/screenshots/01-world.png` (and `02-`, `03-`)
**Data:** `sensei.nodes` + `sensei.folder_projects` + `sensei.edges`. **No new indexing.**
**Daemon:** `GET /api/projects/{id}/diagrams/world?groupBy=project|repository|kind` (`3f0eadd6`)
**App files:** `diagrams/world-state.svelte.ts`, `diagrams/world/+page.svelte`
**Rokkit:** `Graph` with `layout: 'world'` (`@rokkit/graph@1.9.0`), driven through a `GraphState`
**Status:** SHIPPED 2026-10-07 (`3f0eadd6` endpoint, `87c770af` screen)

## Purpose

Containment as area: **how much is here, and where it sits.** A circle per
project, repository and code-vs-tests split, sized by declarations held.

Kanji is 界 — *world, boundary*.

## Data invariants

- **CROSS-PROJECT, and the project in the path scopes nothing.** The picture is
  all indexed code and PROJECT is its outermost ring, so filtering to one would
  leave the top level with a single circle. The id is still required and still
  resolved: the payload NAMES which circle the reader is standing in, and a bad
  id must 404 as on every sibling endpoint.
- **Membership through `sensei.folder_projects`, never a folder column** (#211).
  One repository can legitimately belong to two projects and is drawn under both
  — that is what the rings mean — while `totals.repositories` counts it once.
- **Library nodes are excluded.** A `lib·` node names something whose source was
  never opened; counting it as indexed code would inflate every circle by the
  size of its dependency surface.
- **THERE IS NO "DOCS" RING, and the mockup draws one.** `nodes.is_test` is a
  column, so code-vs-tests is a fact. A documentation FILE is not a declaration —
  `sensei.nodes` holds no markdown, and 31,736 files skipped as
  `unsupported_format` are never walked. An always-empty docs ring is a lie the
  picture would tell confidently (#246). Documentation appears where it IS a
  fact: as the documented SHARE.
- **Edges belong to a FOLDER; declarations to a (folder, is_test) CELL.** Joined
  in SQL, a repository's edges would appear on both its code row and its test
  row and sum to double. Two reads, and the nester keeps a folder SET per circle.
- **The unresolved share is `None` below the repository ring**, never 0.0. There
  is no way to say how many of a repository's unresolved edges came from its
  tests; an unshaded circle and a perfectly-resolved one must not look alike. The
  ring at which it becomes a fact MOVES with the grouping.
- **Three groupings are PERMUTATIONS of one set of cells** — one read, one
  statement the planner can check, and the nesting is a pure function.

## Signals shown

- **The circle pack** — `Graph` under `layout: 'world'`, sized by `weight` on a
  log scale, drilled by `focusPath`.
- **A line saying the picture is wider than the project**, first and
  unconditionally — a view of every project otherwise reads as a view of yours.
- **Corpus totals** — declarations, repositories, projects.
- **IN VIEW panel** — the focused circle's label and weight, its three measures
  in words, and what sits DIRECTLY inside it, biggest first.
- **Group first by** project · repository · code·tests. **Shade by** nothing ·
  unresolved · test · documented share, defaulting to `nothing`.
- **Loading / empty / error**, three distinct states.

## Done gate

1. `GET …/diagrams/world?groupBy=project` returns `units` whose outermost ring
   sums to `totals.declarations`, and `viewing` names the project in the path.
2. The picture renders more than one top-level circle on a machine with more than
   one project indexed.
3. `?groupBy=docs` is a **400**, not a silent fall back to the default.
4. Drilling into a circle scopes the panel; drilling out returns; drilling up
   from the root is a no-op.
5. The IN VIEW panel reads "not a fact at this level" for the unresolved share
   below the repository ring, never "0%".
6. Changing the grouping drops a focus built from the old ordering.
7. Exactly three groupings are offered.
8. e2e passes against the live daemon, and `Graph` MOUNTS under the `world`
   layout without a runtime throw.

## Wrong gate

- **A docs ring with nothing in it.** The reader concludes the repository has no
  documentation, which is false.
- **A view of every project that looks like a view of yours.** The widest,
  easiest misreading this screen can produce.
- **A repository's edges counted once per cell.** Halves every unresolved share
  and looks precise doing it.
- **An absent measure printed as 0%.** Says perfectly resolved.
- **Grandchildren in "largest inside".** Double-counts what the list describes,
  because every container's weight already holds them.
- **A focus kept across a grouping change.** Points at a circle that does not
  exist in the new ordering.
- **An empty canvas on a DB error.** On a picture whose subject is how much there
  is, this is the worst of the three states to collapse.

## Not in scope

Indexing documentation as declarations, which would give the third ring the
mockup draws and is an indexer slice with real decisions in it (#246). Edge
drawing between circles — the `world` layout produces no edges, deliberately.

## Related

- [#219](https://github.com/sensei-hq/sensei/issues/219) — this screen
- [#246](https://github.com/sensei-hq/sensei/issues/246) — the missing docs ring
- [#211](https://github.com/sensei-hq/sensei/issues/211) — project membership
- [#233](https://github.com/sensei-hq/sensei/issues/233) — the read cost
