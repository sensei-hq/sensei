# Capture & Sessions · M3

**Plane:** foundation · **Status:** [roadmap/01-foundation.md](../../../../roadmap/01-foundation.md) ·
**Serves:** solo developer ([persona 1](../../../03-personas.md#1--solo-developer--the-wedge)), team developer ([persona 2](../../../03-personas.md#2--team-developer--the-inheritor))

## Purpose

Capture is the front door of the whole system — nothing sensei knows about the developer's
work exists without it. Four paths feed the pipeline: a one-shot filesystem scan that turns
real folders into repos and multi-repo projects; a continuous root-watcher that keeps the
graph current without manual re-scans; git branch-switch handling that flips the view to the
checked-out branch; and assistant-event capture that turns every session into a stream of
events (session start, prompt, tool call, tool result, session end). Sessions materialise
from those events and every downstream signal — FTR, insights, memories — falls out of them.
The user's experience: *point sensei at a folder once and it just keeps knowing*.

## In scope / out of scope

- In: root-based folder/project scan (incremental, content-hash); multi-repo project
  detection + suggestion + merge/split; the root-watcher with restart reconciliation; branch
  versioning (active-only default, copy-on-switch); assistant-event capture via per-family
  adapters (Claude Code live, Zed read-only, custom via MCP `log_event`); event
  secret-redaction; session materialisation on the analyzer tick.
- Out: the code-graph *indexing* semantics (M4 — capture *finds and queues*; M4 *parses and
  resolves*); FTR computation and memory/insight generation (later planes); dojo
  auto-discover join (T1); the preferences surface for roots (M2).

## Surfaces

- **API** — `POST /api/scan/roots`, `GET /api/projects/suggestions`, `POST /api/projects/merge`
  (+ split), `crates/senseid/src/api/handlers/sessions.rs::materialise_session`,
  `crates/senseid/src/api/handlers/sessions.rs` (`ingest_hook_event`, `hook_gate`,
  `hook_nudge` — the `/hook/*` event ingest), MCP `log_event`.
- **Daemon** — scanner (`crates/senseid/src/indexer/scan_repo.rs`,
  `crates/senseid/src/tasks/handlers/scan.rs`), watcher (`watcher/root_watcher.rs`).
- **Adapters** — `crates/senseid/src/adapters.rs`'s trait with `adapters/{config,manifest}`
  children; Claude Code hook plugin at `marketplace/plugins/sensei/hooks/`.
- **App** — Observatory project list shows discovered projects and multi-repo suggestions;
  Sessions list reads `activity.sessions`.

## Data invariants

- `sensei.folders_to_watch` holds the user's watch roots; a folder whose `content_hash` is
  unchanged since the last scan is skipped end-to-end (incremental).
- `sensei.folders` — one row per discovered repo/folder under a root
  (`path, name, kind, project_id?, scan_state, content_hash, stack, manifest_summary`);
  `sensei.projects` is the logical grouping — a folder reaches its project only through the
  junction (`folders.repository_id → repositories_in_projects → projects`), never a column of
  its own.
- Watch roots have **one watcher per active root**; file events are debounced (default
  ~500ms) and coalesced (a bulk checkout collapses to one reconciliation pass).
- Restart-safety: on boot the watcher compares each root against the last-persisted content
  hashes; any drift becomes a reconciliation pass.
- Branch versioning: `sensei.nodes`/`sensei.edges` carry a `branch` column; the active-branch
  view is a filter; switch = copy-on-switch (reuse unchanged rows with the new branch tag,
  enqueue adds, retract removes from the active view, re-parse modified).
- Assistant events are the atoms: `activity.assistant_events` — one row per event with
  `client_session_id, assistant_family, assistant_id, event_kind, tool_name, ts_ms, payload,
  duration_ms?, error?`. No aggregation at capture time.
- Session materialisation: every session with hook events gets an `activity.sessions` row
  after the next analyzer tick, with `analyzed_at`, `ftr`, `corrections` after enrichment.
- Session-id: `activity.sessions.id` is a sensei UUID; `assistant_events.session_id` is the
  adapter's client-side id; every join resolves through `sessions.client_session_id`.
  (Regression trap, explicitly.)
- **No PII/secrets egress**: payloads are stored locally only; payloads matching a secret
  pattern list are redacted at capture with a logged redaction.

## Signals shown

| Signal | Source | Consumer |
|---|---|---|
| Discovered repo/project rows | scan | every screen; Project window Overview |
| Multi-repo suggestion card | scan analyzer | Observatory Projects pane |
| `activity.assistant_events` rows | adapters / hooks | analyzer materialisation |
| `activity.sessions` rows (ftr, corrections, analyzed_at) | materialisation | FTR, Sessions, replay |
| Branch indicator ("main / feat-x") | branch scan-mode | code-graph + impact views |

## Done gate

- A fresh scan of `~/Developer` + `~/Work` turns every discovered repo into a
  `sensei.folders` row within the incremental scan window; nested roots do not get
  double-indexed (outer subsumes).
- Multi-repo groups appear as proposal cards; accept merges via `POST /api/projects/merge`;
  dismiss is remembered for the 7-day staleness window.
- Every Claude Code session with the plugin installed produces `assistant_events` rows
  continuously (not batched at session end).
- Every session resolves by either the observatory UUID or the `client_session_id` — both
  return the same session.
- A change that landed while the daemon was down is discovered on next boot with no user
  action (restart reconciliation).
- A prompt payload containing a secret never lands verbatim in `assistant_events.payload`.

```bash
# events flowing in the last minute?
psql -A -t -c "select count(*) from activity.assistant_events
               where ts_ms > (extract(epoch from now()) - 60) * 1000" -d sensei
# suggestions pending?
curl -s http://localhost:7744/api/projects/suggestions | jq '.suggestions'
# id resolution consistent?
curl -s http://localhost:7744/api/sessions/$UUID | jq '.client_session_id'
curl -s http://localhost:7744/api/sessions/$CLIENT_ID | jq '.id'   # same session
```

## Wrong gate

- **Every folder becomes its own project** — multi-repo detection never fires.
- **Suggestion cards re-appear immediately after dismiss** — no staleness persistence.
- **Events arrive but sessions never materialise** — analyzer tick or the client-session
  join is broken.
- **`assistant_events` grows past retention with no pruning** — the append-only stream slows
  the scanner.
- **A file added while the daemon was down is invisible until the periodic tick** — restart
  reconciliation missing.
- **Branch switch leaves the old branch's rows in the active view** — copy-on-switch didn't
  fire or the branch tag/filter is broken.
- **Two adapters collide on `client_session_id`** — id namespace must be `{family}:{raw_id}`.
- **A secret-bearing prompt persisted verbatim** — redaction pattern list missed it.
- **Merging two projects loses FTR history** — session project_id attribution must survive a merge.

## Scenarios

- [`scenarios/capture.feature`](scenarios/capture.feature) — scan, watcher, branch flips,
  event capture, materialisation, redaction.

## Open gaps

- [C-11](../../../../roadmap/06-gaps.md) — transcript-scanning review not scoped as a task (P1).
- [C-13](../../../../roadmap/06-gaps.md) — Claude "mods" supersede plugin + hooks; install path
  and migration not verified (P1).
- [C-14](../../../../roadmap/06-gaps.md) — multi-assistant expansion: Claude + OpenCode first,
  others later; the extension seam is not specified (P1).
- Branch scan-mode UI (`active-only` / `pinned` / `all`) is a spec gap; deferred to the
  Preferences Folders pane.

## Related

- [spec/pipeline/capture](../../../../spec/pipeline/capture.md) — the four paths, branch
  versioning, adapter trait, locked decisions (legacy depth, folds here).
- [features/04-project.md](../../../../features/04-project.md) — project grouping story.
- [prd/01-vision.md §The core loop](../../../01-vision.md#the-core-loop) —
  [prd/02-objectives.md](../../../02-objectives.md) §03 Observatory.
- Crate: `crates/senseid/src/{scan,watcher,adapters}`; memories: project_incremental_indexing,
  feedback_no_silent_errors, project_stale_folder_reconcile.