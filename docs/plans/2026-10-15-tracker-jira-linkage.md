---
title: Jira tracker linkage (local) — plan
description: Decompose the tracker-jira-linkage spec into ordered, testable features
type: plan
status: plan
created: 2026-10-15
depends_on:
  - docs/spec/daemon/tracker-jira-linkage.md
  - docs/analysis/2026-10-15-jira-ticket-linkage.md
related_issues: []   # TBD — gh CLI not installed on this machine; create manually
references:
  - docs/prd/modules/04-team/03-dojo-project-planning/README.md
  - docs/design/integrations-and-gating-requirements.md
milestone:
---

# Jira tracker linkage (local) — plan

## Objective

Implement `docs/spec/daemon/tracker-jira-linkage.md` (R1–R10). Section and gate
numbers below refer to that spec. Layers here are **DDL → daemon (pg_store +
`tracker/`) → API → CLI**. No MCP/hooks/commands layer is touched: the read API and
CLI are the v1 contract.

## Prerequisites (block build, not planning)

- **P0 — sessions exist on Linux.** A separate slice on `feature/linux_support`.
  The spec's P0 SQL must return `t`. It does **not** block F1–F4 unit work (they
  use seeded test rows); it blocks F5.
- **Jira tenant (human).** The user provides a Jira Cloud site, board id and API
  token. Without them F1 halts at task F1.1.

## Features

### Feature 1: Connect + read-only sync

- **Issue:** TBD
- **Layers:** DDL (`trackers`, `tracker_projects`, `sprints`, `tracker_tasks`, `tracker_task_sprints`; enums `tracker_kind`, `tracker_error`) → daemon (`http::bounded_builder()` extracted from `federation::http_client()`; `tracker::jira` transport + URL validation + `sync`; credential via `gateway_keys`; R9 status fn; `api/origin_guard.rs` extracted from `repositories.rs` plus the Host check) → API (`GET …/trackers`, `POST …/jira/connect`, `DELETE …/jira`, `POST …/jira/sync`) → CLI (`sensei tracker connect|sync|disconnect`)
- **Depends on:** — (task F1.1 needs the tenant)
- **Tasks (in order):**
  1. **F1.1 (human-gated):** record one scrubbed response per R4 endpoint into `crates/senseid/tests/fixtures/jira/` and confirm the C11 field names. Update the spec's C11 row.
  2. DDL + enums; `dbd` deploy against the local DB.
  3. `http::bounded_builder()` extraction (federation behaviour unchanged).
  4. `origin_guard.rs` extraction; `repositories.rs` switches to it; Host check added.
  5. Transport, URL validation, sync, status, and credential, all TDD against the fake transport.
  6. Routes + `local_caller_guard`; CLI.
- **Acceptance criteria:** Done gates 1b, 1c, 1d, 1e, 1f, 1g, 1h, 1i, 2, 3 and 5 pass, plus the binding parts of 1k (same board → same tracker, `project_already_bound`, unbind/last-unbind, orphan cleanup).
- **Test scenarios:**
  - Given a fake transport with 2 issue pages and 2 sprints, When sync runs twice, Then the second run changes nothing but `last_seen_at`.
  - Given page 2 returns 500, When sync runs, Then no data row changes and `status` = `error`.
  - Given `base_url = https://evil.com`, When connect is called, Then 422 `invalid_base_url` and no row exists.
  - Given `Origin: https://evil.com`, When any tracker route is called, Then 403.

### Feature 2: Scheduled sync

- **Issue:** TBD
- **Layers:** data (`schedules.jsonl` row) → daemon (`tasks/tracker_sync.rs` `tick` + `spawn`; `SCHEDULABLE`; `DESCRIPTIONS`; `server.rs` spawn call)
- **Depends on:** F1
- **Acceptance criteria:** Done gate 4 passes. One tracker's failure in a tick leaves the others synced (unit test with two fake trackers, one failing).
- **Test scenarios:**
  - Given two trackers where A's transport fails, When a tick runs, Then B has `last_success_at` set and A has `last_error` set.
  - Given a manual sync holds the advisory lock, When a tick runs, Then that tracker is skipped and not marked as an error.

### Feature 3: Session ↔ ticket linking

- **Issue:** TBD
- **Layers:** DDL (`session_task_links`, enum `link_source`) → daemon (pure `extract_links`; candidate query via `folder_projects` on `repo_folder_id`; relink inside the sync transaction) → API (`GET …/tracker-tasks/{key}` with its sessions)
- **Depends on:** F1
- **Acceptance criteria:** Done gate 1a passes. Relink is idempotent: syncing twice gives identical link rows. A link to a removed task survives the next sync. A session from a repo outside the project gets no link (seeded-DB test).
- **Test scenarios:**
  - Given known keys {AUTH-420, MY_PROJ-12}, When a branch is `feature_AUTH-420` and a prompt says `MY_PROJ-12 and UTF-8`, Then links = AUTH-420/branch and MY_PROJ-12/prompt only.
  - Given AUTH-420 disappears from Jira, When sync runs, Then `removed_at` is set and its links are still present.

### Feature 4: Usage metrics + read routes + CLI usage

- **Issue:** TBD
- **Layers:** daemon (R7 usage reads in `pg_store`) → API (`GET …/sprints`, `GET …/tracker-tasks`, `GET …/sessions/unlinked`, usage on `…/tracker-tasks/{key}`) → CLI (`sensei tracker usage`)
- **Depends on:** F3
- **Acceptance criteria:** Done gates 1j and the usage parts of 1k pass (a ticket worked on across two bound projects sums both, with a `by_project` breakdown; a repo in two bound projects is counted once in the total). Every usage field carries its `_n`. A sum over zero non-NULL values is `null`. `tokens_in` appears in no response (`rg tokens_in crates/senseid/src/api/handlers/trackers.rs` → 0). CLI output labels time as "active AI time".
- **Test scenarios:**
  - Given 3 sessions linked to AUTH-420, one with NULL `duration`, When ticket usage is read, Then `active_n = 2` and `sessions = 3`.
  - Given a session linked to AUTH-420 and AUTH-421 (both in sprint S), When S's usage is read, Then the session counts once in `linked_sprint` and `shared = 1`.
  - Given a future sprint, When `/sprints` is read, Then its usage is `null` with reason `not_started`.

### Feature 5: Live verification (human)

- **Issue:** TBD
- **Layers:** none (verification only)
- **Depends on:** F1–F4, P0, Jira tenant
- **Acceptance criteria:** Done gates 6–10 pass, and the spec's Evidence table is filled in (hand-SQL equality and per-source precision over ≥10 real sessions).

## Dependency graph

```mermaid
graph LR
  T[Jira tenant - human] --> F1
  F1 --> F2
  F1 --> F3 --> F4
  P0[P0 Linux sessions - separate slice] --> F5
  F2 --> F5
  F4 --> F5
```

First task: **F1.1** — needs the user's Jira Cloud site, board id and API token.
F1.2–F1.5 can start before that against a hand-written fake that follows the
spec's R4 shapes; F1.1 then reconciles the fake with the real fixture.
