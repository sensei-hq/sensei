---
name: Implementation Backlog
description: OPEN work — tracked as GitHub issues (sensei-hq/sensei); the issue list is the source of truth
date: 2026-10-13
---

# Implementation Backlog

Work is tracked as **GitHub issues** in
[`sensei-hq/sensei`](https://github.com/sensei-hq/sensei/issues) — the live
issue list is the source of truth. **This file is an index of references, not a
database** (decision 9, docs reorg 2026-10-13): open → referenced here, filed →
an issue, shipped → the issue closes and the row goes away.

The full write-ups of past findings (indexer defects, dbd defects, with their
evidence) are preserved in [`history/backlog/writeups.md`](history/backlog/writeups.md).

## Open issues (as of 2026-10-13)

> Counts and statuses come from the issue list — re-open this table when it
> drifts from GitHub, or drop it and link the filtered query instead.

### Epics
| Issue | Summary |
|-------|---------|
| [#91](https://github.com/sensei-hq/sensei/issues/91) | Dōjō governance track — admin site + policies + preferences + skills/agents |
| [#85](https://github.com/sensei-hq/sensei/issues/85) | Track 3 — Project window (per-screen, separate Tauri window) |

### Observatory · Instruments
| Issue | Summary |
|-------|---------|
| [#96](https://github.com/sensei-hq/sensei/issues/96) | Instruments: background-task visibility — shipped; e2e verify remaining |
| [#90](https://github.com/sensei-hq/sensei/issues/90) | Instruments Replay: usage-verdict classifier — shipped; blocked on Replay screen |
| [#43](https://github.com/sensei-hq/sensei/issues/43) | Observatory Configure section (design + build) |
| [#44](https://github.com/sensei-hq/sensei/issues/44) | J7 Extend & Customize screens (design + build) |
| [#45](https://github.com/sensei-hq/sensei/issues/45) | J5 Pattern knowledge catalog |
| [#46](https://github.com/sensei-hq/sensei/issues/46) | J9 Context-pack tool |

### UI / quality / site
| Issue | Summary |
|-------|---------|
| [#47](https://github.com/sensei-hq/sensei/issues/47) | Mockup component migration to Rokkit (steps 4–14) |
| [#81](https://github.com/sensei-hq/sensei/issues/81) | Update website `/sensei` route to the new mockup |

### Foundation / deferred
| Issue | Summary |
|-------|---------|
| [#39](https://github.com/sensei-hq/sensei/issues/39) | Bootstrap diagnostic logging + debug mode |
| [#50](https://github.com/sensei-hq/sensei/issues/50) | Extract `bootstrap` into a reusable library (deferred) |

## Open — not yet filed (file as GitHub issues)

Full one-liners and prior context: [`history/backlog/writeups.md`](history/backlog/writeups.md).

- **Library capability provisioning + version-aware doc queries** — nothing
  installs a library's declared skills/agents; `get_lib_docs` lacks a `version`
  override.
- **Metrics engine rebuild + Dōjō sync (v1 = local; sync rides Phase 2)** —
  repo-grain watermark engine; current spec draft at `spec/2026-08-18-…`.
- **Operating-model Phase 2 (front-door intake conversation + playbook catalog).**
- **Codebase-wide silent-error audit** (directed after the `node_kind` drop) → roadmap gaps `F-*`.
- **Stale / orphaned project cleanup** — list endpoints + gated purge + housekeeping UI.
- **Activity-data GC** — TTL prune after analysis has derived insights.

## Where open work is described

| Concern | Source |
|---|---|
| Issue-level work | this file (above) → GitHub |
| Per-module status and the next step | [`roadmap/`](roadmap/README.md) |
| Every known gap, IDed, with evidence | [`roadmap/06-gaps.md`](roadmap/06-gaps.md) |
| Detailed past findings | [`history/backlog/writeups.md`](history/backlog/writeups.md) |