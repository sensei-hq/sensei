# 捉 · Pipeline · Capture

**Owner files:**
- Scanner (one-shot): `crates/senseid/src/scan/`
- Watcher (continuous): `crates/senseid/src/watcher/root_watcher.rs`
- Branch-switch handler: `crates/senseid/src/scan/branch.rs`
- Hook capture: `crates/senseid/src/api/handlers/hooks.rs` +
  `crates/senseid/src/db/pg_store.rs::insert_assistant_event`
- Session materialisation:
  `crates/senseid/src/api/handlers/sessions.rs::materialise_session`
- Adapter — Claude Code: `marketplace/plugins/sensei/hooks/`
- Adapter — Zed (read-only): `crates/senseid/src/adapters/zed/`
- Adapter — future (cursor, cline, aider, …): `crates/senseid/src/adapters/{name}/` following the trait in `adapters/mod.rs`

## Purpose

Capture is the front door. Nothing sensei knows about the developer's
work exists without it. **Four paths** feed the pipeline, each with
its own restart-safety guarantee:

1. **Filesystem scan** (one-shot / manual) — turns real folders into
   projects. Reads the user's chosen roots (default `~/Developer`,
   `~/Work`), detects repos, groups multi-repo projects, seeds the
   folder + code graph.
2. **Root-watcher** (continuous) — a live filesystem watcher on each
   scan root. When files are added / removed / modified, the
   watcher emits scan events that update the folder / code graph
   incrementally, no manual re-scan required. On daemon
   restart the watcher **reconciles against last-known content
   hashes** so any changes that landed during downtime are picked
   up automatically.
3. **Git checkout / branch switch** — when the user switches
   branches, the scanned data flips to the target branch's
   version. Only active-branch state is scanned by default; the
   flip is a copy-on-switch (add / remove only what changed
   between the two branches). This means folder-level data is
   **versioned by branch** — see §"Branch versioning" below.
4. **Assistant event capture** — turns each assistant session into
   a stream of events (session start, user prompt, tool call, tool
   result, session end) via an adapter per assistant family. The
   analyzer materialises sessions from these events, enriches
   them, and downstream signals / FTR / memories fall out of that.
   Formerly called "hook capture" — the term still fits Claude
   Code but the storage table is `activity.assistant_events`
   (agnostic).

Kanji is 捉 — *to catch / capture*.

## Data invariants

### Filesystem scan

- `sensei.scan_roots` — user's chosen watch roots. `Developer` /
  `Work` are typical entries; nested roots (like `rokkit`,
  `dbd-rs`) are allowed but redundant — the outer root subsumes
  them for detection.
- `sensei.folders` — one row per discovered repo/folder under a
  scan root. Fields: `id`, `path`, `name`, `kind` (`git_repo`,
  `standalone`, `library`, `unknown`), `project_id` (nullable —
  membership), `scan_state`, `content_hash`, `stack` jsonb,
  `manifest_summary` jsonb.
- `sensei.projects` — logical grouping. Zero, one, or many
  folders per project. See the multi-repo section below.
- The scanner is **incremental** — content-hash based. A folder
  whose hash is unchanged since last scan is skipped end-to-end.
  Branch switches trigger a re-hash but not a re-scan of unchanged
  files. See (memory: project_incremental_indexing) memory for the
  incremental indexing rules that shipped 2026-06-10.
- Scanner writes signals reliably or logs. Silent errors have
  burned us; see (memory: feedback_no_silent_errors).

### Root-watcher (continuous)

- One `notify`-backed watcher per active scan root, started on
  daemon boot.
- Debounces file events (default 500ms window) and coalesces bursts
  (a bulk `git checkout` fires thousands of events; the watcher
  collapses to one reconciliation pass).
- Emits events onto the scan queue: `folder_added`,
  `folder_removed`, `file_changed`, `manifest_touched`,
  `git_head_moved`.
- **Restart-safety.** On daemon start the watcher enumerates each
  root and compares against last-persisted content hashes in
  `sensei.folders.content_hash`. Any drift is enqueued as a
  reconciliation pass. This means a crash mid-scan doesn't lose
  progress AND changes that landed while the daemon was down are
  discovered on next boot with no user action.
- The watcher runs alongside the periodic re-scan tick; the tick
  is a safety net for missed events (network drives, OS event
  drops), not the primary source.

### Branch versioning (git-driven capture)

Sensei needs to understand code state per branch, not a single
"here's the repo" snapshot. When a user switches branches, the
scan graph must flip to the target branch's state.

