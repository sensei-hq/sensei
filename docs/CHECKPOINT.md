# Checkpoint

**Slice:** #146 complete; clean rebuild IN FLIGHT AND BLOCKED. Branch `develop`.
Last commit `85ada17c`.

## IN-FLIGHT — read this first

`make install-service` is running and blocked on its `db-backup` prerequisite,
which is dumping the DB while a throwaway scan writes to it concurrently
(36 MB after 7 min; scan at ~168k nodes). Both will finish; the scan's output is
DISPOSABLE — see why below.

### The sequence that must complete

1. Wait for `make install-service` (it stops the service, overlays binaries,
   codesigns, restarts). Verify: `stat -f '%Sm' /opt/homebrew/opt/sensei/bin/senseid`
   must be TODAY, not `Sep 1 09:40`.
2. `psql -d sensei -c "delete from sensei.folders;"` — the in-flight scan ran on
   the OLD binary, so its graph reproduces the pre-fix bugs. **Run it twice if the
   first fails**: the first attempt today rolled back on a `project_commands`
   trigger and deleted nothing, the retry worked.
3. Re-scan both roots:
   `curl -sX POST localhost:7744/api/scan -H 'Content-Type: application/json' -d '{"root":"/Users/Jerry/Developer"}'`
   and the same for `/Users/Jerry/Work`. Field is `root`, not `path`.
4. THEN measure. `select count(*) from sensei.graph_nodes where locality='unknown'`
   should be near 0 — pre-rebuild it was 56,748.
   `select count(*) from sensei.folders where branch is not null` must be > 0;
   it is the canary that the new binary is live (the old one wrote props instead).

### Why the in-flight scan is disposable

The running daemon binary is dated `Sep 1 09:40`, which predates every fix in
this session. Its rebuild reproduces the old behaviour on a clean slate, so its
numbers measure nothing. (I first reported this binary as `Aug 19` — that was
the symlink's mtime, not the running process's. Conclusion unchanged.)

## Wipe already done, and what survived

`delete from sensei.folders` cascaded nodes/edges/scan_state/communities.
`scan_state` cascading is what prevents a silently EMPTY rebuild — stale
fingerprints would make the incremental gate skip every file.

Kept, verified after the wipe: memories 16, mcp_servers 6, transcript_turns
3,867, sessions 284, projects 146, watch roots 2. Session→folder attribution
(284 rows) is captured at `/tmp/rebuild/session_folder.csv`; the 22
irreplaceable rows at `/tmp/rebuild/irreplaceable.sql` (not needed — projects
was not wiped).

## Done this run

| commit | what |
|---|---|
| `e1114215` | java: resolve calls through the import map (49% of stubs) |
| `1bf668ed` | ts/js: runtime globals to the runtime (39%) |
| `7c7d9d1c` | rust: prelude items and types to std (10%) |
| `4773da89` | stub GC — the missing exit; 84,339 → 56,748 live |
| `95e23315` | branch as a typed column + `graph_nodes.branch` filter dimension |
| `85ada17c` | GC removes the communities it empties (regression from 4773da89) |

Three languages, three DIFFERENT causes. The design sketch's one-root-cause
premise was wrong; profiling each separately is what found them.

## Filed, not started

**#150** content-hash node identity — Jerry's de-dup argument supersedes the
"measured performance drop" trigger I filed it under: 5 repos already have two
checkouts each, ~80% of files identical, so sharing node rows by content hash
SHRINKS storage and scan time. Scope correction needed in the issue: sharing
across checkouts means dropping `folder_id` from node identity, so nodes belong
to a file VERSION, not a folder. Bigger than adding a column.

**#149** label propagation — post-GC, 0-file stub communities went 27,923 → 233,
so 97.3% of live communities are single-file. But cross-file communities are
still 26–35% stub members (those stubs retain in-edges), so re-measure only
AFTER the clean rebuild.

#147, #148, #131–#145 unstarted. Two loose ends never filed: TS/JS local
callbacks (`t` 573, `fn` 189, `setLoading` 235) need a locally-declared-names
pre-pass; and 42 stub parents carry 575 real method nodes.

## Known-broken

- `docs/analysis/graph-end-state-sketch.md` §1–12 NOT SAFE TO BUILD FROM.
- `references`: 250,126 edges, 0 resolved; targets are `/`, `id`, `true`, `429`.
  `doc_indexer.rs:587` joins absolute paths, discarding the repo root.
- `extends`: 7,863 edges, 0 resolved, while `codebase.rs:55` consumes it (#147).
- `dojo_memberships.sync_status` dead — `set_sync_status` has no callers.
