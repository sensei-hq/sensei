# Change: Indexer storm hardening (compile governor, stale prune, phase timing, list caps)

## Why

The v0.26.0 large-root run (~/Developer 258G + ~/Work 20G, 308k files,
209 repos) measured four defects live:

- Five concurrent ts-driver compiles on tsconfig-less packages pegged the
  machine (load 31 on 16 cores) for 17+ minutes each; the daemon starved
  at 17% CPU and small files took 80s+. No global cap, no fail-fast
  without a tsconfig, and the same package compiled twice at once.
- Every failure is a deleted path retried 3x (`detect_communities` /
  `scan_git_history` "Folder not found"), flooding 240 alerts. Folder
  rows outlive their directories; nothing prunes them.
- `duration_ms` cannot distinguish parse from compile-wait from resolve
  from write — the 82s/9KB file is unattributable without a profiler.
- The operations center renders unbounded lists into a 30k-task storm.

## What Changes

- Compile governor: a global semaphore caps concurrent compiler
  subprocesses (2); same-root compiles share one flight (no duplicates).
  Failing compiles keep today's `engine_not_run` semantics for their
  files. (A tsconfig-less fast-skip was tried and reverted: working
  packages that compile fine without one regressed to unresolved. The
  per-package grind is a ts-driver hang class, filed as follow-up.)
- Stale prune: folder-scoped handlers stat their path first; a vanished
  directory deletes its folder tree (cascades) and skips Ok — no retry,
  no failure, no alert. Re-created paths re-enter through the normal
  discovery walk.
- Phase timing: `process_file` records parse / compile-wait / resolve /
  write splits through the task context; the executor persists them to a
  new nullable `task_executions.phases_ms` JSONB column (one additive
  column; additive pre-release DDL, no migration). Debug spans mirror
  the splits for live debugging.
- List caps: ops center lists render at most 200 rows with an explicit
  "showing X of Y" overflow row, mirroring the backend's capped-lists
  contract. Totals never lie.
- Clear redelivery: uncleared drops persist in a pending set and flush
  every refresh until written (per-PUT timeout, in-flight guard, ack
  cancel, generation-dropped stale responses) — a single failed write
  no longer loses the recurrence memory forever (exposed by 4.4).

## Non-Goals (measured)

- No C++ adapter: 86,026 of 86,038 C++ files on this machine are
  vendored archives (envoy checkout, R packrat libs, grpc subtree);
  first-party C++ here is ~zero and the walk already skips unclaimed
  extensions (zero C++ tasks attempted, ever). `.h` stays with the C
  adapter (file-only nodes for C++ headers — observed, accepted).
- No `vendor/` exclusion: zero corpus files outside already-pruned
  `node_modules`.
- No throughput redesign (worker count, batch writes): measure again
  after the governor lands; the storm above explains today's tail.