- **Scanned-branches list** — `sensei.folder_branches` (proposed)
  stores which branches have ever been scanned per folder. The
  default policy: **only the active branch is kept fully
  scanned**. Historical branch state is retained in a compact
  form (per-file content hash + minimal graph deltas) so a
  switch-back doesn't need a full re-scan.
- **Branch field on graph tables** — `sensei.nodes`,
  `sensei.edges`, and per-file scan-state rows carry a `branch`
  column. The active-branch view is a filter; historical branches
  are queried through the same tables.
- **Copy-on-switch semantics** — when the watcher observes a git
  HEAD move:
  1. Compute the diff between old-branch HEAD and new-branch HEAD.
  2. For files unchanged between the two: reuse the existing graph
     rows, just re-tag `branch = new_branch`.
  3. For added files: enqueue as new scan work.
  4. For removed files: retract from the active view (but keep the
     historical rows tagged with the old branch).
  5. For modified files: re-parse; supersede the old branch's rows
     for the active view.
- The switch is **incremental** — the same content-hash rules
  that make cross-tick scans cheap apply here (see
  (memory: project_incremental_indexing) memory for the 2026-06-10
  implementation).
- **Branch scan modes** (open decision — bias: yes, worth
  supporting):
  - `active-only` — default. Only the currently checked-out
    branch is scanned. Switching flips the view.
  - `pinned` — user pins additional branches (e.g. `main`) that
    should be kept scanned so drift comparisons work without a
    checkout.
  - `all` — every branch scanned. Expensive; only worthwhile for
    small repos.
- **UI implications** (mockup does not cover this yet — flagged
  as a spec gap):
  - Some cards may want a "branch: main / feat-x" indicator when
    a project has > 1 scanned branch.
  - A branch switcher/toggle would let the user view code-graph
    or impact data as of a different scanned branch. Deferred to
    a follow-up screen spec; called out as a `todo` under
    Impact / Traceability screens.

### Multi-repo projects

A **project** in sensei is a logical unit, not a filesystem repo.
Real-world projects are often split across repos:

- One repo for docs / wiki
- One for backend / API
- One for web UI
- One for mobile app
- One for infra / IaC
- One for design system

Sensei should **detect these groups** at scan time and **suggest
the grouping**, not force the user to combine them by hand.

Detection heuristics (ordered by confidence):

1. **Shared parent folder + related naming.** `~/Work/acme/api`,
   `~/Work/acme/web`, `~/Work/acme/docs`, `~/Work/acme/infra` are
   almost certainly one project.
2. **Cross-repo dependency signals.** Repo A's `package.json`
   depends on a package published from Repo B; Repo A's OpenAPI
   client references Repo B's spec; Repo A's docs link to Repo B.
3. **Shared org / client label.** If two repos declare the same
   `client` or `org` metadata (from the project detect step),
   propose grouping.
4. **User confirms.** Auto-suggested grouping shows up as a
   proposal card the user accepts or dismisses. Never merge
   silently.

Once grouped:

- `sensei.projects.id` is the join key; `sensei.folders.project_id`
  points at it. Multiple folders per project is already supported
  by the DDL and the handlers (`list_folders_by_project`,
  `set_folder_project`, `add_solution_repo`).
- `POST /api/projects/merge` (existing) folds one project's
  folders + sessions + memories into another — used when the user
  accepts a suggestion or manually combines projects.
- Splitting is the inverse — an escape hatch when auto-grouping is
  wrong. Endpoint TBD (open decision below).
- The **primary folder** (chosen at grouping time) drives:
  - The project icon inference (see [[pipeline/project-icon]])
  - The default `vision` seed
  - The maturity roll-up

Folder-role labels (`ui`, `backend`, `docs`, `wiki`, `infra`,
`mobile`, `design`) are stored on the folder-project membership
row so the UI can render "acme's UI + backend + docs" instead of
opaque repo names. `add_solution_repo` already carries a `role`
field.

### Assistant event capture

- **Assistant events** are the atoms — assistant-family-agnostic.
  Each event carries `client_session_id`, `assistant_family`
  (claude / zed / cursor / …), `assistant_id`, `event_kind`
  (`session_start`, `user_prompt`, `tool_call`, `tool_result`,
  `session_end`, `notification`, …), `tool_name` (nullable),
  `ts_ms`, `payload` jsonb, optional `duration_ms`, optional
  `error`.
