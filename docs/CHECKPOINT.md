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
2. ~~#5~~ **DONE (`571b934b`), deployed and measured — see below.** Was NOT what
   the issue said. Call collection is driven by `scan_body`, not
   `walk_stmt`; the oxc `Visit` already recurses through arrow bodies/args/try
   (verified `oxc_ast_visit-0.124.0/src/generated/visit.rs:1640,2522,3849`). A
   vitest file's body is `ImportDeclaration` + `ExpressionStatement`, so `walk_stmt`
   hits `_ => {}`, NO `emit_*` runs, NO visitor is ever constructed, and
   `produce_fqns` returns `defs: [] refs: []`. Fix = a module-level caller anchor
   (`fqn::item(lang, pkg, "", module)`, already minted at process.rs:1148) over the
   RESIDUAL statements. Emit ZERO new defs.
3. **#4 kotlin — UNBLOCKED. Step 8 (verify node kinds) is DONE.**

   `mod kotlin_grammar_shapes` in kotlin.rs now PINS every shape the producer
   will resolve against, all observed in-tree rather than inherited:
   - `child_by_field_name` returns None for every node (FIELD_COUNT 0) — port
     java's field reads as positional KIND SCANS.
   - `interface_declaration` DOES NOT EXIST; an interface is `class_declaration`
     + the `interface` keyword. kotlin.rs's arm matching it is DEAD, which is why
     every kotlin interface is emitted as `SymbolKind::Class`.
   - call: `call_expression > simple_identifier + call_suffix` (unqualified);
     `call_expression > navigation_expression(recv, navigation_suffix(method))`
     (qualified).
   - heritage: `delegation_specifier`; the SUPERCLASS is the one holding a
     `constructor_invocation` — structural, not a heuristic.
   - `companion_object` nests its OWN `class_body`; enums use `enum_class_body`.
   - imports: `import_header` under `import_list`; a star import carries
     `wildcard_import`.

   **Two design claims were REFUTED here — do not act on them.** "`by` delegation
   destroys the class body" is FALSE (the member is reachable; a delegate carries
   no `constructor_invocation`, so the superclass rule already treats it right),
   and I could not reproduce ANY parse-degrading shape, so no test asserts one.

   **ALL SUB-STEPS DONE at producer level.** `09fd073b` jvm lift → `9373d3f7`
   shared finders (+`rel_to`, net -16 lines) → `345e4b21` defs → `9a4d4d9c`
   heritage → `4622d2d5` calls.

   `resolves_in_scope` STAYS FALSE for kotlin, and mod.rs:777 is still correct.
   The plan said to flip it once an import map existed; that was wrong. The
   capability is specifically "resolve a BARE NAME using the language's own scope
   rules", and the bare arm is exactly what is left unresolved on purpose —
   kotlin has TOP-LEVEL FUNCTIONS, so java's "an unqualified call is a method on
   the enclosing class" rule is false here and porting it would mint a phantom
   per top-level call.

   NOT YET DEPLOYED OR MEASURED. Every kotlin figure so far is a call-SITE count
   from parsing the corpus (10,072 call expressions: 5,903 navigation, 3,496
   unqualified; 127 delegation specifiers), not an observed edge count. Next:
   deploy, clear `scan_state` for the kotlin corpus, scan the WATCH ROOT, then
   measure refs/relations. The bare-call arm (3,496 sites) is the largest known
   gap and wants the definition pre-pass typescript's #3 introduced.

