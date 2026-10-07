# Checkpoint

**Slice:** Observatory diagrams — screens. #220 Neighbourhood in progress.

**Done this slice:** #215 #216 #224 #222 #223 #217 #161 #227 #235 #242 #243 #231
#232 #244 #233 #219 (code landed). #220 endpoint `4845e435` — walk + hop reads +
route, senseid 3317/0, every test mutation-probed.

**#220 remaining:** page `diagrams/neighbourhood/+page.svelte` + `+page.ts`, nav
entry in `diagrams-nav.ts`, e2e `app/e2e/tests/diagrams-neighbourhood.spec.ts`,
spec doc `docs/spec/screen/observatory-diagrams-neighbourhood.md`. State + spec
are written and green (16/16, 10/10 mutants) but UNCOMMITTED, with
`FunctionDetail` → `FunctionMatch` and `searchFunctions` → `trySearchFunctions`
in `app/src/lib/{types,api}.ts`.

**Queue after:** #221 Schema → #218 transcript opt-in → #236 #225 dōjō → #238 →
#239 (#240 #241). Deferred by decision: #237, #230, #234.

**Next command:** `cd app && bun run test:unit -- diagrams/neighbourhood-state`,
then write the page.

**Environment (2026-10-07):** production DB cut to ONE root,
`~/Developer/sensei-hq/sensei` (was `~/Developer` + `~/Work`). Graph truncated,
both roots deleted, new binaries installed, re-indexed (≈2.4k files, 29k nodes).
Essential backup before the cut: `database/backup/essential/20261007-171121`.
Sessions (391) and memories (16) untouched. 275 projects and 13,504 repositories
are now orphaned rows. `prune_empty_projects` only drops `discovery` projects
with no sessions/memories/recommendations, and there is NO repository pruner.

**Open questions:** whether to prune orphaned repositories (they hold commit
history); #219 / #231 / #233 can now be closed with evidence against the fresh
index. Each one still needs its measurement re-run.

**Known broken:** app e2e gate red before this work (#245, 21 failed). Commits
are not pushed to `develop`. dbd#41 blocks `dbd reconcile` on a deployed DB.
`--features senseid/embedded-llama-cpp` (#203).