- Persisted to `activity.assistant_events` — one row per event,
  no aggregation at capture time. The older name `hook_events`
  was Claude-specific; the rename to `assistant_events` reflects
  that the storage is agnostic. Claude Code's "hooks" are one
  adapter; Zed's session log is another; future adapters follow
  the same shape.
- Session materialisation runs on the analyzer tick (see
  [[pipeline/analyzer]]). It walks `assistant_events` by
  `client_session_id`, materialises turns, computes corrections
  and FTR, writes `activity.sessions`.
- **Session-id gotcha:** `activity.sessions.id` is a sensei UUID;
  `activity.assistant_events.session_id` stores the adapter's
  own client-side id (Claude Code's UUID, Zed's session id, etc.).
  Every join that starts from `sessions.id` must resolve back to
  `client_session_id` via `activity.sessions.client_session_id`.
  Regression trap; see (memory: feedback_apis_consistent_with_data).
- **No PII / no secrets in captured payloads.** Prompt text can
  contain code, project names, session context. It is stored
  locally, never egressed to a remote host by capture. If a payload
  looks like it contains a secret (matched against a pattern list),
  redact at capture time and log the redaction.

### Adapter architecture (assistant families)

Every assistant family is an adapter — same trait, isolated
implementation. No cross-family conditionals in shared code.

    trait AssistantAdapter {
        fn family(&self) -> AssistantFamily;
        fn ingest_event(&self, raw: RawInput) -> Result<Vec<AssistantEvent>>;
        fn healthcheck(&self) -> AdapterHealth;
    }

The daemon owns dispatch: incoming events go to the right adapter
by family, which normalises them into the canonical
`AssistantEvent` shape written to `activity.assistant_events`.

| Family | Adapter path | Status | Notes |
|---|---|---|---|
| Claude Code | `marketplace/plugins/sensei/hooks/` (plugin) + `crates/senseid/src/adapters/claude/` (daemon-side normaliser) | live | Every hook type, PreToolUse / PostToolUse / UserPromptSubmit / Stop / SessionStart / SessionEnd |
| Zed | `crates/senseid/src/adapters/zed/` | live (read-only) | Historical session ingest; multi-model corpus (`sessions.provider` distinguishes gpt-5 / grok / gemma4 / claude-through-many-providers). Real-time hook is TBD |
| Cursor | `crates/senseid/src/adapters/cursor/` | not built | Follow the Zed pattern; not required for v1 but must be supported once the system is working end-to-end for Claude Code |
| Cline | same directory | not built | as above |
| Aider | same directory | not built | as above |
| Custom | user-defined via MCP `log_event` | live | Any tool can push an `assistant_event` through the MCP surface for cases where an adapter doesn't exist |

The v1 contract is: **Claude Code is the reference implementation;
the rest ride the same adapter trait when they arrive.** No
spaghetti — a new family means a new file, not edits scattered
across the daemon.

## Signals produced

| Signal | Source | Consumer |
|---|---|---|
| `sensei.projects` (rows) | scan | every screen |
| `sensei.folders` (rows) | scan | Project window Overview |
| Multi-repo project suggestion | scan analyzer | [[screen/observatory-projects]] proposal card OR new banner |
| `activity.assistant_events` (rows) | assistant plugin / adapter | [[pipeline/analyzer]] materialisation |
| `activity.sessions` (rows) | materialisation | [[pipeline/ftr]], [[screen/observatory-sessions]] |
| `activity.assistant_events` (rows) | materialisation | [[screen/observatory-instruments-replay]] |
| `sensei.tool_usage_stats` (view) | roll-up over assistant_events | [[pipeline/signals]] |

## Done gate

- On a fresh scan of `~/Developer` + `~/Work`, every discovered
  repo is a `sensei.folders` row within the incremental scan
  window.
- Auto-suggested multi-repo project groups appear as proposal
  cards. User accept merges via `POST /api/projects/merge`; user
  dismiss leaves the folders as independent projects.
- Every Claude Code session with the sensei plugin installed
  produces `activity.assistant_events` rows continuously (not
  batched at session end).
- Every session with hook events has an `activity.sessions` row
  after the next analyzer tick; every session row has non-null
  `analyzed_at`, `ftr`, `corrections` after enrichment.
- Session-id resolution: any endpoint or MCP tool that takes a
  session identifier accepts either the observatory UUID or the
  `client_session_id` and resolves consistently.
- Scanner emits errors — no silent `.ok()` drops. Failed folders
  are visible in the scan status view.
- No hook event payload leaks to a remote host from capture. Local
  storage only.