3b. **Historical: #4's three blockers, now cleared or scoped:**
   - There is NO `node-types.json` in `crates/senseid/grammars/kotlin/src/` (only
     `parser.c`, `scanner.c`, `tree_sitter/`), and `#define FIELD_COUNT 0` means
     `child_by_field_name` returns None for EVERY kotlin node — so every java
     resolver line that reads a field must become a kind scan. Only 2 of ~7
     required node kinds were verifiable in-tree (`interface_declaration` is
     ABSENT — kotlin.rs:585's arm is dead, and every kotlin interface is already
     emitted as `SymbolKind::Class`).
   - `find . -name '*.kt'` returns **0** in this repo, so `corpus(&["kt"])` is
     empty and the corpus invariants assert NOTHING about kotlin.
   - The one real kotlin corpus has a folder permanently `failed` — defect B.
   A wrong node kind yields a silently EMPTY producer, indistinguishable from
   today's behaviour, so hand-written fixtures cannot tell you it is wrong.
   **Unblock in this order:** fix defect B (it is a prerequisite AND it makes the
   245-file corpus available to verify the kinds empirically), then do the 7
   sub-steps: lift `package_root`/`is_first_party` into a shared
   `languages/jvm.rs` (they are PRIVATE to `mod java_fqn`, so copying violates
   DRY) → shared test helpers (5 byte-identical `def_fqn`/`ref_to` copies exist;
   kotlin has neither) → verify node kinds → defs (companion_object 41,
   enum_class_body 11) → heritage → calls LAST.

   Recommended shape for defect B: kotlin's own JVM naming puts a top-level
   member on a synthetic `<File>Kt` class, so anchoring top-level functions and
   properties on the FILE resolves the collision while leaving TYPES anchored on
   the package — which preserves the deliberate java/kotlin addressability parity
   at kotlin.rs:595-597. Needs user sign-off: it changes existing kotlin fqns.
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

## #5 MEASURED LIVE (deployed, OmniRoute reindexed)

- **module nodes as call SOURCES: 0 → 35,368.** The anchor works as designed.
- **call edges from test-file sources: 9,167 → 43,752 (4.8x).** Test files went
  from 7.0% of TS/JS call edges to ~26%.
