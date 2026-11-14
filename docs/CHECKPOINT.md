# Checkpoint — 2026-11-12 (lanes/relay-queue on lane/relay-autonomous-queue → develop)

**Slice:** Relay autonomous queue — phases 1–5 COMPLETE, merged into develop `072ffec80`; `origin/main` merged back
(`99038648a`, LICENSE), CHANGELOG dated (`f0d4e3f8c`), bumped to v0.22.0 (`8e60fd7c1`), release PR develop → main
open (user, 2026-11-12: "merge then minor"). Merging the PR and `make release` on main remain the user's. Plan: `docs/plan/2026-11-10-relay-autonomous-queue/{README,2-daemon,3-mcp,4-app}.md`
(every step ticked, "Executed" notes per task).

**Done:**
- Phases 1–4: DB `b9eb57ac2` on develop; daemon cutover live + `scripts/relay-queue-smoke.sh` PASSED; seven `relay_*`
  MCP tools installed; app sprint band / create-sprint / floating reviews, `relay-sprint` e2e PASSED; service + app
  installed.
- 5.1 docs sync `616ebf188` (PRD, data model, architecture, journeys, features, design status, CHANGELOG).
- 5.2 issues: #399 #400 #401 #402 #403 #404 #405 closed with comments; #415 narrowed + retitled; #398 #396 #393
  noted; backlog row `42de5033e`.
- 5.3 verification on the final tree: fmt + clippy (CI flags) clean; `make test` green end to end — crates, app unit
  3418, dōjō unit 2400, svelte-check, plugin, SQL 100 files (after `scripts/deploy-dojo-db.sh` converged the stale
  local Supabase). Branch-caused failures fixed red-first: `e4dc5f2d2`+`fa82e955b` (walk: blanket `impl<T> … for T`
  invented a type), `ac4951b65` (#161 typed inputs), `2e5265d39` (ratchet 1,136 → 1,153, measured).
- README risk "upgrade path on other machines" resolved: v0.21.1 ships no `relay` schema; the installer reconciles.

**Remaining (user's call):** fix develop's two pre-existing CI reds (bootstrap `sql_test_runner` on CI; `cargo audit`
RUSTSEC-2026-0459) so the protected-branch PR goes green → merge the PR → `make release` on main → verify the shipped
artifact reproduces `scripts/relay-queue-smoke.sh` and the diagrams section.

**Next command:** `gh pr checks <release PR>`; when green and merged: `git checkout main && git pull && make release`.

**Open questions (user):** none — the release is in flight; the diagrams fix is queued behind it.

**Known broken:** the diagrams section on a released (main) app against a develop daemon — main ⇄ develop discrepancy,
queued in `docs/backlog.md` "Open — not yet filed". `sensei_test` (Rust template) is stamped for this DDL tree, which
develop shares after the merge.
