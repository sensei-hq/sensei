# Change: Relay ops follow-ups — sentinel, janitor, shared build (#444)

## Why
Design §10.1–10.3 was estimated separately (3–4 days) and deferred past the
autonomous queue. Without it, multi-lane runs can still fill the disk (seen:
168 GB → 17 GB, swap 40/42 GB, a reviewer killed mid-run), leave dead
worktrees and caches behind forever, and stampede the compiler with
duplicate parallel builds.

## What Changes
- **§10.1 Health sentinel.** A 15 s probe (disk free via `statvfs`,
  swap via `sysinfo`) classifies Normal / Warning / Critical by the
  design's watermarks. Warning emits SSE `health_warning` and triggers an
  async janitor run; Critical throttles both claim endpoints with 503
  `relay_host_disk_exhausted`, runs the janitor synchronously, and
  escalates to the person (`open_asked`) when cleanup recovers < 20 GB.
- **§10.2 Janitor.** The four-stage cascade (worktree prune, artifact
  scavenging, manifest clean targets, 7-day log compaction) runs on task
  done/cancelled, hourly, and at the warning watermark.
- **§10.3 Shared build.** Machine-wide `shared-build` resource lease (the
  request endpoint exists from F3); broker-executed builds via
  `POST /api/relay/resources/execute` (manifest + target + worktree,
  streamed output, recorded in `props`); shared `CARGO_TARGET_DIR` /
  `sccache` / pnpm store across worktrees.

## Non-goals
- §10.4 (merge gate) already landed.
- The gemma4 semantic tiebreaker stays deferred (module FOLLOW-UP).
