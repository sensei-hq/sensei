# Checkpoint

**State 2026-10-13.** Active slice: **every call edge placed — compiler engine
per language** (`docs/plan/2026-10-13-indexer-full-call-placement.md`). Scope:
the sensei repo. Rust, then JS/TS/Svelte, then references/imports, then SQL/ER.

| increment | status |
|---|---|
| Rust engine: rust-analyzer `scip`, in-adapter `CompilerEngine`, task-layer IO | DONE `02d573fa` `13e42d80` |
| Macro/path calls, locals dropped; renamed imports, all features × 3 platforms | DONE `2b2f9d81` `c9893779` |
| `CompileProjects` task ahead of the files | DONE `6aea71ec` |
| Compile scheduling: once per source state `5b1b0238`, one in flight per repo `a6e0783d`, turns + last-finished read `923ce05d`, once per project per task `d46a30b5` | DONE |
| TS/JS/Svelte engine: embedded node driver (project's typescript + svelte2tsx), `lib·ecmascript` for globals | DONE `e90bb9ce` |
| Vendored/generated excluded by `.gitattributes` linguist marks + `@generated` header; skipped files shed nodes | DONE `6ab328ef` |
| Live verify, 7th rescan (t0 06:52:41Z) → per-reason root-cause cycle, Rust + TS/Svelte | IN PROGRESS |
| references + imports via the compiler (`plumbing`/`external_boundary` remain there) | TODO |
| bootstrap installs/checks engines (`ts_compiler_engines.json` → `~/.sensei/engines`; rust-analyzer + rust-src) | TODO |
| ≥90% regression gate; SQL/ER placement; `graph_placement` language/package axes; final report; docs/prd/spec R5 §5 | TODO |

**Next command:** when the rescan finishes, run
`select * from sensei.graph_placement where project='sensei' and edge_kind='calls'`,
then group the misses by language and reason over the files parsed after t0.

**Open questions:** none.

**Known broken:** the live DB is guarded at v1 by dbd, so additive DDL is
hand-applied with psql (`scan_skip_reason` += vendored, generated: done live and
in `sensei_test`). `dbd import` fails on `model_capability.decision`
(pre-existing). Release N and #236 4e are on hold until this slice is done.
Queue afterwards: #225, #239–241, #245, #252, #254, #238, and the #221 visual
check. Untracked files from another session are not touched.
