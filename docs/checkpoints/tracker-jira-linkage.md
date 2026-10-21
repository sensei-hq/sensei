# Checkpoint — tracker integration (Jira) — 2026-10-16

**Slice:** Jira tracker linkage, local daemon (T-1a + T-1d local slice)
**Branch:** `feature/jira_ticket_linkage` (off `feature/linux_support`)
**Stage:** design ✅ → build in progress (F1.2–F1.5 + F2 done; F1.1 needs the Jira tenant)

| Artifact | Path |
|---|---|
| Analysis | `docs/analysis/2026-10-15-jira-ticket-linkage.md` |
| Spec | `docs/spec/daemon/tracker-jira-linkage.md` (status: reviewed) |
| Plan | `docs/plans/2026-10-15-tracker-jira-linkage.md` (issues TBD — no `gh` here) |
| Deviations | `docs/backlog.md` → "Jira ticket linkage" entry (7 T3 overrides) |

## Claims found false, and what changed

- **C21** (draft 1): "tracker_links drop is recorded in backlog". It wasn't, so it's now recorded.
- **C19** (draft 2): "no Origin guard exists". `origin_may_mutate` + `ALLOWED_ORIGINS` exist in `repositories.rs`. The new guard was replaced by extracting the existing one to `api/origin_guard.rs`, plus a Host check.
- **C20** (draft 2): "no session read uses the project junction". The `folder_projects` view already does this, so R1 now reuses it on `repo_folder_id`.
- MISLEADING, all fixed: C7 (5 `KeychainError` variants, not 2), C9 (`DESCRIPTIONS` + `tick()` + the `server.rs` spawn), C18 (loopback is the default, not the only bind), C24 (`repo_folder_id` is also set-null).

## Model (user, 2026-10-16)

A sensei project holds many repos (README front matter `project:`) and exactly one
Jira board; the board holds many sprints; a sprint holds many tickets; tickets link
to sessions from any of the project's repos. The 2026-10-15 "one board, many
projects" binding was reverted: `trackers.project_id` and `(kind, base_url, board_ref)` are
both unique; usage breaks down `by_repo` (parts sum to the total).

## Integration seam (user, 2026-10-16)

Jira is the first adapter behind a `TrackerAdapter` trait (house adapter pattern, like
`TranscriptAdapter`). Tables, credentials, GET transport, sync coordinator, relink,
usage, routes (`/trackers/{kind}/…`) are provider-neutral; GitLab is the next adapter
(spec "Future adapters": iterations vs milestones, `#12` keys scoped by repo,
self-managed hosts).

## Desktop app connect screen (user, 2026-10-16)

Project window section 繋 Integrations: `docs/spec/screen/project-integrations.md`
(plan F6). Form: Board URL, Atlassian account email (required: Jira Cloud tokens are
`email:token`), API token. The project key is derived from the board. Reviewed for UX
(a11y, dialog, recovery actions) and conformance (every daemon code mapped;
`board_name` and the `board_not_found` body added to the daemon contract;
board-URL parsing moved to F1).

## Build progress

Run contract: `.sensei/slice-state.json` (resumable without this conversation).

- Preflight GO (2026-10-16, after disk cleanup). `sensei_test` created and deployed.
- Baseline: 3511 pass / 7 fail. The failures are pre-existing (6× rustup missing for
  rust-analyzer on nix, 1 flaky).
- **F1.2 schema** ✅ 7 DB tests (enums, 5 tables, tombstone trigger).
- **F1.3 pure core** ✅ 21 unit tests (URL policy, Retry-After, Jira host/board, current
  sprint, status, error codes) and `crate::http::bounded_builder()`.
- Full suite after: 3539 pass / 6 fail (the baseline set). 17 `dead_code` warnings
  remain until F1.4/F1.5 wire the code.
- **F1.4** ✅ Jira adapter, GET-only transport (429 policy, redirects off),
  bounded keyring, sync coordinator (no DB connection across fetches, mutation-checked),
  connect/re-connect/disconnect + tombstone drain. 39 tests.
- **F1.5 + F2** ✅ origin guard extracted to `api/origin_guard.rs` (+ loopback Host
  check), routes `/api/projects/{id}/trackers[/sync|/{kind}/connect]`, CLI
  `sensei tracker connect|sync|disconnect`, scheduled `tracker_sync` (1800 s). 31 tests.
- Full suite 3598 pass / 7 fail (baseline set + 1 pre-existing load flake);
  clippy 0 warnings; fmt clean.
- Found on the way: the Rust indexer miscounts `impl Trait for &Type` (backlog;
  test uses a named wrapper as the recorded workaround).
- **Review (approve depth)** of the F1.2–F2 slice found and fixed:
  - a closed sprint that returns never regained its members;
  - connect/disconnect blocked on a 500-tombstone drain;
  - silent drain failures that could starve the queue;
  - a sync on an old credential overwriting a re-connect;
  - `ReqwestRaw` untested (redirects, `Retry-After`, auth);
  - missing seam/1k/guard tests;
  - Jira key upper-casing leaking into `pg_store`;
  - spec drift.
  F1 had been marked complete without gate 1l. Its sync part is now tested; the
  link and usage parts land with F3/F4.
