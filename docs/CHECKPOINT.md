# Checkpoint

**Slice:** per-language indexer gap remediation. Branch `develop`, issue #130.
Plan: `docs/spec/2026-09-02-capability-refactor-map.md` + the per-language issue list.

## Done

- **Slices 2/3/4 — DEPLOYED and verified live.** Inheritance 0 → 2,677 edges at
  99.8%. Imports 18.9% → 100.0% (136,562/136,573), survived the stub prune.
- **`78828f7d` #1** TS/JS global receiver → runtime lib. NOT deployed (~12,371 eligible).
- **`2aaf6a09` #2** java third-party CALL → lib node. NOT deployed (~17,209 fabricated).
- **`b9818af5` #2b** java third-party SUPERTYPE → lib node. NOT deployed. Found this
  round: #2 left `resolve_supertype` on the `is_external_pkg` JDK allowlist, so one
  adapter answered two ways about `org.springframework` — lib when CALLED, fabricated
  first-party when EXTENDED. 125 live edges. Both paths now share `is_first_party`;
  the allowlist is deleted.

- **`b5ae9ba4` #3** TS/JS bare identifier is PROBED, not asserted local. NOT deployed
  at time of writing. Needed a definition PRE-PASS (`walk_stmt` takes
  `locals: Option<..>`; `None` = defs-only) because `produce_fqns` emitted each def
  and immediately scanned its body, so a HOISTED callee was absent mid-walk.
  Methods excluded via `parent_fqn.is_none()`. `sfc_fqn_output` unions locals
  across `<script>` blocks (6 of 316 `.svelte` files have >1).

## Remaining, in dependency order — ORDER CORRECTED

**#3 had to land BEFORE #5, not after.** All four map agents agreed independently:
#5 surfaces ~70,059 invisible call sites, and until #3 stopped minting, each new
site made a NEW phantom (~2,050 measured from describe/it/expect on two Karma
repos alone). The def-set worry is handled by harvesting the probe set FROM the
def walk, so #5's new arms stay in sync by construction.

1. ~~MEASURE #1/#2/#2b/#3~~ **DONE — deployed and verified live, see below.**
2. **#5** — NOT what the issue said. Call collection is driven by `scan_body`, not
   `walk_stmt`; the oxc `Visit` already recurses through arrow bodies/args/try
   (verified `oxc_ast_visit-0.124.0/src/generated/visit.rs:1640,2522,3849`). A
   vitest file's body is `ImportDeclaration` + `ExpressionStatement`, so `walk_stmt`
   hits `_ => {}`, NO `emit_*` runs, NO visitor is ever constructed, and
   `produce_fqns` returns `defs: [] refs: []`. Fix = a module-level caller anchor
   (`fqn::item(lang, pkg, "", module)`, already minted at process.rs:1148) over the
   RESIDUAL statements. Emit ZERO new defs.
3. **#4** kotlin (independent). 7 sub-steps: lift `package_root`/`is_first_party`
   into a shared `languages/jvm.rs` (they are PRIVATE to `mod java_fqn`, so copying
   would violate DRY) → shared test helpers → verify node kinds → defs → heritage →
   calls LAST.
4. **#2c** lib-package keying: `resolve_type_call`/`resolve_supertype` key on ONE
   segment (`lib·java·…` 1,206) while `import_target.rs` uses TWO (`lib·java.util·…`
   371, `lib·java.io·…` 131). Same dependency, two groupings, two code paths. Do it
   AFTER the measurement — it changes lib node identity graph-wide.

## Next command

`cargo test -p senseid --bin senseid languages::` — 3.9s, no DB, no daemon-stop.
(`--lib` fails: senseid has no lib target.) Full gate:
`cargo test -p senseid --bin senseid --no-fail-fast` — 2,748 tests, ~183s, and it
DOES need the DB; three publish_run/pg_store tests take ~55s each under a
serialisation gate and print "running for over 60 seconds". That is a NOTICE, not a
hang — do not kill the run (I did, once).

