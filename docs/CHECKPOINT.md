# Checkpoint — 2026-10-15

**Slice:** Jira tracker linkage, local daemon (T-1a + T-1d local slice)
**Branch:** `feature/jira_ticket_linkage` (off `feature/linux_support`)
**Stage:** design ✅ → build (gated)

| Artifact | Path |
|---|---|
| Analysis | `docs/analysis/2026-10-15-jira-ticket-linkage.md` |
| Spec | `docs/spec/daemon/tracker-jira-linkage.md` (status: reviewed) |
| Plan | `docs/plans/2026-10-15-tracker-jira-linkage.md` (issues TBD — no `gh` here) |
| Deviations | `docs/backlog.md` → "Jira ticket linkage" entry (6 T3 overrides) |

## Claims found false, and what changed

- **C21** (draft 1): "tracker_links drop is recorded in backlog". It wasn't, so it's now recorded.
- **C19** (draft 2): "no Origin guard exists". `origin_may_mutate` + `ALLOWED_ORIGINS` exist in `repositories.rs`. The new guard was replaced by extracting the existing one to `api/origin_guard.rs`, plus a Host check.
- **C20** (draft 2): "no session read uses the project junction". The `folder_projects` view already does this, so R1 now reuses it on `repo_folder_id`.
- MISLEADING, all fixed: C7 (5 `KeychainError` variants, not 2), C9 (`DESCRIPTIONS` + `tick()` + the `server.rs` spawn), C18 (loopback is the default, not the only bind), C24 (`repo_folder_id` is also set-null).

## Change after design (2026-10-15, user)

One Jira board can serve several sensei projects. The connection is now its own row
(`sensei.trackers`, unique by site + board), bound to projects via
`sensei.tracker_projects` (a project binds at most one). Per-ticket usage is
tracker-wide with a `by_project` breakdown. Re-reviewed by the data-correctness and
conformance reviewers. Fixed as a result: a bind/last-unbind race (`for update` lock
order), read-time candidate intersection so an unbind takes effect immediately, the
CLI deferring URL matching to the server, DELETE returning 200 + body, and the
Wrong-gate scope. Backlog deviations (7) and (8).

## Gate for `/sensei:build`

- **P0** (Linux session synthesis) must make the spec's P0 SQL return `t`. Today `f`. Separate slice. Blocks F5 only.
- **Jira tenant** from the user is needed for task F1.1 (fixtures; checks claim C11). F1.2–F1.5 can proceed against the fake.

## Next command

`/sensei:build docs/plans/2026-10-15-tracker-jira-linkage.md`, starting at F1.1 (needs the tenant) or F1.2 (DDL).
**Slice:** #268, the dōjō diagram screens (next). #225 is live-verified; #238 is built.

**Done** (pushed to develop, through 54e60083):
- #238 steps 2–5: the usage, routing, gate and drive mods. Each was live-verified.
  The policy calls are in `docs/decisions.md` 2026-10-15; PRD M3 §Usage & cost capture.
- #225 live verification on the local stack: all 5 done-gate lines pass.
  Defects found and fixed:
  - `dojo.graph_schema_indexes.name` is now nullable.
  - A deleted file leaves the index (`repos_in_scope`; `prune_vanished` reads `files` rows).
  - `database/tests/run.sh` routes each directory to its scope's database.
  - The app states skip reasons in words.

**Remaining:** #225 decisions (below) → #268 dōjō diagram screens → #254 JPA 8b.
#238 stays open until `make bump` ships the mods (C-13 install path).

**Next command:** `/sensei:session refocus`, then #268. Its fixture is the local Supabase tenant
`personal/graphdev`, holding this repository's develop graph. Start the stack with
`cd dojo && bun run dev -- --host 127.0.0.1`.

**Open questions:**
- #225: `pg_store/tests.rs` exceeds the 3 MB slice cap. Raise the cap, chunk the slice, or split the file?
- #225: should one over-cap file be a partial success rather than error/never-agreed?
- Does `--resume` keep the session id and reset /cost? (`GREATEST` would under-count)
- Should `check_hooks` WARN when the mods module is missing?

**Known-broken / pending:**
- This repository's graph row reads `error` until the over-cap decision is made.
- The mods and the new columns reach installs only at the next release.
- The committed `database/design.dbml` is stale against `dbd dbml` (pre-existing).
