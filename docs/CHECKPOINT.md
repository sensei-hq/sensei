# Checkpoint

**State 2026-10-13.** Slice **every call edge placed — compiler engine per
language** (`docs/plan/2026-10-13-indexer-full-call-placement.md`): DONE for
calls and references on the sensei repo. Report:
`docs/reports/2026-10-13-placement.md`.

| increment | status |
|---|---|
| Rust (rust-analyzer) + TS/JS/Svelte (project's checker, svelte2tsx, `svelte-kit sync`) engines | DONE |
| Compile scheduling (once per source state, one in flight, turns, stale → re-queue) | DONE |
| Calls: callee selection, wrapped/qualified/anonymous/dynamic callees, every TS shape | DONE — Rust 100%, TS 99.96%, Svelte 99.94% (live) |
| References through the compiler (`0586141d` …`8902e3db`) | DONE — Rust 99.92%, TS 99.33%, Svelte 99.55% (live) |
| Owned code only (linguist marks, `@generated`; worktrees in dot-dirs) | DONE |
| Bootstrap installs/checks engines; doctor; daemon boot | DONE |
| TS driver a typed package (`29df86e0`), type-checked in CI | DONE |
| Placement floor ≥90% per language/package/kind, nightly (`e4ec0267`) | DONE — passes |
| SQL/ER: SQL calls/refs 100%; schema store ⊇ every pg FK | DONE (verified) |
| `resolution_quality` package axis (`571d8850`) | DONE |
| Docs: R5, §5, `08-resolve.md`, report | DONE (this commit) |

**Residue, each named:** `untyped_receiver` where the source says `any`
(~90 occurrences); `macro_expansion` 25; `dynamic_dispatch` 1; Rust
`compiler_no_symbol` 9 (let annotations in `#[tokio::test]` bodies, fn-local
types); 3 stale C edges from file-less stubs (backlog entry).

**Next command:** resume the queue — release N (CHANGELOG `[Unreleased]` +
docs/skills/site sync, then `make bump`), #236 4e, then #225, #239–241, #245,
#252, #254, #238, #221 visual check.

**Open questions:** none.

**Known broken:** none in the schema — the live DB was reconciled to the DDL
tree with `dbd reconcile` and the import files loaded with `dbd import` (the
project is pre-release, so reconcile is the path; earlier hand-applied DDL and
reason rows are all in `database/`). 