## MEASURED LIVE after deploying #1+#2+#2b+#3 (partial reindex)

Reindexed ONLY `Labs/OmniRoute` + `Dayamed/server`, so global figures understate.

- **#3, OmniRoute**: phantom TS/JS function nodes **8,504 → 1,453 (−83%)**. Total
  function nodes fell by exactly 7,051 too, so NO real definitions were lost.
  Global TS/JS phantoms 21,981 → 14,930 (ts 11,562 → 4,883 = −58%; js −4% only
  because js phantoms live in folders not yet rescanned: swarco 4,618,
  base-app-webapp 1,144, alert-platform 783).
- **#2b, Dayamed/server**: `java·org.springframework%` heritage targets **22 → 0**;
  heritage now lands on lib nodes (`lib·org·…` 23, `lib·java·…` 112).
- **Edge movement is #1+#3 COMBINED and must not be split**: OmniRoute resolved
  50,578 → 53,999 (+3,421), unresolved 26,903 → 23,568 (−3,335). Both shipped in
  one binary. Only the phantom-NODE drop is attributable to #3 alone.

### #1b — the residual, now characterised

The 1,453 surviving OmniRoute phantoms are the BINDINGS-ARM fabrication, not #3.
Top names: encode 76, decode 68, abort 58, getTime 24, close 17, safeParse 16,
pipeThrough 14, toISOString 14, getDate 13 — instance methods on a receiver bound
by `const x = new Foo()`, and 558 of 1,453 carry a 5-segment method fqn. So
`const d = new Date(); d.getTime()` still mints a first-party method for a runtime
built-in. Fix = apply `global_runtime` to the bindings arm exactly as #1 did to
the member arm. REQUIRES rewriting `ts_method_scope`, which pins it as correct.

## Landmines the map surfaced — check these before #5 and #4

- **`fqn_lang` is seeded from `defs.first()`** (process.rs:1119-1126). After #5 a
  `.js` test file yields refs but ZERO defs, so `fqn_lang` falls back to
  `file_lang` = "javascript" and the module container is written
  `javascript·pkg·mod`, while the producer anchor says `typescript·pkg·mod`
  (TS_LANG is hardcoded for BOTH adapters). `fqn_ids.get` then misses. Fix in #5.
- **A nested declaration cannot be named.** `fqn.rs` has exactly four forms and none
  expresses one; `fqn::method(lang,pkg,mod,"outer","inner")` collides with a real
  method on a class named `outer`, and `nodes.ddl:54`'s unique `(folder_id, fqn)`
  would MERGE them. #5 must emit zero new defs.
- **Two corpus tests assert resolution-rate FLOORS this slice pushes down**:
  corpus_tests.rs:411 (rust >0.90) and :475 (java >0.95). If either trips, confirm
  the drop is honest-unresolved before touching the floor. The module header
  (corpus_tests.rs:15-20) says outright that a real fix makes the rate FALL.
- **The bindings arm still fabricates and a GREEN TEST PINS IT.** typescript.rs
  mints `fqn::method(TS_LANG, pkg, ctx.module, ty, method)` for
  `const g = new Gadget(); g.spin()` even when `Gadget` is declared nowhere, and
  `ts_method_scope` asserts that string as correct. Survives this whole slice.
- **kotlin has ZERO corpus coverage here** — `find . -name '*.kt'` returns 0, so the
  generic corpus invariants assert nothing about it.

## Measured myself this round — these supersede older quotes

- #3: 21,981 phantom TS/JS function nodes (ts 11,562 + js 10,419) absorb 25,696 call
  edges vs 33,547 real defs — 41% of TS/JS function call edges hit a node with no
  definition anywhere.
- #4: kotlin has `imports|3713` and zero other edge kinds across 2,062 def nodes.
- #5: test files are 22% of TS/JS files but yield 7.0% of call edges (5.2 vs 19.8/file).
- Headline unchanged: real def-to-def call linkage is 73,792/340,229 = **21.7%**, not
  65.2%. The defect is INVENTION, not absence.