Optional check:
```
# Are assistant events flowing right now?
psql -A -t -c "select count(*) from activity.assistant_events
                where ts_ms > (extract(epoch from now()) - 60) * 1000" -d sensei

# Multi-repo project suggestions pending?
curl -s http://localhost:7744/api/projects/suggestions | jq '.suggestions'

# Session-id resolution consistent across paths?
curl -s http://localhost:7744/api/sessions/$OBSERVATORY_UUID | jq '.client_session_id'
curl -s http://localhost:7744/api/sessions/$CLIENT_SESSION_ID | jq '.id'
# expected: both resolve, both return the same session
```

## Wrong gate

- **Every folder becomes its own project.** Multi-repo detection
  never fires; user's `acme-api` + `acme-web` show up as two
  separate projects with no suggestion to combine.
- **Suggestion cards keep re-appearing immediately after
  dismiss.** No persistence of the "user said no" state; the
  7-day staleness window should hold until it fires.
- **Assistant events arrive but sessions never materialise.**
  Analyzer tick isn't scheduling `materialise_session` OR the
  client-session join is broken.
- **`activity.assistant_events` grows to 1M+ rows and slows the
  scanner.** Retention isn't running; see the analyze-first-guard
  retention task from (memory: project_ingest_scan_bug_batch).
- **Root-watcher misses a file addition made while the daemon was
  down.** Restart reconciliation isn't running against
  `content_hash`; the change is invisible until the periodic tick
  catches up.
- **Branch switch flips the git HEAD but the code graph still
  shows the old branch's rows in the active view.** Copy-on-switch
  didn't fire, or the branch tag isn't being applied to the graph
  filter.
- **Two adapters both write to `assistant_events` with the same
  `client_session_id`.** Family collision; the id namespace must
  be `{family}:{raw_id}` or events cross-contaminate sessions.
- **Session ids returned by the API are UUIDs, but the assistant
  event stream keys them as strings.** Path resolution broken —
  the session bug we already fixed in this repo.
- **A prompt payload containing an API key was persisted verbatim
  to `hook_events.payload`.** Secret-redaction at capture didn't
  fire. Add to the pattern list.
- **Merging two projects loses their historical FTR trend.** The
  `merge_projects` fold must preserve the sessions' project_id
  attribution so the FTR view continues to compute cleanly.
- **Zed sessions never carry a `model` field.** Zed-adapter
  regression; the multi-model corpus depends on this being populated
  (see (memory: project_standalone_completion_plan) memory).
- **`~/Developer/rokkit` and `~/Developer` are both listed as
  scan roots and the inner is being redundantly indexed.** Root
  deduplication is missing; the outer subsumes.

## Locked decisions (2026-07-07)

- **Split endpoint** — worth building. `POST /api/projects/{id}/split`
  will let a user un-merge repos previously combined by
  auto-suggestion or manually. Small effort; ships in v1.
- **Suggestion staleness** — a dismissed multi-repo suggestion
  goes stale after **7 days** by default. After the staleness
  window, the same shape can re-fire (perhaps the user wants it
  now). The window is configurable in Preferences under Scan.
- **Role auto-detection** — needed. The scanner reads
  README / Dockerfile / package.json / manifest fingerprints and
  pre-fills the role on `add_solution_repo` (`ui` / `backend` /
  `docs` / `wiki` / `infra` / `mobile` / `design`). User can
  override; the auto-fill is a starting point.

## Open decisions

(All prior open decisions locked above; new ones surface here.)

- **Branch scan modes UI.** The `active-only` / `pinned` / `all`
  choice sits under Preferences → Scan → Branch policy. The exact
  layout is deferred to a Preferences pane spec.
- **Non-Claude assistant real-time capture.** Zed adapter is
  read-only today. When the first non-Claude family gets
  real-time capture (Zed live? Cursor?), the adapter trait needs
  a `stream_events()` sibling to `ingest_event()`. Not urgent
  until v2.

## Related

- [[pipeline/analyzer]] — the tick that materialises sessions
- [[pipeline/ftr]] — depends on `sessions.ftr` written by materialisation
- [[pipeline/signals]] — depends on `sensei.tool_usage_stats`
- [[pipeline/project-icon]] — resolves against the primary folder
- [[screen/observatory-projects]] — where multi-repo suggestions surface
- [[screen/project-overview]] — where the multi-repo membership is shown
- (memory: project_ingest_scan_bug_batch) (memory) — historical scan bugs
- (memory: project_stale_folder_reconcile) (memory) — self-healing reconcile
- (memory: project_incremental_indexing) (memory) — content-hash incremental rules
