# Checkpoint — 2026-11-12 (lanes/relay-queue on lane/relay-autonomous-queue → develop)

**Slice:** Relay autonomous queue — phases 1–5 COMPLETE on the branch, merged into develop with `--no-ff` and pushed
(this commit is the last on the branch before the merge). The release cut (bump → PR to main → `make release`) is the
user's call and has not been made. Plan: `docs/plan/2026-11-10-relay-autonomous-queue/{README,2-daemon,3-mcp,4-app}.md`
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

**Remaining (release, user's call):** on develop, `git merge origin/main` (LICENSE, #370) → `make bump v=minor` → PR to
main → `make release`; then verify the shipped artifact reproduces the smoke loop and the diagrams section.

**Next command:** in `lanes/merge` (develop): `git merge --no-edit origin/main && make bump v=minor`.

**Open questions (user):** cut the release now, or batch it with the diagrams fix queued in `docs/backlog.md`?

**Known broken:** the diagrams section on a released (main) app against a develop daemon — main ⇄ develop discrepancy,
queued in `docs/backlog.md` "Open — not yet filed". `sensei_test` (Rust template) is stamped for this DDL tree, which
develop shares after the merge.
