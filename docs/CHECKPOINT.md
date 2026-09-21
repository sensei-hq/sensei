# Checkpoint

**Slice** — indexer-v2 **stage 11** (`docs/spec/indexer/11-file-index.md`):
one file in, nodes and edges out. Rust only (§11). **The walk is table-free and
`index_file` exists. §10's resolved-reference bar is NOT met: 94,851 against
≥96,304, 1,453 short.**

**Done, in order** — `3852a199` S8 (method keyed on type+name) · `12d5c096`
impl-header anchoring · `a13e1e65` collapsed-spelling table deleted ·
`b75e1853` S7 Named/Candidate · `5c51b22e` S8 narrowed (inherent wins) ·
`cb2a06f4` I7 guards read the driver · `0e7baad3` I8+I9 `index_file` + Delete
mode · `50f42e35` I6a `Home::Unstated` · `9c7622cf` I6b rust walk drops
`TypeHomes` · `ddda0619` I6c glob-root rung.

**MEASURED per increment** (resolved / total):

    baseline  96,386          S8      96,418        impl anchor 96,420
    table out 96,336          S7      97,049        S8 narrowed 97,088
    I7        97,096          I8+I9   97,176        I6a         97,190
    I6b       93,801 (-3,389) I6c     94,851 (+1,050)

**THE OPEN GAP — 1,453 references, and it is the next task.** I6b removed the
table and cost 3,389; I6c's glob-root rung returned 1,050. Where the remaining
2,339 sit has NOT been measured — do that before writing any more code. Prime
suspects, in order: an inline qualified path longer than `<Ty>::<member>`
(`considered_path` returns nothing for it); files with a package-rooted glob
the one-glob rule refuses; `Home::Ambiguous`, whose last producer died with the
table. Run `barrier_necessity` — it classifies every resolved member reference
by what its own file states, and is built for exactly this question.

**Measurement rules, learned the hard way, now in the tests** — (1) the corpus
IS this repo: read BOTH columns AND the total. Both falling = smaller corpus
(I3, -84, not a regression). Columns moving against each other with a flat
total = a real resolution change (I6b). (2) A ratchet that prints nothing on
success can only be lowered by editing the test.

**Ratchets** — A7 rust 3 (S8's raise to 4 was REVERTED by the narrowing).
Split-impl anchoring 12. References naming an identity no declaration mints
1,300 — `docs/backlog.md` holds the stage-12 obligation: resolve them or mark
them EXTERNAL, never leave a ghost.

**Next** — measure the 1,453 first. Then I11 (the conservation property is
ALREADY BUILT and green at `rust/mod.rs`
`the_symbol_count_equals_an_independent_count_of_declaration_nodes`; what is
missing is the unnameable-container term). Then I12 (record), then stage 12.

    cargo test -p senseid --bin senseid -- indexer::
    cargo test -p senseid --bin senseid -- --ignored --nocapture index::corpus
    cargo test -p senseid --bin senseid -- --ignored --nocapture indexer::index::barrier_necessity

**Spec defects found** — S1's claim that the guard covered the driver was FALSE
(fixed, spec amended). §5/§8 draw the `TraitImpl` edge trait→type; the code and
the shipped `implements` column run type→trait — code kept, spec needs
correcting. §4.2's `via` list omits `NamedByThisFile`. §7 defers the wildcard
export list to stage 12 while §10 demands ≥96,304 at stage 11 — those two
cannot both hold; §10 is the one under pressure.

**Gate at HEAD** — fmt clean, clippy `--all-targets -D warnings` 0, indexer
447/0. 4 ignored tests fail environmentally (dbd-rs path, gateway config, 2
installer hooks), none in the indexer. `cargo clean -p senseid` between
increments keeps `target/` at ~5 GB; it reaches 12 GB in about three builds.
