# Checkpoint — 2026-10-26

**Slice:** issue burndown (docs/roadmap/08-issue-burndown.md), wave 5, plus demo prep (user: a demo this
week, sensei + dōjō, metrics and diagrams).

**Released v0.21.0** (user go, 2026-10-26): PR #340 merged, tag `v0.21.0`, `make release` installed it
locally (daemon reports 0.21.0) and merged main back into develop. `release.yml` run 40408273707 builds;
**`deploy-dojo` waits for the user's approval** (it creates `dojo.my_metric_series`).
**dbd v0.27.8 released** (crates.io; dbd PR #111 merged): table comments converge (dbd#110), stated dialect
and `config::parse` (dbd#109). sensei pins it everywhere.

**Verified on the installed 0.21.0:** 7-day sessions 2 (was empty); `tokens_per_day` 45M vs
`cache_read_per_day` 5.1B over 14 days (cache reads apart, user's rule); community labels by directory
(largest: `function (crates/senseid/src/indexer)`).

**Closed today:** #289, #194, #331, #338, #190; dbd#109, dbd#110. Filed #339.

**Waiting on observation:** #336 (a week with no `ingest_capture` abandonment), #125 (orphan count after
this release), #161, #338's cadence — all now released.

**#123:** lower-severity list done; Track B: B1 swarm 4/6 (findings fixed), B2/B3 `dojo-db.yml` (dōjō
access rules in CI, green), CLI exit codes, B4 ported and the local duplicate agents removed (user OK;
copies in /tmp/local-agents-removed; `/review-slice` now names the `sensei:` agents). Left: B2's app
e2e in WebKit.

**Before the demo:** fresh sessions and a dōjō sync (synced session metrics end 10-18); a fresh Claude
session (this one runs a pre-rename MCP); the run-through: Today → Metrics → Diagrams (Structure,
Cycles, Neighbourhood, Schema with torii) → `get_callers` live in Claude → dōjō My metrics and Teams.

**Next:** approve `deploy-dojo`; confirm the release run green and the Worker serves 0.21.0; then the
demo run-through, or B2's WebKit e2e / wave 5's #281.

**Open (the user's):** approve `deploy-dojo` for v0.21.0.

**Known:** the dōjō Workers build check and qlty fail on PRs (flaky integration; qlty out of minutes);
the Worker built locally with WORKERS_CI=1 for this release. After a DDL change, run `make test-template`.
Run fmt + workspace clippy before every Rust commit.
