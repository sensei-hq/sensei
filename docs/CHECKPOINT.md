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

**In progress — #123:** left (6) `event_count`'s meanings, (d) `#L10` fragments, Track B.

**Next:** #190 (hours: `DbdManifestAdapter` → dbd's `project::survey`; workspace designs →
`Workspace::load`), then the two loaders that turn a failed read into an empty list (packs,
contributions in `you/[section]/+page.ts`).

**Open (the user's):** whether to release now for the demo; whether "Tokens" should exclude cache reads.

**Known:** the dōjō Workers build is intermittent on PR/develop commits. qlty is out of minutes. After a DDL
change, run `make test-template`. The Svelte MCP did not connect this session (svelte-check stood in).
