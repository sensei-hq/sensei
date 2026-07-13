# Design: Rock-Solid Code Indexing (watcher-independent convergence)

- **Status:** P0 in progress (subagent, develop); P1/P2 queued.
- **Date:** 2026-07-13
- **Owner:** capture/indexing (daemon `senseid`)
- **Prompted by:** a live drift incident this session + Jerry's question "how do we make it rock solid?"

## 1. Problem — the index silently drifted from the filesystem

While the daemon was running normally, the code index diverged from disk and **nothing surfaced it**. Concrete evidence from 2026-07-13:

- The fs-watcher had not committed a scan in ~5h. A `hive-mind → dojo-mind` crate rename/move was entirely missed.
- `sensei` "lost" ~5,000 nodes (16,354 → 11,181) because the moved sub-crate was re-scoped into a **phantom standalone project**, so `search`/graph for `sensei` silently missed all of `dojo-mind`.
- The deleted `crates/hive-mind/` directory left **137 orphan nodes across 3 ghost folder rows** that no reconcile path touched.
- Every MCP/API query kept returning confident answers off the stale index — **no error, no warning, no staleness signal**.

The immediate bugs were fixed this session (`prune_vanished` for files, `prune_vanished_folders` for deleted/moved dirs, `heal_nested_standalone_roots` for phantom projects, plus a boot+hourly `reconcile_scheduler`). Post-fix the index self-heals and is accurate again (verified: ghosts swept to 0, `dojo-mind` re-attached, this session's own new symbols indexed within ~2 min).

But the heal is **eventually-consistent on an hourly cadence and rests on a watcher that already proved it can freeze silently.** That is not rock-solid. This doc defines what is.

## 2. Why the current design isn't rock-solid

The index's correctness currently depends on the fs-watcher catching every relevant event. It won't, because:

1. **FSEvents is lossy + resettable.** macOS coalesces and can drop events; a daemon restart resets the stream and events during the gap are lost. There is no persisted stream cursor, so a restart cannot replay what it missed.
2. **The safety-net reconcile is too slow + skippable.** `reconcile_scheduler` runs hourly and is watermark-gated (`reconcile.last_run`); this session it **skipped on boot** because the watermark was recent, so a restart could leave drift until the next hourly tick. Worst-case staleness = ~1h.
3. **Change-detection reads every file.** The scan_state diff hashes file content to decide "changed", so a full reconcile is expensive — which is *why* it only runs hourly. Cost forces infrequency; infrequency permits drift.
4. **Failure is silent.** When the watcher stalls, the index just rots. There is no liveness signal, no drift metric, no alert.
5. **Only known drift classes are healed.** The heals added this session are point fixes (vanished file, vanished dir, phantom project). A new drift class (some future rename/move pattern) would again go uncaught until a human notices.

## 3. Design principle

> **Real-time watcher events are a latency optimization, never the source of truth. The index must converge to the filesystem through a cheap, frequent, self-correcting reconcile. The watcher only makes convergence *fast*. A watchdog + invariant-audit make any failure *loud* and *self-healing*.**

Correctness must not depend on any single event being caught. If every watcher event were dropped, the index must still be correct within one reconcile interval.

## 4. Architecture — layered convergence

```
  filesystem  ──(FSEvents, best-effort, low-latency)──►  watcher  ─┐
       │                                                            ├─► index
       └──(cheap mtime-gated reconcile, every ~2-5min + on boot)───┘
                         ▲                              ▲
                    watchdog (liveness)          invariant self-audit
                 forces reconcile on stall     repairs unknown drift classes
```

- The **reconcile** is the correctness backbone: guaranteed convergence, bounded staleness.
- The **watcher** is the latency layer: sub-second updates when it works, irrelevant to correctness when it doesn't.
- The **watchdog** + **invariant audit** are the assurance layer: make stalls visible and repair drift the point-fixes don't know about.

## 5. Work tiers

### P0 — make correctness independent of the watcher  *(in progress)*
The single highest-leverage change: make a *no-op* reconcile near-free so it can run constantly + on every boot.

- **mtime fast-path in `process_git_folder` change-detection.** `sensei.scan_state` already stores `mtime` (bigint) — **no DDL needed**. Stat each file's mtime vs the stored value; if equal → skip reading+hashing+reindexing. Only re-hash files whose mtime moved (content-hash stays the source of truth for "did content change" — mtime is only a cheap gate to avoid hashing unchanged files). Use **directory mtime to skip whole unchanged subtrees** so the walk is O(changed), not O(all files).
- **Frequent + always-on-boot reconcile.** With a no-op pass now cheap, change `reconcile_scheduler` to run every ~120–300s (configurable via `reconcile.interval_secs`) **and unconditionally on boot** — the cheap mtime pass must not be watermark-gated (a restart must never leave drift). Reserve the watermark only for any genuinely-expensive work.
- **Anchors:** `db/pg_store.rs` (`upsert_scan_state`/`list_scan_state`, `scan_state.mtime`), `tasks/handlers/process.rs` (the diff), `tasks/reconcile_scheduler.rs`.
- **Done-gate:** worst-case staleness ≤ the reconcile interval regardless of the watcher; a no-op scan performs zero file reads/hashes (proven by test); boot always reconciles even with a fresh watermark; all existing scan/process/reconcile tests stay green; no DDL.
- **Possible DDL:** only if mtime-only proves insufficient (same-mtime, different-size) — then propose adding `scan_state.size`; do not add unattended.

### P1 — kill silent failure + fix the watcher's root cause
- **Watcher liveness + watchdog.** Watcher heartbeats `watcher.last_event_at`; a watchdog that sees no events *and* no reconcile in N minutes, or an FSEvents error, logs a WARN and forces an immediate reconcile. Surface watcher state + last-reconcile + drift count in the observatory so staleness is visible, never silent.
- **FSEvents overflow/drop handling.** On kernel/user-dropped or a `notify` Rescan signal, immediately full-reconcile the affected root. Debounce bursts (a `git checkout` touching 1000s of files) into one subtree reconcile; never let the event buffer overflow-and-drop silently.
- **Persist the FSEvents cursor.** Store the FSEvents `lastEventId`; on boot resume the stream from it, and if the kernel can't replay that far (`HistoryDone`), fall back to a reconcile of that root. (Likely needs dropping below the `notify` crate to raw FSEvents for `sinceWhen` — spike first.)
- **Watch `.git/HEAD` + refs.** Branch switches / rebases / big moves are where drift is born (this session's `hive→dojo` move is the archetype). `version_rescan` partially does this; make a HEAD change *always* force a repo reconcile.

### P2 — catch drift classes we haven't hit yet + make it provable
- **Continuous invariant self-audit.** Generalize the point-fixes into a periodic checker: every node's file exists; every file's folder is under a live root; no ghost folder; no standalone project nested in a repo. Violation → auto-repair + log. New drift classes get caught automatically.
- **`sensei index doctor`.** A read-only command reporting drift (indexed-but-gone / on-disk-but-unindexed / mis-scoped) without fixing — for confidence and CI.
- **Chaos test.** A test harness that injects FSEvents drops, mid-scan restarts, and rapid moves, then asserts the index converges. This session proved unit-green ≠ live-correct; this closes that gap.

## 6. Non-goals / open questions

- Not replacing content-hash as the correctness anchor — mtime is only a cheap gate.
- Reconcile interval default (120s? 300s?) — pick empirically against a large tree once the dir-mtime subtree-skip is in; must stay cheap enough that a no-op pass is unnoticeable.
- Whether `notify` can expose the FSEvents `sinceWhen` cursor or P1 needs a raw-FSEvents path — spike in P1.

## 7. Status / sequencing

1. ✅ Immediate drift fixed this session (prune file/dir, phantom heal, boot+hourly reconcile) — shipped + live-verified on develop.
2. 🔨 **P0 in progress** (subagent, develop) — mtime fast-path + frequent/always-on-boot reconcile. No DDL.
3. ⏳ P1, then P2 — sequential reliability chunks after P0 verifies.

Ordered after R8 (shipped `310c3477`) and the CI patch release (v0.2.44) per Jerry's direction. Progress tracked in `docs/llm-spec/park/_run-state.md`.
