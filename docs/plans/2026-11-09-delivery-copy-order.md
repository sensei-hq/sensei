---
title: Delivery copy order for boards without history — plan
description: Build amendment A5 of tracker-delivery-metrics — order a GitHub/GitLab board's dōjō copies by sync time
type: plan
status: plan
created: 2026-11-09
depends_on:
  - docs/spec/daemon/tracker-delivery-metrics.md (A5, gates 19–23, claims C25–C37)
  - docs/analysis/2026-11-09-delivery-metrics-forge-trackers.md (DF1, DF3)
---

# Delivery copy order for boards without history — plan

## Objective

Build A5: a tracker that reads no history (GitHub, GitLab) sends `facts.synced_at`; the dōjō
keeps the later-synced copy when neither copy has history and both carry one, and caps
`synced_at` at its own clock. Jira sends none and keeps its order. Work goes on
`feature/delivery_metrics`. Builds run with `-j 1` in the background; `make test-template`
after any `database/` change.

**Before starting,** re-run the A5 claims ledger (C25–C37). Any `FALSE` stops the build.

## Features

### T1 · Daemon sends `synced_at` (gate 19)
- **Layers:** `crates/senseid/src/dojo/delivery.rs`, `crates/senseid/src/tasks/dojo_delivery.rs`,
  tests in `crates/senseid/src/tasks/dojo_sync.rs`.
- `BoardFacts.synced_at`; `facts_json` emits it; `facts_hash` removes it. `build_one` skips a
  board whose token reads `last_success_at` null; sets `synced_at` only for a `history: false`
  adapter.
- **Done:** gate 19, red first.

### T2 · Dōjō stores and orders by it (gates 20–22, 23 sql)
- **Layers:** `database/ddl/table/dojo/delivery_boards.ddl`,
  `database/ddl/function/dojo/file_delivery_facts.ddl`, `database/tests/dojo/delivery_facts.sql`.
- Nullable column; `least(synced_at, now())`; rule 3 in the stale predicate; comments state the
  four rules. `make test-template`.
- **Done:** gates 20, 21, 22 and the clamp half of 23, red first.

### T3 · Ingest shape check (gate 23 dojo)
- **Layers:** `dojo/src/lib/server/delivery-ingest.ts` and its spec.
- `DeliveryFacts.synced_at`; one shared timestamp check for `history_at` and `synced_at`
  (400 for a non-string or an unparseable string).
- **Done:** the dojo half of gate 23, red first.

### T4 · Close
- Wrong-gate (A5) items checked; backlog DF1 entry closed; spec status `built`.
- Independent of order: T1, T2 and T3 touch disjoint files and each lands alone (C29: either
  side can ship first).
