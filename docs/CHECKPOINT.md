# Checkpoint

**Slice:** clean rebuild DONE (the install blocking `ccd5517e` completed). Branch
`develop`, last commit `ccd5517e`. Rebuilt on the new binary: 7,642 folders /
408,969 nodes / 731,370 edges / 46,512 files. Branch canary POSITIVE — 84
folders, 9 branches. internal 344,001 (84.1%) · unknown 54,594 (13.3%) ·
external 10,374 (2.5%).

## The headline invariant did NOT hold

`ccd5517e` predicted `unknown` "near 0". It went **56,748 → 54,594** (3.8%) — the
three language fixes did not clear the stubs, so do not plan on that premise. Of
the 54,594: **44,689 have in-edges** (need RESOLUTION, not GC), **9,863 match the
GC predicate yet survive**, 42 are the known stub parents. Still java-dominated:
java 27,967 · ts 10,969 · js 10,409 · rust 4,306.

## Root-caused: the GC runs before the thing that creates its work

The 9,863 are an ORDERING bug, attributed 100%. Per-folder stub GC fires at the
community terminal barrier (14:12–14:15); `scan_root reconcile` then runs
`dedup_structural_folder_nodes` (`graph.rs:1830`) at 14:19:18, deleting 36,032
duplicates. That cascade strips the in-edges that made those stubs ineligible,
and nothing GCs again. All 9,863 sit in `kind='folder'` folders — the dedup's
exact target set — across 4 (cluster:scheduler 4,852 · server 4,526 · external
469 · web-portal 16). Zero in any git/standalone/subtree folder.

**Next:** red-first — a test that dedups a structural folder and asserts no
orphan stub survives the reconcile, then call `prune_orphan_stubs` after
`dedup_structural_folder_nodes`. Fix the ordering, not the GC.

## Known-broken

- **`imports`: 0 of 136,532 resolved — NEW.** 25,692 are LOCAL and must resolve
  (relative 15,924 · alias 8,618 · `crate::` 1,150); 110,840 external, correctly
  unresolved. Detection reads calls+imports+extends+references
  (`community.rs:185`) and three are 0% — hence 97.8% single-file communities
  (#149: cross-file stub share 20.3%, was 26–35%).
- `references` 251,185 / 0 (`doc_indexer.rs:584` and `:587` — two separate
  defects, see commit msg). `extends` 7,901 / 0 (#147). `calls` 65.1%, the only
  working kind. `dojo_memberships.sync_status` dead.
  `graph-end-state-sketch.md` §1–12 NOT SAFE TO BUILD FROM.

## Environment state

Nothing is left disabled: `sensei.metrics` was temporarily retired to stop a
boot-time metric wave starving the scan, then **restored and verified** (9 active
task names, 29 rows, 2 retired). Daemon is up under brew services on the new
binary. Rebuild traps worth knowing before the next one are in this commit's
message (cold-graph queue starvation `queue.rs:479`, wipe needs the daemon
stopped, `/health` not `/api/health`).
