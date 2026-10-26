# Checkpoint — 2026-10-26

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md), wave 5, plus **demo prep** (user: a demo
this week, sensei + dōjō, metrics and diagrams).

**Released:** v0.20.2 and dbd 0.27.7. Closed today: #289, #194, #331, #338. Filed #339.

**Waiting on the next release:** #336, #125 (5dea36e3), #161, #338's cadence, the VS Code installed scan.

**Demo prep, done today (develop, not yet released):**
- 3f6f4199: sessions "last 7 days" / `sessions7d` / `last_session_at` key on last activity.
- 00e84641: community labels by where most members live (the largest no longer read "function ()").
- 87bc0bbc, 7d22700c, eeb834ce: dōjō **My metrics** at `/you/metrics` (`dojo.my_metric_series`,
  `GET /v1/you/metrics`, four cards with daily lines). Seen in the browser on the local dōjō.
- Watch roots: dbd, gateway, torii added (user) and indexed.

**Before the demo:**
1. A patch release (`make bump v=patch` → PR → `make release`; deploy-dojo needs the user's approval),
   or at least `make install` locally. Note that a daemon reinstall revokes its Keychain approval.
2. Fresh sessions and a dōjō sync: the synced session metrics end 10-18.
3. Demo from a fresh Claude session; this one runs a pre-rename MCP.
4. A run-through: Today → Metrics (radar, a trend) → Diagrams (Structure, Cycles, Neighbourhood,
   Schema with torii) → `get_callers` live in Claude → dōjō My metrics and Teams.

**In progress — #123 Track B** (details in the burndown): B1 swarm 4/6 on Track A; its live findings fixed
(d9c1c338 one unit per VS Code session, 62a1bfca UTF-8, ffb4544c facet writes, docs). B2/B3: `dojo-db.yml`
runs the dōjō's 58 access-rule files on a fresh Supabase via `scripts/deploy-dojo-db.sh` (also release's
deploy-dojo), green. B4 ported (72032126). Left: over-long `kind:0` line → 0 turns silently; CLI exit codes
and app e2e for B2.

**#190 blocked on sensei-hq/dbd#109.** Filed dbd#110: reconcile/diff ignore a changed TABLE comment (the facet
tables' new table comments are in the DDL, not live; column comments converge).

**Next:** the over-long `kind:0` line, then B2's CLI exit codes. After a DDL change: `make test-template`.

**Open (the user's):** release now for the demo?; should "Tokens" exclude cache reads?; dbd#109's shape and
dbd#110, then a dbd release; delete the local reviewer agents (`/review-slice` names them)?

**Known:** the dōjō Workers build is intermittent on PR/develop commits. qlty is out of minutes. After a DDL
change, run `make test-template`. The Svelte MCP did not connect this session (svelte-check stood in).
