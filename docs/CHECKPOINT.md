# Checkpoint

**Slice** — indexer-v2 **stage 11** (`docs/spec/indexer/11-file-index.md`).
Rust only (§11). The walk is TABLE-FREE, `index_file` exists, and rust identity
collisions are at ZERO. **§10's done-gate is NOT met: 94,927 resolved against
≥96,304, short by 1,377.**

**HEAD** `6b1e68c3`. Gate: fmt clean, clippy `--all-targets -D warnings` 0,
indexer **450/0**, A7 rust **0** (511 total, all typescript).
Corpus: 94,927 resolved / 98,793 unresolved / 193,720 total.

## Done, in order

`3852a199` S8 method keyed on type+name · `12d5c096` impl-header anchoring ·
`a13e1e65` collapsed-spelling table deleted (391 lines) · `b75e1853` S7
Named/Candidate · `5c51b22e` S8 narrowed (inherent wins) · `cb2a06f4` I7 guards
read the driver · `0e7baad3` I8+I9 `index_file` + Delete mode · `50f42e35` I6a
`Home::Unstated` · `9c7622cf` I6b rust walk drops `TypeHomes` · `ddda0619` I6c
glob-root rung · `fa37eb3f` method-body locals · `81f1552d` cfg variants ·
`6b1e68c3` anonymous `const _`.

## Resolved-reference count, per increment

    baseline  96,386   S8      96,418   impl anchor 96,420   table out 96,336
    S7        97,049   narrow  97,088   I7          97,096   I8+I9     97,176
    I6a       97,190   I6b     93,801   I6c         94,851   locals    94,860
    cfg       94,921   const _ 94,927

## THE TWO OPEN ITEMS — both in `docs/backlog.md` with full design and examples

1. **Where the 1,377 are.** NOT located. The next measurement is the
   UNRESOLVED side of `index::barrier_necessity`, decomposed the same way its
   resolved side already is. Three ranked hypotheses are written down, each
   with the check that settles it. None is verified — treat them as such.
2. **`Form::MemberVariant`.** A `cfg`-gated MEMBER cannot carry its condition.
   The reason first recorded — "the grammar has no shape for it" — was WRONG
   and is corrected in the backlog: a form has to be added, and the design is
   there. Costs a missing fact, not a wrong edge (all seven are single-arm).

## Root causes, so they are not rediscovered

- **The corpus IS this repository.** Deleting production code lowers `resolved`
  with no resolver change. Read BOTH columns AND the total: both falling is a
  smaller corpus (I3, −84, not a regression); columns moving against each other
  with a flat total is a real change (I6b, −3,389). Now printed by the test.
- **A ratchet that prints nothing on success** can only be lowered by someone
  who edits the test to find out where it stands. The split-impl share prints.
- **At ZERO a ratchet becomes an invariant.** `<= 0` on a `usize` reads as a
  ceiling and means equality; A7 rust is now `==`.
- **A fixture cannot tell two rules apart that the corpus can.** Twice this
  slice: `!fn_scope.is_empty()` vs the depth rule (passed the fixture, collapsed
  two fields in `transcript/zed.rs`), and the cfg condition CARRIED AS STATE
  (passed the fixture, leaked past `#[cfg(unix)] let cmd = ..` onto the next
  declaration). Both caught only by the corpus conservation property.
- **A7 went 19 → 3 → 2 → 1 → 0 by REPAIR, never by tolerance.** Each step is a
  defect that had been sitting on a known-list: a local named under its
  function, that rule reaching a method body, a cfg pair becoming callable+arms,
  an anonymous `const _` ceasing to be a symbol it never was.

## Spec defects found

- **S1's claim that the guard covered the driver was FALSE** — `guard_sources`
  did not read `index.rs`, so six guards passed over it. Fixed, spec amended.
- **§5/§8 draw the `TraitImpl` edge trait→type**; the code and the shipped
  `implements` column run type→trait. Code kept; spec needs correcting.
- **§4.2's `via` list omits `NamedByThisFile`**, which §5's own worked output
  uses.
- **§7 defers the wildcard export list to stage 12 while §10 demands ≥96,304 AT
  stage 11.** Both cannot hold. One must be amended, not worked around.

## Next commands

    cargo test -p senseid --bin senseid -- indexer::
    cargo test -p senseid --bin senseid -- --ignored --nocapture index::corpus
    cargo test -p senseid --bin senseid -- --ignored --nocapture index::barrier_necessity

Four ignored tests fail environmentally (dbd-rs path, gateway config, 2
installer hooks); none in the indexer. `cargo clean -p senseid` between
increments keeps `target/` near 5 GB — it reaches 12–19 GB in about three
builds, and a full `cargo clean` reclaimed 306 GiB once.
