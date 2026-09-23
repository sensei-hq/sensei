---
name: Triggers
description: Every production condition that starts an index cycle, and the scope it starts with
date: 2026-09-23
status: current
---

# Stage 13 — triggers

What STARTS the pipeline. `00-overview.md` describes the pipeline itself;
`14-scenarios.md` gives the executable scenarios.

---

## §1 The complete list

Enumerated from the SIX production enqueue sites, not from memory. The list is
verifiable: `rg 'TaskKind::ScanRoot' crates/senseid/src` outside `#[cfg(test)]`
returns exactly these.

| # | condition | enqueues | where | scope |
|---|---|---|---|---|
| 1 | daemon boot | `ScanRoot` per root | `reconcile_scheduler.rs:77` (boot tick, unconditional) | `Full` |
| 2 | every 300s | `ScanRoot` per root | `reconcile_scheduler.rs:77` (the `reconcile` schedule) | `Full` |
| 3 | watcher stalled ≥30 min | `ScanRoot` per root | `reconcile_scheduler.rs` watchdog | `Full` |
| 4 | **daemon BINARY VERSION changed** | `ScanRoot` per root | `version_rescan.rs:94` | `Full` |
| 5 | explicit scan — `POST /api/scan` | `ScanRoot` for that path | `workspace.rs:627` | `Full` |
| 6 | an exclusion was REMOVED | `ScanRoot` for that root | `workspace.rs:560` | `Full` |
| 7 | FSEvents overflow (`need_rescan`) | `ScanRoot` per affected root | `root_watcher.rs:277` | `Full` |
| 8 | branch switch (`.git/HEAD`) | `ScanRoot` for that REPOSITORY | `root_watcher.rs:277` | `Full` |
| 9 | file changed | `ScanRoot` per watch root | `root_watcher.rs:610` | **`Events`** |
| 10 | crash recovery | `ProcessGitFolder` per non-terminal folder | `resume.rs` | `Full` |
| 11 | explicit re-index — `POST /api/index` | `ProcessGitFolder` for one repo | `workspace.rs:730` | `Full` |

And two conditions that enqueue **nothing**, deliberately:

| condition | what happens instead |
|---|---|
| an exclusion was ADDED | `prune_under_prefix` deletes that subtree immediately. Deleting needs no walk. |
| a root was REMOVED | `folders.root_id … ON DELETE CASCADE` (verified live: `confdeltype = 'c'`) removes its folders, and nodes/edges/files cascade from those. |

**Adding a watch root does NOT itself enqueue a scan.** `add_watch_root`
(`workspace.rs:425`) writes the row and registers the watcher; the scan arrives
as a separate `POST /api/scan` from the client
(`app/src/lib/scan-state.svelte.ts`). This is worth stating because it reads
like it should: the row appears, the watcher starts, and nothing indexes until
the second call.

## §2 Why only one trigger is `Events`

Eight of the eleven are `Full` because they follow an event we did NOT observe:
a restart, an upgrade, a dropped-events signal, a user asking. None of them can
name what changed, so none may treat absence as deletion.

The watcher batch is the single case where something watched the change happen,
and even it is not exhaustive — it saw the paths it was told about and nothing
else. See `14-scenarios.md` §Safety.

## §3 Known gaps

**G1 — removing a watch root does not unregister the live watcher.**
`delete_watch_root` deletes the row; nothing calls `RootWatcher::unregister`
(its only caller is the `health.rs` debug endpoint). The OS-level watch survives
until the thread next restarts. Events for the dead root still arrive and are
then dropped by `process_batch`, which groups by `list_watch_roots()` — so the
index stays correct; the daemon just pays for a watch nobody asked for.

**G2 — `Scope::Full` arrives by default, not by decision.** `resume.rs`,
`version_rescan.rs` and both API handlers build tasks with `Task::new`, which
defaults to `Full`. That is the correct answer for all four, but it is a
default: a future caller gets absence-as-deletion licence by omission.

**G3 — an event scope narrows the fan-out, not the walk.** `repo::discover`
still walks the whole root for `.git` before `narrow` filters. It is `.git`-only
with prunes, so far cheaper than the per-repo file walks it avoids — but the
design says "narrow" and the code says "walk, then filter".
