# 捉 · Pipeline · Capture

**Owner files:**
- Scanner: `crates/senseid/src/scan/` (root watcher, project detection, folder classification)
- Hook capture: `crates/senseid/src/api/handlers/hooks.rs` +
  `crates/senseid/src/db/pg_store.rs::insert_hook_event`
- Session materialisation:
  `crates/senseid/src/api/handlers/sessions.rs::materialise_session`
- Plugin hook shim (Claude): `marketplace/plugins/sensei/hooks/`

## Purpose

Capture is the front door. Nothing sensei knows about the developer's
work exists without it. Two streams:

1. **Filesystem scan** — turns real folders into projects. Watches
   the user's chosen roots (default `~/Developer`, `~/Work`),
   detects repos, groups multi-repo projects, keeps the graph fresh.
2. **Assistant hook capture** — turns each assistant session into a
   stream of hook events (session start, user prompt, tool call,
   tool result, session end) that the analyzer enriches into
   sessions + memories + signals + FTR.

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
  files. See [[project_incremental_indexing]] memory for the
  incremental indexing rules that shipped 2026-06-10.
- Scanner writes signals reliably or logs. Silent errors have
  burned us; see [[feedback_no_silent_errors]].

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

### Assistant hook capture

- **Hook events** are the atoms. Each event carries
  `client_session_id`, `assistant_family` (claude / zed / cursor
  / …), `assistant_id`, `hook_name` (`SessionStart`,
  `UserPromptSubmit`, `PreToolUse`, `PostToolUse`, `Stop`, …),
  `tool_name` (nullable), `ts_ms`, `payload` jsonb, optional
  `duration_ms`, optional `error`.
- Persisted to `activity.hook_events` — one row per event, no
  aggregation at capture time.
- Session materialisation runs on the analyzer tick (see
  [[pipeline/analyzer]]). It walks hook events by
  `client_session_id`, materialises turns, computes corrections
  and FTR, writes `activity.sessions` + `activity.assistant_events`
  (the derived per-turn stream keyed by `sessions.id`).
- **Session-id gotcha:** `activity.assistant_events.session_id`
  stores the observatory session id; `activity.hook_events`
  stores the client-side one. Every join that starts from
  `sessions.id` must resolve back to `client_session_id` via
  `activity.sessions.client_session_id`. Regression trap; see
  [[feedback_apis_consistent_with_data]].
- **No PII / no secrets in captured payloads.** Prompt text can
  contain code, project names, session context. It is stored
  locally, never egressed to a remote host by capture. If a payload
  looks like it contains a secret (matched against a pattern list),
  redact at capture time and log the redaction.

### Assistant family coverage

| Family | Hook source | Notes |
|---|---|---|
| Claude Code | plugin hooks (`marketplace/plugins/sensei/hooks/`) | Full — every hook type |
| Zed adapter | `crates/senseid/src/adapters/zed/` | Multi-model corpus; `sessions.provider` distinguishes gpt-5 / grok / gemma4 / claude-through-many-providers |
| Cursor / other | TBD | Follow the Zed-adapter pattern |

## Signals produced

| Signal | Source | Consumer |
|---|---|---|
| `sensei.projects` (rows) | scan | every screen |
| `sensei.folders` (rows) | scan | Project window Overview |
| Multi-repo project suggestion | scan analyzer | [[screen/observatory-projects]] proposal card OR new banner |
| `activity.hook_events` (rows) | assistant plugin / adapter | [[pipeline/analyzer]] materialisation |
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
  produces `activity.hook_events` rows continuously (not batched
  at session end).
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
# Are hook events flowing right now?
psql -A -t -c "select count(*) from activity.hook_events
                where ts_ms > (extract(epoch from now()) - 60) * 1000" -d sensei

# Multi-repo project suggestions pending?
curl -s http://localhost:7744/api/projects/suggestions | jq '.suggestions'

# Session-id resolution honest across paths?
curl -s http://localhost:7744/api/sessions/$OBSERVATORY_UUID | jq '.client_session_id'
curl -s http://localhost:7744/api/sessions/$CLIENT_SESSION_ID | jq '.id'
# expected: both resolve, both return the same session
```

## Wrong gate

- **Every folder becomes its own project.** Multi-repo detection
  never fires; user's `acme-api` + `acme-web` show up as two
  separate projects with no suggestion to combine.
- **Suggestion cards keep re-appearing after dismiss.** No
  persistence of the "user said no" state.
- **Hook events arrive but sessions never materialise.** Analyzer
  tick isn't scheduling `materialise_session` OR the client-session
  join is broken.
- **`activity.hook_events` grows to 1M+ rows and slows the
  scanner.** Retention isn't running; see the analyze-first-guard
  retention task from [[project_ingest_scan_bug_batch]].
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
  (see [[project_standalone_completion_plan]] memory).
- **`~/Developer/rokkit` and `~/Developer` are both listed as
  scan roots and the inner is being redundantly indexed.** Root
  deduplication is missing; the outer subsumes.

## Open decisions

- **Split endpoint.** No `POST /api/projects/{id}/split` today.
  If a user grouped two repos and now wants to separate them, the
  path is manual. Worth building or worth documenting the manual
  path (delete + rescan)? Bias: build it, small effort.
- **Suggestion decay.** How long a dismissed suggestion stays
  suppressed. Bias: forever unless the file structure changes
  materially (new cross-repo dep, new shared parent).
- **Role auto-detection.** `ui`/`backend`/`docs` roles today
  require the user to pick from `add_solution_repo`. A heuristic
  (README says "API", `Dockerfile` uses `nginx`, `package.json`
  has `react`, …) could pre-fill the role. Bias: yes, low-cost
  and improves the "acme's UI + backend + docs" chip strip.

## Related

- [[pipeline/analyzer]] — the tick that materialises sessions
- [[pipeline/ftr]] — depends on `sessions.ftr` written by materialisation
- [[pipeline/signals]] — depends on `sensei.tool_usage_stats`
- [[pipeline/project-icon]] — resolves against the primary folder
- [[screen/observatory-projects]] — where multi-repo suggestions surface
- [[screen/project-overview]] — where the multi-repo membership is shown
- [[project_ingest_scan_bug_batch]] (memory) — historical scan bugs
- [[project_stale_folder_reconcile]] (memory) — self-healing reconcile
- [[project_incremental_indexing]] (memory) — content-hash incremental rules
