---
title: Delivery copy order for boards without history — plan
description: Build amendment A5 of tracker-delivery-metrics — order a GitHub/GitLab board's dōjō copies by sync age
type: plan
status: built (3c5b40838, 2c1a17046; 2026-11-10)
created: 2026-11-09
depends_on:
  - docs/spec/daemon/tracker-delivery-metrics.md (A5, gates 19–23, claims C25–C39)
  - docs/analysis/2026-11-09-delivery-metrics-forge-trackers.md (DF1, DF3)
---

# Delivery copy order for boards without history — plan

## Objective

Build A5: a tracker that reads no history (GitHub, GitLab) sends `facts.synced_age_s`, the whole
seconds since its last successful sync measured on its database's own clock; the dōjō stores
`synced_at = now() - age` on its clock and keeps the later-synced copy when neither copy has
history and both carry one. Jira sends none and keeps its order. Work goes on
`feature/delivery_metrics`. Builds run with `-j 1` in the background; `make test-template`
after any `database/` change. The spec's "Changes, by surface" table is the file list.

**Before starting,** re-run the A5 claims ledger (C25–C39). Struck-through rows (C34, C38) are
recorded FALSE and already designed out; any other row now `FALSE` stops the build.

## Features

### T1 · Daemon sends `synced_age_s` (gate 19)
- **Layers:** `crates/senseid/src/dojo/delivery.rs`, `crates/senseid/src/db/pg_store/delivery.rs`,
  `crates/senseid/src/tasks/dojo_delivery.rs`, tests in `tasks/dojo_sync.rs` and
  `db/pg_store/tests/delivery_tests.rs`.
- `BoardFacts.synced_age_s`; `facts_json` emits it; `facts_hash` removes it. `delivery_sync_age`
  reads `last_success_at` and its age in one SQL read. Pure `sync_age(history, token, read)`
  skips a board whose read is null or differs from the token; `build_one` uses it.
- **Done:** gate 19, red first.

### T2 · Dōjō stores and orders by it (gates 20–22, 23 sql)
- **Layers:** `database/ddl/table/dojo/delivery_boards.ddl`,
  `database/ddl/function/dojo/file_delivery_facts.ddl`, `database/tests/dojo/delivery_facts.sql`.
- Nullable `synced_at`; an explicit range check (0..315 360 000) raising `invalid_parameter_value`,
  then `now() - make_interval(secs => age)`; rule 3 in the stale predicate; comments state the four
  rules. No cap. `make test-template`.
- **Done:** gates 20, 21, 22 and the sql half of 23, red first.

### T3 · Ingest shape check (gate 23 dojo)
- **Layers:** `dojo/src/lib/server/delivery-ingest.ts` and its spec.
- `DeliveryFacts.synced_age_s`, a whole number in 0..315 360 000 or null; `history_at` checked as a
  parseable timestamp. Both refusals 400.
- **Done:** the dojo half of gate 23, red first.

### T4 · Close
- Wrong-gate (A5) items checked; backlog DF1 entry closed; spec status `built`.
- T1, T2 and T3 touch disjoint files and each lands alone (C29: either side can ship first).
