# Checkpoint

**State 2026-10-13.** Active slice: **every call edge placed — compiler engine
per language** (`docs/plan/2026-10-13-indexer-full-call-placement.md`). Scope:
the sensei repo. Rust, then JS/TS/Svelte, then references/imports, then SQL/ER.

| increment | status |
|---|---|
| Rust engine: rust-analyzer `scip`, in-adapter `CompilerEngine`, task-layer IO | DONE `02d573fa` `13e42d80` |
| Macro/path calls, locals dropped; renamed imports, all features × 3 platforms | DONE `2b2f9d81` `c9893779` |
| `CompileProjects` task ahead of the files (watchdog no longer kills compiles) | DONE `6aea71ec` |
| One compile per state of the sources (gate fired 3×/scan, sessions wiped → 50,768 `engine_not_run`) | DONE `5b1b0238` |
| Live Rust verify (5th rescan) → per-reason root-cause cycle | IN PROGRESS |
| TS/JS/Svelte engine: `ts_compiler_driver.mjs` (untracked, unwired) → `TsEngine` | NEXT |
| `plumbing` / `external_boundary` placed onto `lib·` nodes, not `missed` | TODO |
| references + imports via the compiler; vendored/generated excluded by markers | TODO |
| bootstrap installs/checks engines; ≥90% regression gate; SQL/ER placement | TODO |
| `graph_placement` language/package axes; final report; docs/prd/spec R5 §5 | TODO |

**Next command:** wait for the rescan to finish, then run
`select * from sensei.graph_placement where project='sensei' and edge_kind='calls'`
and take the files parsed after `/tmp/scan_t0`.

**Open questions:** none.

**Known broken:** the live DB is guarded at v1 by dbd, so additive DDL and
reason-code rows are hand-applied with psql. `dbd import` fails on
`model_capability.decision` (pre-existing). Release N (CHANGELOG + docs sync +
`make bump`) and #236 4e are on hold until this slice is done. Queue afterwards:
#225, #239–241, #245, #252, #254, #238, and the #221 visual check. Untracked
files from another session (`src/`, `docs/components/`, `docs/rune-usage.md`,
`svelte5-runes-*.md`) are not touched.
