# Checkpoint

**Slice:** Observatory diagrams — data → derivation → screens. Lanes 1 (data) and
2 (derivation) are DONE for the architecture set. Nothing is blocked on data any
more; what is left is screens, plus two foundations re-prioritised on 2026-10-05
(persistence fold-in, resolution attribution).

## Next — autopilot queue (in order)

Sequence re-set 2026-10-05. Each item sits where nothing earlier depends on
anything later; the reasoning is below the list, not implied by it.

    [x] #215 registering a root enqueues a scan             91809876
    [x] #216 watcher batch needs no database                91809876
    [x] #224 git history scanner                            964a99b6
    [x] #222 derivation: SCC + layering                     83c9f9ab
    [x] #223 derivation: Zones (+ the matrix needs none)    8ec2fa6c
    [x] #217 governance seed drift — ponytail is opt-in     6cbba6ee

    -- A. persistence (settle the pattern before 30 more land on it) --
    [x] #161 8+-arg writers → typed rows   8fe58786 bfea831e a1e45890
        10 row types, 0 functions left at 8+. 41 remain in the 5-7 band,
        left open deliberately — that tail is multi-key lookups, not
        a producer's struct spelled out, so it is a different risk.
    [x] #227 fold the 30 into PgStore + a gate   4bbd63ae 44256959
        ZERO production SQL outside db/pg_store/. 25 metric reads in
        metric_reads/ (one module per group), gateway catalogue + the
        federation lookup folded, day_filter/bind_day moved and the
        handler-side duplicates deleted. check-sql-in-layer.py in
        make test-fast, verified by planting a statement.

    -- B. indexing truth (the instrument before the fixes) --
    [x] #235 resolution by (language, kind, outcome)        6426808b
        sensei.resolution_quality(project). Corrected its own premise:
        97.2% of import edges were never CLASSIFIED (→ #242), so the
        2.6% figure divided by a population nobody asked about.
        csharp calls place at 15.5% vs java 43% vs corpus 29.6%.
    [x] #242 97.2% of import edges are never classified  0a667709 4cc6dc2b
        `Import` carries a `target: Resolution` like `Reference` does;
        `resolve` places it through `Ladder::place_brought`. External
        imports now land on LIBRARY SURFACE by `fully_qualified_external`,
        the same door a call to that member uses — so "who depends on
        serde" is answerable from the one edge kind that names a
        dependency. `place_entered` delegates, which also collapsed a
        DOUBLE ROW: one `use` line wrote two `imports` edges whenever the
        specifier named a module (7 rows for 6 lines in playbook/mod.rs).
        Found and fixed on the way: `upsert_lib_node_by_fqn` stamped
        `modified_at = now()` unconditionally, so a re-scan churned two
        rows per external dependency.
        Corpus after: 9,640 of 9,654 imports placed (99.86%), 14 faults,
        ZERO no-verdict; both totals reconcile. 7766dfd1 puts the two
        tables in the acceptance report so it cannot regress unseen.
        NEEDS A RE-INDEX to move the live DB — the placement comes from
        the walk, so backfill-edge-verdicts.sh cannot help, and
        DEPLOYING IS BLOCKED BY #243.
    [x] #243 make install could not build at all                1f09b5d9
        llama.cpp's vendored cpp-httplib vs OpenSSL 3.5+. Turned its
        TLS off through a cmake prelude — sensei embeds llama.cpp for
        INFERENCE and never serves HTTP from it. Verified on the shipped
        binary: daemon up, embedded-llama present, a real embedding back.
    [x] #231 fqn segment 3 is a SYMBOL for some adapters
        f4daee2b fd1f9368 297e3cd0
        `join` dropped an empty module, sliding the symbol's NAME into
        the module's slot. It now drops only a TRAILING empty, so
        `lib·tokio` is unchanged and the module is segment 3 always.
        Closed a REAL collision too: the ratchet in
        `no_two_shapes_mint_one_string` went from one pair per language
        to ZERO — `Config::load` at a root and `load` in a module
        `Config` were one node. Both minters changed; `module_of` answers
        the root case with the package's own name.
        NEEDS A RE-INDEX, like #242 — every package-root identity moved.
    [ ] #231 fqn segment 3 is a SYMBOL for some adapters   ← NEXT

    -- C. screens --
    [x] #232 Layers + Cycles screens          d0f191d7 1690a5d7
        Two screens, one controller, under a Diagrams sub-nav. All three
        cautions honoured: `skip` is NOT a break (25 climbs vs 15 skips
        live, so counting both would overstate by 60%), unknownUnit sits
        beside the picture, and zero is a FINDING with words rather than
        an empty canvas. Verified in a browser against project `sensei`:
        154 units / 4 layers / one 22-member cycle cut at
        senseid/adapters → senseid/tasks ×2 / 21 self-dependencies.
        Found + fixed on the way: the PRESSED chip was invisible on every
        diagram screen — `[type="button"]` in the preflight ties
        `.bg-primary` on specificity and wins by order (#244).
    [x] #244 a background utility could not paint a typed button
        ca691586 — one line: the reset goes in a cascade layer.
        I was wrong TWICE in that issue and corrected both: the
        population is 13 sites across 7 screens (not 2), and Observatory
        Instruments was never affected — its chip has no `type`
        attribute, so the reset reaches it at (0,0,1) and loses cleanly.
        The bug is specifically `[type="button"]` at (0,1,0), which TIES
        the utility and wins on source order.
    [!] #245 the app e2e gate is RED — 117 passed / 21 failed / 17 skipped
        Pre-existing, four unrelated groups (fixture gaps, a harness bug,
        missing surfaces, a11y contrast at 4.47:1 against a 4.5:1 floor).
        Established by diff that none can come from the diagrams work.
        All four new diagram specs pass.
    [x] #233 diagram read cost                              1da79013
        Decided WITH the screens, which is what the issue asked for:
        the access pattern is INTERACTIVE (grain + kinds are controls,
        and Layers/Cycles share one payload), so a cache fixes every
        request after the first — where the interaction lives. Keyed on
        `max(files.indexed_at)` per project, so it cannot go stale; NOT
        `folders.modified_at`, which moves without the data. Bounded at
        64, LRU. Precomputation (the issue's options 1/2) is still the
        answer for FIRST paint and is not foreclosed — it needs exactly
        this version marker to know when it is stale.
        LIVE TIMING PENDING: the cache cannot hit while the re-index
        moves the version every few seconds.
    [ ] #219 World · #220 Neighbourhood · #221 Schema

    -- D. consent + dōjō --
    [ ] #218 transcript ingestion opt-in
    [ ] #236 dōjō → kavach data routes
    [ ] #225 dōjō team view (needs the sync-shape decision)

    -- new, after the cycle (2026-10-06) --
    [ ] #238 Claude Code mods — capture, usage at source, model-by-risk,
        unattended drive, tool gating. Fixes four live defects on its own.
    [ ] #239 EPIC productionise sensei + dōjō
          ├ #240 band A — user-scoped metrics have no path to the dōjō
          └ #241 band C — shipped dōjō screens render fixture values

    -- deferred by decision --
    [ ] #237 C#/JS indexing coverage — after #235, so the delta is readable
    [ ] #230 declared layering · #234 pack adoption UI — both need a product call

Per slice: TDD, mutation-probe each test, full suite + clippy + fmt +
`scripts/check-sql-against-schema.py`, then close the issue with evidence.

## Why that order

- **#161 before #227.** #227 adds ~30 `PgStore` methods; #161 is converting that
  family off positional arguments. The reverse order writes 30 functions in a
  style already decided against — and `upsert_project_metric_repo` (10 positional
  args) is on #161's list AND in #227's population.
- **#235 before #231 and #237.** A corpus-wide resolution percentage mixes "never
  indexed" with "indexed and the resolver missed". Changing either without the
  breakdown means changing it blind, and arguing afterwards about what moved.
- **#232 before #233.** The right caching answer (matview · refresh-on-barrier ·
  daemon cache) depends on how a real screen consumes the payload.

## Open questions

- #230 is NO LONGER BLOCKING (2e898607). "Which calls break the downward flow"
  is answered by a cycle's back edges — 25 of them on project `sensei`, led by
  `senseid/db → senseid/tasks ×149`. A declared layering is now only for the
  narrower "does the measured structure match the one you intended".
- #234 nothing lets a developer ADOPT a seeded rule pack. DECIDED 2026-10-05:
  the MCP tool pair (`list_available_packs` / `adopt_pack`) writes it first;
  CLI and app screen follow if they earn it.
- `sensei.assistants` is an EMPTY table with no reader or writer (4 tree
  references, all prose). #218's consent should use a `sensei.config` key; the
  table itself wants a separate give-it-a-writer-or-drop-it call.

## Known broken

- **#242 97.2% of import edges are never CLASSIFIED** — 251,270 of 258,623 carry
  neither `resolved_via` nor `unresolved_reason`. Not a resolver failure; nothing
  attempted them. `references` has 2 such rows of 1.69M, `calls` 4,421 of 1.35M,
  so it is imports-specific. This made "imports resolve at 2.6%" meaningless.
- **C# resolves worst AND is indexed least** — `calls` place at 15.5% for csharp
  against 43% for java and 29.6% corpus-wide, and only 52.3% of its tracked files
  carry a node. Two defects stacking (#237).
- **#237 ~11,100 tracked files have no nodes**: C# 52.3%, java 79.9%, js 78.4%,
  py 74.2%. Cause NOT established — check whether those 9 folders ever finished
  before suspecting the adapter.
- **#231 an fqn's segment 3 is a SYMBOL for some adapters, not a module.** 13.7%
  of `sensei`'s module dependencies and 78% of the largest client project's
  point at a unit that owns no file. Counted as `coverage.unknownUnit`; the
  shipped Structure diagram has the same hole (31 of 213), uncounted.
- **#238 four live defects in the hook tap, measured**: no token/cost data in
  ANY hook payload (0 of 5,000 recent rows); `assistant_events.success` dead
  (31 non-null of 554,999); the SessionStart hook's session creation has NEVER
  succeeded (0 rows with task='session'); the nudge hook blocks on every tool
  call (254,404 PreToolUse vs 536 SessionStart). Plus `duration_ms` present on
  5,000/5,000 payloads and read by no SQL — that one needs no mods.
- `create_memory` has NO production caller (all 17 sites are tests), and
  `sensei.memories` has three writers. Now `#[cfg(test)]`; which one production
  uses is unanswered.
- `project_health` and `unused_tools` have NEVER produced a metric row despite
  being effective since 2026-08-09. `cost` last computed 2026-09-20 while every
  other family ran 2026-10-05, with a subscription configured.
- **dbd#41 blocks `dbd reconcile` on any database `dbd deploy` created.** It
  emits a false `ALTER TYPE` for a column whose extension type lives off the
  search_path — the configuration `design.yaml` asks for. Workaround: apply the
  one changed file with `psql -f`, or drop and redeploy `sensei_test`.
- **App e2e 114/21/17 skipped** — the known red gate (#187). Sampled causes are
  harness defects, not this slice.
- `--features senseid/embedded-llama-cpp` will not compile (#203). `EMBED=0`
  works on every make target and is how the live daemon was built.
- 70+ commits UNPUSHED on `develop`. Otherwise green: senseid 3270/0, clippy
  `-D warnings` + fmt, dbd doctor, SQL statements plan against a fresh deploy
  (and that planner now runs in CI).
- Two unattributed single-run failures, both passing on rerun and in isolation:
  `audit_repairs_nested_standalone` and `prune_empty_projects_…`. The latter uses
  fixed project names and runs a GLOBAL prune, which is the #183 class. The one
  race I could name — two tests fighting over the `cost.subscription` config key
  — is fixed with a mutex.