## TWO NEW DEFECTS FOUND WHILE MEASURING — both worth their own issue

### A. A code change to the indexer can never trigger a reindex

`version_rescan`'s doc says a binary-version change makes "the code graph rebuild
under the new binary". It does NOT. It only enqueues one `ScanRoot` per watch root;
`ScanRoot` fans out `ProcessGitFolder`, whose `plan_reindex` diffs
`(rel_path, mtime)` against `scan_state` and hashes only mtime-drifted candidates.
**There is no indexer-version component in the skip decision**, so a
content-identical file is never re-derived no matter what the producer code does.
Observed: `process_git_folder: OmniRoute — 0 changed files, 9822 unchanged`.

This is very likely why so many past counts here are "eligible, not observed".

WORKAROUND USED (scoped, reversible, backup at 07:26 predates it): delete
`sensei.scan_state` rows for the target folders, then scan the WATCH ROOT. Note
`sensei scan <repo>` discovers projects BENEATH the path — pointing it at a repo
logs `0 git, 0 standalone project roots` and does nothing. Scan the watch root
(`$HOME/Work`), not the repo inside it (`$HOME/Work/<org>/<repo>`).

### B. Kotlin build variants are a poison pill — blocks #4

`base-app-android` never finishes indexing. Every per-country source set
(`app/src/{sgalert,panama,ecuador,guatemala,panamacity,cityofdoral}/`) declares the
same `colorPrimary` in the same package, and kotlin fqns anchor on the package with
an EMPTY module segment (`fqn::item(KOTLIN_LANG, package, "", name)`), so all six
collapse to one fqn:

    upsert fqn def kotlin·com.senecaglobal.countryapps.ui.activities.ui.theme·colorPrimary
    → duplicate key value violates unique constraint "nodes_unique_fqn"
    → folder left `failed`, scan_state not advanced

`scan_state` is not advanced, so it retries forever — 112 occurrences in 200KB of
log. This is a PREREQUISITE for #4: the Kotlin fqn scheme cannot distinguish source
sets, and #4 would add refs on top of a folder that never indexes.

## Known broken / do not repeat

- `resolved` counts resolution-to-a-STUB (241,046/438,270). Never quote as "resolved".
- `edge_resolution_class` names what it MEASURES, not a verdict. `name-collision-1` is
  not a defect queue — its head is `json` 1,600, `path` 483, external accessors.
- `detected_patterns`: 1,447 rows, `family` NULL on ALL, unchanged across a reindex
  that added 106,906 edges. Undiagnosed.
- 176,261 of 176,281 unresolved refs are markdown prose. Needs its own class.
- The 40,651 duplicates are a FOLDER-REGISTRY defect (one repo at two paths; `cluster`
  vendoring `server`/`scheduler`), NOT an fqn-scheme defect. Refuted twice — drop it.
- `nodes.language` is last-writer-wins on the reference path (graph.rs lacks the
  `CASE WHEN EXCLUDED.resolved` guard its neighbours have).
- Assume tests PIN current behaviour, not intent — two java tests already did.
- `import_target.rs` deliberately probes the graph rather than testing any prefix. Do
  NOT "fix" it to use `is_first_party`; the probe is the stronger question.

## Traps

Gate with the daemon STOPPED for the FULL suite (`languages::` alone needs neither).
NEVER `git checkout <path>` to revert a probe — destroyed uncommitted work twice; `cp`
to /tmp and verify with an explicit grep. ONE probe at a time. rustfmt rewraps between
scripted edits, so prefer `Edit`. `PIPESTATUS` is bash — in this zsh it is silently
EMPTY, and a backgrounded chain reports the LAST command's code, not cargo's; redirect
and read `$?` unpiped. `make install-debug` ends with `cargo clean` + a multi-GB dump,
so every deploy is ~10 min. DDL goes to BOTH `sensei` and `sensei_test`.
