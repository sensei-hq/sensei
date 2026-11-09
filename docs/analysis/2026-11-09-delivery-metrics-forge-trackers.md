---
name: Delivery metrics with GitHub and GitLab boards
description: Analysis — what the delivery push and the dōjō's board copy need once forge trackers (no history, no points) report sprint boards
date: 2026-11-09
stage: analysis
depends_on:
  - spec/daemon/tracker-delivery-metrics.md
  - spec/daemon/tracker-gitlab.md
  - spec/daemon/tracker-github.md
  - analysis/2026-11-06-forge-trackers.md
next: /sensei:build (A5)
---

# Delivery metrics with GitHub and GitLab boards

The forge trackers (#377, D91, D92) merged into `feature/delivery_metrics`. Both adapters
report `capabilities() = {history: false, writeback: false}`. This analysis checks every
delivery-metrics path against that.

## Works unchanged

| Path | Why it holds |
|---|---|
| `board_key` | hashes `kind + base_url + board_ref`. A forge `board_ref` is a node or numeric id, so two installs agree (C24 holds). |
| Sprint and ticket figures | `sprint_replay.rs` already answers `unsupported_by_tracker` for every history-built figure and for `predictability` (§G8). |
| Reason words in the dōjō | `dojo.delivery_reason_words` reads `sensei.reason_codes`, where `unsupported_by_tracker` is registered. |
| Effort per ticket | comes from `session_task_links`. The forge slices made targets host-aware (`owner/repo#n`, bare `#n`). |
| Dōjō ingest | validates field shapes only: no check on kind, key format or `external_id` format. Sprint states and `seq` fit. |

## Fixed on the branch (2026-11-09)

1. **`pending_tickets` was every forge ticket, forever.** `delivery_token` counted `HISTORY_DUE`
   without checking the adapter's capability. Forge tickets carry `issue_updated_at` and never get
   `history_through`, so the push claimed a history read that never comes. Now a `history: false`
   tracker reports 0 pending (test `a_board_whose_tracker_reads_no_history_has_nothing_pending`).
2. **"Read from Jira" wording** on the Delivery screen and in the facts-filing comment. It now says
   "read from your tracker" (screen spec test asserts no tracker is named).

## Decisions

| # | Decision |
|---|---|
| DF1 | **A board without history orders its copies by sync time** (user, 2026-11-09). The dōjō keeps one copy per board and orders pushes by pending count, then `history_at` (`file_delivery_facts.ddl:89-92`). A forge board's `history_at` is always NULL, so today the last push wins and a stale daemon can overwrite a fresher copy. The facts gain the board's sync age (`synced_age_s`, seconds since `trackers.last_success_at` on one clock; the dōjō stores `synced_at = now() - age`, so no two machines' clocks are compared — user, 2026-11-09, after the build-start audit), sent only by a tracker that reads no history, and the dōjō refuses as `stale` a copy synced earlier than the one it holds when neither has history and both carry a sync time. Jira sends none, so Jira's order is unchanged. Designed as amendment A5 of `spec/daemon/tracker-delivery-metrics.md`. This changes the D4 contract (wire field + one column + ingest check), so it is designed and built as its own slice. |
| DF2 | **Forge tickets from repositories the workspace does not share still travel** (user, 2026-11-09), as Jira's do: the board travels under the workspace's consent. A GitHub project can hold issues from repositories outside the workspace, and forge tickets carry their repository (`scope`), so they could be filtered later. Recorded in `docs/backlog.md`, not built. |
| DF3 | **A copy's `synced_at` is not B11 "sync state"** (user, 2026-11-09; raised by the A5 design review). B11 keeps tracker connections local. `synced_at` is held per copy only to order copies, as `history_at` already is; it names no URL, account or field and nothing can be connected or re-synced from it. |