- OmniRoute resolved 53,999 → 77,115; unresolved 23,568 → 35,820.
- **COST, and it is real: OmniRoute phantoms regrew 1,453 → 3,496 (+2,043)** —
  against the map's prediction of ~2,050. Still −59% vs the 8,504 at session
  start. #3 did NOT prevent this because the new phantoms come from two OTHER
  minting paths, now both identified:
  - **#1b bindings arm** (below) — `…·executors/deepseek-web·TextDecoder·decode`.
  - **#3b, FIXED THIS ROUND** — `import_anchor` took the FIRST of
    `local_module_candidates`, i.e. the `src/`-PREFIXED variant.
    `ts_module_path` strips a leading `src/` unconditionally, so that names a
    module no def can carry: **0 of 21,881** real defs have a `src/` segment
    against **1,038 phantoms** that do. The import-EDGE path survives this
    because slice 4 gave it a graph probe that tries both; the CALL path has no
    probe. Third instance this session of a fix landing on one path and not its
    sibling (cf. #2b). Fixed by taking `next_back()`. NOT yet reindexed — the
    1,038 figure is the eligible count, not an observed drop.

### Also found, not fixed

A BARE `src/…` specifier classifies as EXTERNAL with package `"src"` — that is
where the 230 `lib·src·…` nodes come from. A fabricated npm package named `src`.

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

## IS THE KOTLIN FQN FIX APPLICABLE TO OTHER LANGUAGES? — YES, MEASURED

**The fix is language-agnostic.** All TEN languages declaring `supports_fqn`
(c, java, kotlin, python, rust, sql, svelte, swift, typescript, vue) emit defs
through `upsert_node_by_fqn`, which is where `cbe379b6` landed. Nothing bypasses it.

**Other languages hit it far harder than kotlin.** Measured by running each
producer over real corpora (colliding fqns / extra rows):

| corpus | lang | colliding | extra |
|---|---|---|---|
| Dayamed | java | 16,154 | 40,488 (66% of 61,396 defs) |
| Dayamed | ts/tsx | 1,740 | 1,740 |
| Dayamed | sql | 26 | 66 |
| OmniRoute | sql | 16 | 19 |
| base-app-android | kt | 26 | 69 |
| this repo | ts/js/svelte/rs/sql/c | 0 | 0 |

**Build variants are noise. The dominant cause is a VENDORED TREE**: `cluster/`
holds byte-identical copies of its siblings —
`cluster/external/src/.../SurescriptPanelTransformation.java` vs
`external/src/.../SurescriptPanelTransformation.java`, and
`cluster/web-portal/src/app/app.component.ts` vs `web-portal/src/...`. Same file,
indexed twice. That is packaging output, not distinct code.

### DONE AND VERIFIED LIVE

- **`422bd248` build output excluded** — bundle/minified/map/hashed-asset nodes
  **3,555 across 13 files → 0**; the Vite bundle `index-Cgv8QKbu.js` is gone.
  4,515 test files stay, and the list now says so.
- **`296b9a32` one repository at several paths is grouped and reported.**
  `~/Developer/sensei-hq/gateway` IS A SYMLINK to `~/Developer/gateway` (`pwd -P`
  proves it) — one inode tree, two registrations, 4,543 shared fqns, the whole
  rust duplicate population. `assign_repositories` skipped it because it keys on
  a git remote and filters `kind IN ('git','subtree')`, and a symlinked twin is
  `standalone` with no `.git`. Now `symlink_repository_links` (pure, scan_logic)
  groups by resolved path; verified live via `symlinks_linked=1` and a shared
  `repository_id`. A fifth `index doctor` class reports it under "needs your
  decision (NOT repaired automatically)" — output confirmed:
  `gateway: …/gateway | …/sensei-hq/gateway` and `variants: … | …`.
  A directory claimed by TWO repositories is left alone, not resolved by guessing.

- **`621677b9` CORRECTS `ce9e1657` — read this before trusting the class.** The
  first version keyed on `scan_state.content_hash` and reported SEVEN cases; SIX
  were false. `heal_nested_standalone_roots` deletes a mis-scoped root's NODES and
  re-classifies the folder but NEVER deletes its `scan_state` rows, so a folder
  healed long ago still looks fully duplicated by content while holding one
  module-container node. Own nodes vs own scan_state: `swarco/documentation`
  4,311/469 (REAL), `cluster/server` 1/1,970, `cluster/scheduler` 1/1,816,
  `cluster/web-portal` 1/912, `cluster/external` 1/268, `sensei/marketplace`
  1/77, `sensei/homebrew` 1/3. Only `swarco/documentation` is genuine — its own
  git checkout, own remote, inside the `swarco` repo. The check now asks whether
  the SAME FILE carries nodes under BOTH folders, which stale bookkeeping cannot
  fake. Also narrowed both sides of the folder join to node-bearing folders
  first: it was a self-cross-product, 70s against the test DB's 107,230 folders,
  now 4.07s there and 0.68s live.
  DEPLOYED AND VERIFIED: `sensei index doctor` now reports containment **1**, the
  single genuine case (`swarco/documentation`, 770 files, inside `swarco`).
- **STALE `scan_state` AFTER A HEAL IS ITS OWN OPEN DEFECT.** That residue is
  what fooled the first version. `heal_nested_standalone_roots` should delete the
  folder's scan_state alongside its nodes.
- **MARKETPLACE IS NOT DOUBLE-SCANNED** (asked and checked): it has no `.git`
  (git subtree), gets ONE folder row (`workspace_member`) from sensei's own
  monorepo sub-project pass at process.rs:845-885 — structural only, enqueues no
  second scan. Parent holds 774 nodes for `marketplace/%`; the member folder
  holds 1. An earlier "77 nodes" figure here was a query error (a LEFT JOIN
  multiplied 1 node by 77 scan_state rows).

- **`ce9e1657` containment reported** — a sixth doctor class for a folder indexed
  TWICE because it is registered inside another holding the same files. Class 3
  (`nested_standalone`) cannot see these: it matches only `standalone` inside
  `git`, and these are git-inside-git. Detected on `scan_state.content_hash`
  (already written per file) with `starts_with(inner, outer || '/')` — the same
  predicate `nested_standalone_candidates` uses — restricted to the 179 of 9,131
  folders that carry scan_state, so it costs 0.13s. Threshold 0.95, not 1.0: an
  inner checkout legitimately gains a file the outer has not picked up. Live
  output: 7 cases — `cluster/{server,scheduler,web-portal,external}`,
  `swarco/documentation`, and `sensei/{marketplace,homebrew}`.
- **`f9672f1d`** the doctor now prints "… and N more (sample capped)" — the
  containment count said 7 above five lines, and the two dropped were the exact
  subtrees the recommendation cites.

**NEITHER ADVISORY CLASS IS EVER AUTO-REPAIRED, and class 6 is why.**
`homebrew/` and `marketplace/` are git SUBTREES of THIS repository, deliberately
present twice, and they sit at 100% overlap exactly like an accidental nested
checkout. Nothing in the data distinguishes them. Auto-healing would delete a
legitimate subtree.

### STILL OPEN — genuine VENDORING, not a registration artefact

`Dayamed/cluster` holds byte-identical copies of `server`, `scheduler`,
`external`, `web-portal` (12–13k shared fqns each; java 16,154 colliding fqns /
40,488 extra nodes). That is real duplicated source on disk, so neither the glob
change nor repository grouping touches it, and the doctor's new class does not
claim to. Note the containment sub-shape measured at 100% overlap:
`cluster` ⊃ `cluster/server`, `cluster/scheduler`, `cluster/web-portal`,
`cluster/external`, and — in THIS repo — `sensei ⊃ sensei/marketplace` (77 files)
and `sensei/homebrew` (3). The last two are legitimate git SUBTREES, which is
exactly why containment must never be auto-healed.

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

### B. Kotlin build variants were a poison pill — FIXED (`cbe379b6`)

**The constraint that decides this: an fqn is a LOOKUP KEY.** A reference mints
its target from what the CALL SITE can see — for kotlin, package + name — with
no idea which file holds the definition. So the DEFINITION side must not add
file/path information to disambiguate, or the two sides mint different strings
and never match. N files therefore legitimately map to ONE fqn.

I first tried to fix this in the PRODUCER (anchor top-level members on their
source root, since every variant's file is `Color.kt`). That was WRONG and was
reverted before landing — it would have made kotlin references never resolve,
i.e. traded a poison pill for a silently unresolvable language. **Do not retry
it.** The fqn scheme is unchanged.

Fixed in `upsert_node_by_fqn` instead: an `nodes_unique_fqn` conflict raised
from the `adopt_node_by_identity` recovery now resolves to the existing holder
rather than erroring. Reuses `node_id_by_fqn`; a holder the constraint reports
but the SELECT cannot find still errors rather than inventing an id.

MEASURED by parsing the corpus directly: 423 top-level declarations, 26
colliding, ALL 26 `val` — no class/object/interface/fun collides.

Known and unchanged: the canonical node's `file_path` is last-writer-wins across
declaring files. Lossy, not fabricated, pre-existing.

### B (original description, kept for context)

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

## #4 KOTLIN MEASURED LIVE (deployed, corpus reindexed)

| metric | before | after |
|---|---|---|
| calls | 0 | **3,150** (599 resolved / 2,551 honest-unresolved) |
| extends | 0 | **43** |
| implements | 0 | **20** |
| interface nodes | 0 | **12** |
| enum nodes | 0 | **7** |
| definition nodes | 1,992 | **2,353** (+361 companion/enum members) |
| phantom code nodes | — | **18** (0.6% of the new edges) |

The 18 is what justifies defs→heritage→calls ordering: a naive java port would
have minted one per bare call, ~3,496. Resolved calls split 425 lib / 174
first-party; heritage 56 lib / 7 first-party (an Android app extends framework
classes).

## GRAPH-WIDE ASSESSMENT — measured, supersedes any earlier estimate

### Call edges are NOT complete. 46% honest, 19% fabricated, 35% unresolved.

| lang | calls | real def | lib | PHANTOM | unresolved | honest % |
|---|---|---|---|---|---|---|
| typescript | 150,235 | 36,411 | 51,355 | 9,926 | 52,543 | 58.4 |
| **java** | 119,429 | 16,107 | 15,557 | **47,616** | 40,149 | **26.5** |
| rust | 81,512 | 15,919 | 26,109 | 11,485 | 27,999 | 51.6 |
| javascript | 19,711 | 4,938 | 5,154 | 1,045 | 8,574 | 51.2 |
| python | 4,992 | 904 | 1,864 | 885 | 1,339 | 55.4 |
| kotlin | 3,150 | 112 | 425 | 62 | 2,551 | 17.0 |
| svelte | 2,170 | 651 | 303 | 370 | 846 | 44.0 |
| sql | 1,164 | 894 | 0 | 270 | 0 | 76.8 |

**JAVA IS NOW THE LARGEST FABRICATION PILE: 47,616 phantoms, 40% of its call
edges** — bigger than typescript's ever was. Same shape as TS's #3: java's
`resolve_call` unqualified arm mints
`fqn::method(JAVA_LANG, package, "", class, method)` on the "an unqualified call
is a method on the enclosing class" rule, which is false whenever the method is
inherited from a lib supertype or came from a static import the map missed. It
needs the member/locals probe #3 gave typescript. THIS IS THE HIGHEST-VALUE
REMAINING FIX IN THE WHOLE SLICE.

### Inheritance: 4 of 8 languages. TS/JS/SVELTE HAVE NONE.

java 1,166 extends + 1,002 implements, rust 753 implements (trait impls), python
389 extends, kotlin 43 + 20. **typescript/javascript/svelte: ZERO** —
`typescript.rs` never touches `relations` (0 occurrences), yet 33 TS/svelte
classes declare `extends`/`implements` in THIS repo alone. Direct analogue of
what kotlin just got.

Also: `capability_matrix()` reports `fqn` and `scope` only — there is NO
inheritance capability cell, so the system cannot self-report this gap.

### OO DESIGN PATTERNS ARE NOT DETECTED AT ALL.

`inference.detected_patterns` (1,451 rows, `family` NULL on ALL — still) detects
WORKFLOW patterns: `rule-candidates`, `correction-prone`, `rework: <file>`. No
adapter, factory, strategy, repository, decorator. Nothing structural.

### Node-kind coverage is uneven

- rust: class, const, enum, file, function, interface, method, module, struct, type
- typescript / kotlin: no struct, otherwise full
- java: NO const/field, no module
- python: THINNEST — class, file, function, method, module only (no const, enum, protocol)
- svelte: component, file, function, hook, interface, module, type (no class/method)

### Docs and tests are well covered

markdown is the LARGEST language by nodes: 154,771 (146,514 section, 6,398 doc,
1,743 rationale). Test nodes flagged: typescript 17,913/78,943, java
7,411/81,868, rust 2,880/27,259.

### Depth works, but is truncated by the linkage

Real def-to-def rust chains: 15,919 at depth 1 → 2,636 still alive at depth 12
(bounded by the query, not the data). So depth IS computable and non-trivial —
but it rides on ~20% real linkage, so absolute complexity numbers systematically
UNDER-report. Usable for relative comparison within a language; not yet as an
absolute measure.

## JAVA PHANTOMS — DIAGNOSED. It is a STUB-LIFECYCLE gap, not a resolver bug.

My own two hypotheses were REFUTED by measurement. Do not repeat them:

1. "Most of the 47,616 is stale, from repos not reindexed since #2/#2b." FALSE.
   Reindexed `Dayamed/cluster` deliberately to test it: phantoms went
   **18,174 -> 29,814**. The old number was an UNDER-count from a partial index.
   Java total is now **59,256**.
2. "The dominant arm is `resolve_call`'s unqualified -> enclosing-class mint."
   FALSE — it is the SMALLEST at 4%. Also refuted: "wildcard static imports are
   the cause" — there are 61 wildcards against 2,382 explicit static imports.

MEASURED distribution, by comparing each phantom target's package/class against
its caller's:

| arm | edges | share |
|---|---|---|
| C: imported type, target has NO source def | 40,520 | 68% |
| B: receiver assumed same-package | 16,304 | 28% |
| A: unqualified -> enclosing class | 2,432 | 4% |

C's head is Lombok: `SurescriptDetails·setId/getId/setStatus`,
`PatientModel·getUserDetails` — `@Data` entities whose accessors are generated at
compile time and genuinely absent from source. The CLASS is named correctly; the
METHOD does not exist to be found.

### THE ACTUAL MECHANISM

- A reference creates an unresolved stub; a later definition with the same
  `(folder_id, fqn)` MERGES into it and flips `resolved=true` (the SCIP/LSIF
  moniker model, `upsert_node_by_fqn_merges_ref_and_def`). Legitimate and
  necessary — a ref can precede its def.
- `prune_orphan_stubs_scoped` collects only stubs with NO EDGES AT ALL
  (`NOT EXISTS … e.target_id = n.id OR e.source_id = n.id`, graph.rs:1407).

So **a stub with inbound edges whose def never arrives is PERMANENT by
construction.** Nothing reconciles it once ordering is no longer a factor.

### THE FIX (not implemented — next task)

At the SAME terminal barrier `mark_folder_indexed_fail_closed` uses (after every
ProcessFile for the folder has drained, so ordering cannot explain a miss): for
each `resolved=false AND file_path IS NULL` node in the folder, unresolve its
inbound edges (`target_id = NULL`, KEEP `target_name`) and delete the node. The
primitives already exist and are used for removed files —
`unresolve_edges_to_file` + `delete_nodes_by_file`.

LANGUAGE-AGNOSTIC and it is the single biggest honesty win left: java 59,256,
typescript 9,926, rust 11,485, javascript 1,045, python 885, svelte 370, sql 270
— roughly 71,000 fabricated call targets become honest-unresolved.

CAUTION for whoever implements it: the barrier condition is load-bearing. Run it
before the folder's files have all drained and it will delete stubs whose def was
merely late, turning real edges into unresolved ones. Verify against
`upsert_node_by_fqn_merges_ref_and_def` and the D4.1 community barrier.

## STUB RETRACTION — BUILT, ADVERSARIALLY REVIEWED, REVERTED. DO NOT RETRY AS SPECIFIED.

I implemented `retract_undefined_stubs` at the D4.1 barrier (unresolve inbound
edges, delete the node), it passed its own red test and the full 2,776-test gate,
and a 4-agent adversarial review then found it UNSAFE on measured grounds. It is
reverted. The findings below are worth more than the fix was.

### Blocker 1 — THE BARRIER IS NOT TERMINAL. This kills the whole premise.

- `analyzer_scheduler.rs:252` enqueues `DetectCommunities` with plain `enqueue`,
  **no `blocked_by`** — daily, per indexed folder. It can fire mid-`process_file`.
- `queue.rs:321` on failure: *"Still unblock dependents (they'll see partial data
  but won't deadlock)"* → a transiently-failed ProcessFile's retry lands 2-8s
  AFTER the barrier already ran.
- The housekeeping call sits BEFORE the status gate, and `community.rs`'s own test
  `detect_communities_is_fail_closed_on_failed_folder` proves the barrier RUNS on
  `failed` folders — exactly where a definition is missing because its file never
  processed.

So "every file that could define this has been processed" is FALSE. The existing
`prune_orphan_stubs` survives this only because a stub with no edges at all is
garbage regardless of timing.

### Blocker 2 — THE PREMISE IS ~48% FALSE. The def exists under a DIFFERENT fqn.

15,407 of 32,087 edge-bearing stubs have a sibling node with the SAME `name`, the
SAME `folder_id`, and a real `file_path`. The definition is not absent — THE
RESOLVER BUILT THE WRONG FQN. Verified rust case: stub
`rust·<pkg>·postgres::tests·PostgresContentStore·new` against the real
`rust·<pkg>·postgres·PostgresContentStore·new` — 1,361 of 4,404 rust stubs
recover by deleting one `::tests` module segment. Retracting these would DESTROY
a recoverable link and hide a resolver bug.

### Blocker 3 — `edges_unique_unresolved` aborts the UPDATE

`unique (folder_id, source_id, target_name, target_file, kind) NULLS NOT DISTINCT
WHERE target_id IS NULL`. All 88,094 inbound edges have `target_name IS NULL`
(because `insert_edge` stores the name ONLY on the `target_id IS NULL` branch),
so `SET target_id = NULL` alone violates it on 59,586 rows. Measured collisions:
NULL 59,586, `n.name` 2,357-3,478, **`n.fqn` 0**. My code used `n.name` — still
broken.

### Blocker 4 — `nodes_parent_id_fkey` is ON DELETE CASCADE

42 predicate-matching nodes are parents of **593 REAL file-bearing definitions**.
Without the children guard the delete destroys them. (My implementation DID carry
the guard — but the naive predicate as written in the spec does not.)

### Also must-handle

`edges_target_id_fkey` is ON DELETE CASCADE, so unresolve-then-delete must be ONE
transaction (`prune_file_nodes` at graph.rs:1460 is the precedent using
`pool.begin()`); ~41,294 of the matched nodes carry a `community_id` and the
existing emptied-community cleanup is gated on the OTHER delete's row count; and
NO existing test would have caught a late-def deletion.

### THE REAL FIX THIS EXPOSED, and it is better

Blocker 2 is a genuine resolver defect, not a lifecycle one: a reference made from
inside an inline `mod tests` is qualified with the CALLER's module path
(`postgres::tests`) instead of the TYPE's own canonical module (`postgres`). Fix
that and 1,361 rust stubs become REAL def-to-def edges — an increase in honest
linkage rather than a retraction of fabrication. The checkpoint already noted the
cause: "local_types is module-flat, so names inside `mod tests` anchor on the
tests module."

## RUST MODULE ANCHORING — TYPES FIXED AND MEASURED (`9ed5f2d1`). FUNCTIONS REMAIN.

`FileScope::local_types` was a flat set, so `resolve_type_module` fell back to the
CALLER's walk position; inside `mod tests` that is `<module>::tests`. Now a
`HashMap<name, declaring_module>`, `collect_scope` carries the module path and
appends each inline `mod`, and both readers return the DECLARING module.
`resolve_trait_fqn` lost its now-dead `module` parameter.

MEASURED after deploy + reindex of this repo: global `::tests`-qualified rust
stubs 1,981 -> 1,851. Scoped to this repo, 562 remain — and ALL 562 are
`kind = function`.

### THE REMAINING HALF, quantified

`collect_scope` tracks only `struct_item | enum_item | trait_item | type_item |
union_item`. There is NO `function_item` arm, so a FREE FUNCTION declared at file
level and called from inside `mod tests` still resolves against the caller's
module. Split of the 562:

- **460 are file-level functions MISQUALIFIED** — a resolved def exists at the
  stripped fqn. Example, from today's own work:
  `rust·senseid·tasks::handlers::scan_logic::tests·symlink_repository_links`
  against the real `…::scan_logic·symlink_repository_links`.
- 102 are genuinely declared INSIDE `mod tests` (no def at the stripped fqn) —
  test helpers whose defs are not emitted. A DIFFERENT defect; do not conflate.

FIX: a parallel `local_fns: HashMap<name, declaring_module>` populated from
`function_item` in `collect_scope`, consulted on the bare-call path. Same shape
as the type fix, same red-test pattern. ~460 recoverable in this repo; if the
ratio holds across the 1,851 global, ~1,500.

### THE JAVA EQUIVALENT — AND MY LOMBOK CLAIM WAS WRONG

I told the user java's 40,520-edge bucket was Lombok-generated accessors. The
review REFUTED that: it verified `setDateAdded`, `setTitle`, `getQuestionText`
are HAND-WRITTEN, in source, and INDEXED. The real cause is `jvm.rs:87` minting
the target under the CALLER's package when the receiver class is not in the
import map — e.g. stub `java·com.rfs.admin.service.impl·OnboardingConfiguration·
setDateAdded` against the real `java·com.rfs.admin.domain·OnboardingConfiguration·
setDateAdded`. Measured 1,718 of 16,693 java stubs recover by same-class +
same-method lookup in the same folder. Same defect class as the rust one:
a derivation using the caller's position instead of the declaration's.

Also measured by the review, and it kills any "def never arrives" argument:
20,467 fqn nodes flipped `resolved` inside one six-minute window while the stub
set fell 41,298 -> 32,129.

## BOTH DERIVATION FIXES SHIPPED AND MEASURED LIVE (`cb9fc944`)

One defect class, two languages: a call target derived from the CALLER's position
instead of the DECLARATION's.

### rust free functions — RECOVERY

| scope | before | after |
|---|---|---|
| `::tests` stubs, this repo | 562 | **105** (−81%) |
| real def-to-def call edges, global | 16,173 | **17,348 (+1,175)** |
| phantom call edges, global | 11,235 | **10,056 (−1,179)** |

The residual 105 matches the 102 previously identified as `let`-bound CLOSURES in
test bodies (`let r = |n| …`) — those have no definition node and are correctly
unresolvable. Not a defect; do not chase it.

### java wildcard imports — HONESTY

| scope | before | after |
|---|---|---|
| phantom call edges, rfsjava | 3,641 | **150 (−96%)** |
| phantom call edges, global java | 40,099 | **36,608 (−3,491)** |
| real def-to-def, global java | 16,355 | 16,332 (flat) |

Flat real_def is EXPECTED and correct: this fix declines to guess rather than
recovering. `import com.acme.domain.*;` binds no simple name, so an unqualified
type is ambiguous between every wildcard package and same-package. With NO
wildcard, java's own rule makes it same-package and `ctx.package` stays correct —
both directions are pinned by the test, so this is not a blanket retreat.

**Combined: 4,670 fabricated call edges removed, 1,175 of them replaced by REAL
def-to-def links rather than merely unresolved.**

### AND IT CORRECTED A CLAIM I HAD MADE

I attributed java's largest phantom bucket to Lombok-generated accessors. Wrong —
the adversarial review verified `setDateAdded`, `setTitle`, `getQuestionText` are
hand-written, in source, and indexed. The cause was this derivation.

### THE STANDING LESSON FOR THIS WHOLE SLICE

Every real fix took the same shape: **derive an fqn from where the symbol is
DECLARED, never from where it is referenced.** #3 (ts locals probe), #2b (java
supertype), #3b (src/ anchor), rust types, rust functions, java wildcards — all
one rule. When a target cannot be derived that way, emit unresolved. The
stub-retraction attempt failed precisely because it treated the SYMPTOM of a
mis-derivation as absence.

### REMAINING, and none of it is a fabrication

- rust `let`-bound closure calls (~105 here) — unresolvable by nature.
- typescript/javascript/svelte INHERITANCE: still zero. `typescript.rs` never
  touches `relations`, yet 33 TS/svelte classes declare `extends`/`implements` in
  this repo alone. Direct analogue of what kotlin received.
- `capability_matrix()` has no inheritance cell, so that gap cannot self-report.
- OO DESIGN patterns (adapter/factory/strategy) are not detected at all;
  `inference.detected_patterns` does workflow patterns and its `family` is NULL
  on all 1,451 rows.
- defect A: an indexer code change still cannot trigger a reindex — delete the
  folder's `scan_state` rows and scan the WATCH ROOT.

## FINAL STATE — the remaining backlog items are DONE

### TS/JS/SVELTE INHERITANCE (`5b3cc0e5`) — 0 -> 239 edges, ZERO phantoms

`typescript.rs` never wrote `out.relations`. Now emits extends/implements,
resolved through the SAME ladder a call target uses: external import -> `lib·`,
local import -> sibling module, same-file -> here, otherwise UNRESOLVED. A
computed superclass (`extends mixin(Base)`) is skipped rather than guessed.
`sfc_fqn_output` also forwarded defs and refs but NOT relations — that was the
whole capability for svelte/vue.

Measured after reindexing two repos: typescript extends 208 (6 lib, 95 real def,
95 honest-unresolved), typescript implements 26 (24 real def), javascript
extends 5. **119 to real definitions, 7 to lib, 0 phantoms.** The OO structure is
now visible and pattern detection is unblocked:

    RedisQuotaStore  implements  …lib/quota/types·QuotaStore
    SqliteQuotaStore implements  …lib/quota/types·QuotaStore
    MockTransport    implements  …lib/health-transport·HealthTransport
    MapLocalStorage  implements  Storage (unresolved — a DOM global, not invented)

### THE GAP IS NOW SELF-REPORTING

`capability_matrix()` reported `fqn` and `scope` only, so an empty `relations`
list could not distinguish "no producer" from "nothing to report" — which is
exactly why this hid for so long. Added `emits_inheritance`, defaulting false,
inherited by a framework from its host, and PROBED against a per-language
fixture. The probe earned its place immediately: it caught that
`JavaScriptAdapter` had not declared the capability despite calling the identical
`typescript_fqn::produce_fqns`.

### DEFECT A FIXED (`166cd89d`)

`version_rescan` promised a rebuild and delivered none — `plan_reindex` has no
indexer-version component, so a content-identical file is skipped however much
the producer changed ("0 changed files, 9822 unchanged"). It now clears
`scan_state` for the root it is rescanning. NOT exercised by `make install-debug`
(VERSION stays 0.9.1, so `version_changed` is false); it takes effect on the next
real `make bump`.

## WHAT IS GENUINELY LEFT — none of it is a fabrication

- **OO DESIGN PATTERNS** (adapter/factory/strategy) are still not detected.
  `inference.detected_patterns` does WORKFLOW patterns and its `family` is NULL
  on all 1,451 rows. Now UNBLOCKED — inheritance edges are the substrate it
  needed. This is a new feature, not a defect.
- rust `let`-bound CLOSURE calls (~105 here) — unresolvable by nature, correct as
  unresolved.
- java's remaining ~36,600 phantoms after the wildcard fix: the review measured
  1,718 as same-folder mis-qualifications; the rest need per-case diagnosis, NOT
  a blanket retraction (see the reverted-retraction section above).
- python is the thinnest producer: class/file/function/method/module only — no
  const, enum, or protocol.

## THE RULE THAT ENDED THE CHURN

Every fix that worked took one shape: **derive an fqn from where a symbol is
DECLARED, never from where it is referenced.** #3 ts locals, #2b java supertype,
#3b src/ anchor, rust types, rust functions, java wildcards, ts inheritance. When
the declaration cannot be determined, emit UNRESOLVED. The one attempt that
failed — stub retraction — failed because it treated the SYMPTOM of a
mis-derivation as absence, and would have deleted 593 real definitions via an
ON DELETE CASCADE while doing so.
