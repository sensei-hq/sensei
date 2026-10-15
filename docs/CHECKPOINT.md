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

## Gate for `/sensei:build`

- **P0** (Linux session synthesis) must make the spec's P0 SQL return `t`. Today `f`. Separate slice. Blocks F5 only.
- **Jira tenant** from the user is needed for task F1.1 (fixtures; checks claim C11). F1.2–F1.5 can proceed against the fake.

## Next command

`/sensei:build docs/plans/2026-10-15-tracker-jira-linkage.md`, starting at F1.1 (needs the tenant) or F1.2 (DDL).
