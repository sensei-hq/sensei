# Change: Dōjō diagram screens over the shared code graph (#268)

## Why
Each repository's code graph syncs to the dōjō (`dojo.graph_*`, one per
tracked branch) and nothing reads it there. Teams cannot see structure,
layers, zones, world, neighbourhood, or schema for the code they own —
the app's diagram views stop at the machine boundary.

## What Changes
- **Reads.** `dojo/src/lib/server/graph-reads.ts`: nodes/edges/files/
  schema for one (repository, branch) under the existing RLS
  (`dojo.can_read_repository`). Edge targets and schema refs resolve
  at read time against the same branch's nodes; an unresolved key
  shows as external, never dropped.
- **Port.** A dōjō `CodeMapPort` (`graph-port.ts`) answering the exact
  `@sensei/codemap` wire shapes: L0 (structure/layers), module (L1),
  file (L2), zones (Martin metrics from edges). Cochange answers
  `commits: 0` (no history is synced — honest, not empty-as-failure);
  complexity answers `ok: false` (cyclomatic Measures are not synced).
- **Screens.** A repo diagram route (structure default, lens switcher,
  `CodeMap` + `CodeMapSide`), a branch picker over
  `my_graph_branches` defaulting to `is_default`, and team scoping
  (tenant → team → projects) for the team world view.
- **Tracked branches.** An authority-gated screen to add/remove tracked
  branches through `dojo.set_graph_branch`.
- **Schema view.** The ER canvas over `graph_schema_*` for the same
  (repository, branch).

## Non-goals
- Syncing complexity Measures or commit history to the dōjō (the port
  says what it cannot read; follow-up issues if wanted).
- The per-repo/per-workspace metric screens (#532/#533 build on these
  reads but are separate issues).
- Mobile layout fixes (separate issues; these screens follow the
  shell contract from the start).