- **F3 linking** ✅ `session_task_links` + `tracker_task_keys` (aliases, so a renamed
  ticket keeps linking by its old key); `tracker_error` gains `db`. Jira `match_links`
  (ASCII-only, longest prefix, consumed spans), provider-neutral `link::known_tasks`
  (tie-break), sync step 3 (candidates in chunks of 200 from every project repo,
  `user_text` only, matching in `spawn_blocking` under the sync deadline), relink in the
  apply transaction (`FOR KEY SHARE`, `unnest`), `db` failures recorded,
  `GET /api/projects/{id}/tracker-tasks/{key}`. 19 tests written red first; mutation-checked.
  Spec fixes: `_` separates again (3db9cee9 contradicted gate 1a); `KnownTask.current`;
  404 `task_not_found`. Adapter registry is now `&'static [&'static dyn TrackerAdapter]`.
- **Live connect (2026-10-17):** `got_a_guy` ↔ senecaglobal.jira.com board #333 (12 sprints,
  728 tickets). Needed `*.jira.com` in the host rule (33417322). 0 links so far: transcript
  consent is off, so no turns are read.
- **F4 usage** ✅ R7 usage (one grouped query, JSON-built so NULL sums stay null), the
  pure sprint window (`not_started`, `no_dates`), ticket usage with `by_repo`, the sprint
  partition (linked_sprint / linked_other / unlinked), routes `/sprints`,
  `/tracker-tasks[?sprint]`, `/sessions/unlinked`, usage on `/tracker-tasks/{key}`, CLI
  `sensei tracker sprints|usage`. The R1 candidate rule is one SQL fragment
  (`candidate_sessions`) used by linking and every usage read. Tested on the live board.
- **F6 app screen** ✅ project window 繋 Integrations (`/project/[id]/integrations`):
  connect form (board URL or Advanced site + board id, email, token), first-sync polling,
  connected card (Sync now / Replace token / Disconnect with a confirming dialog), every
  daemon code in words, failures take focus, the token never kept. `ModalShell` extracted
  (SignInOverlay and the logs report modal moved onto it); `api.ts` gained one `tryJson`
  that keeps the error body (`tryPutJson`/`tryPatchJson` now use it). 50 new unit tests;
  app suite 1837/1837, svelte-check 0. Checked live in headless Chromium against the real
  board. Not run: the Tauri e2e suite (macOS-only harness); the logs modal live (needs Tauri).
- **F7 Sprints screen** ✅ (asked for after F6) project window 期 Sprints
  (`/project/[id]/sprints`): sprint list (active selected), the sprint's own usage split
  (linked / other / not linked) or why it has none, its tickets with sessions, active AI
  time, tokens and cost, a ticket's per-repo breakdown and sessions (old keys shown "as
  <key>"), unlinked sessions on demand. Spec `docs/spec/screen/project-sprints.md`; the
  tracker error words moved to `$lib/tracker-errors.ts` (second consumer). 27 new tests;
  app 1864/1864, svelte-check 0; checked live on the real board.
- **F8 on-demand discovery (R11)** ✅ (asked for 2026-10-19) a ticket first named in a
  Claude Code session is fetched by key — board-scoped, GET only, keys shape-checked before
  any JQL — when the session is ingested (`TrackerDiscover`, coalesced per tracker) or the
  key is looked up, then inserted and linked to every session naming it. Adds only; keys
  the board lacks are cached 6 h (`tracker_unknown_keys`); a failure pauses discovery 5 min
  and writes no `last_error`. New code `bad_request` (400) so a 5xx is never "not on the
  board"; 404 `task_not_on_board`. The full and discovery applies share their SQL helpers.
  Found by the tests: the shared candidate fragment's `tr` alias shadowed an outer `tr`.
  C33/C34 still need a live check on the tenant.
- **F9 backlog** ✅ (asked for 2026-10-21) tickets no live sprint lists: `GET …/backlog`
  ({tickets, usage}, each session once), `?backlog=true` on the ticket list (422 with
  `sprint=`), a Backlog row on the Sprints screen, `usage --backlog` and a backlog line in
  `sensei tracker sprints`. One ticket query serves all/sprint/backlog (`TaskScope`).
  Live: 199 backlog tickets on board 333, P057542601-782/783 among them.
- **Live discovery check (2026-10-21):** lookup fetched P057542601-782 in 1.6 s; a Claude
  Code session naming P057542601-783 had it inserted and linked 0.7 s after ingest.
- **Next:** F5 live verification once Claude Code transcript consent is on.

## Gate for `/sensei:build`

- **P0** (Linux session synthesis) must make the spec's P0 SQL return `t`. Today `f`. Separate slice. Blocks F5 only.
- **Jira tenant** from the user is needed for task F1.1 (fixtures; checks claim C11). F1.2–F1.5 can proceed against the fake.

## Next command

`/sensei:build docs/plans/2026-10-15-tracker-jira-linkage.md`, continuing at F5. F1.1 still needs the Jira tenant.
