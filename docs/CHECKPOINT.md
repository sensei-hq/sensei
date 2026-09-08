# Checkpoint

**Slice:** indexer v2, rust. Spec `docs/design/indexer-v2.md`, plan
`docs/plans/indexer-v2-rust.md`.

## Done — steps 1-4 of 10, committed 3a777a20

`crates/senseid/src/indexer/` — facts.rs, fqn.rs, lang/rust.rs (3,400 lines).

SAFETY PROPERTY HOLDING, verified: `languages/` has zero changes, v2 has NO
caller in the production path. The existing indexer still runs. Cutover has not
happened and is a separate decision.

Gate: fmt 0, clippy 0, **3,228 tests / 0 failed** (up 40).

## What the gate agent caught, before the walk was built on it

The fqn grammar had no namespace discriminator, so two different declarations
minted ONE key. Real collisions in this repo:
  - `WatcherHealth` has a `healthy` field (root_watcher.rs:79) AND a `healthy()`
    getter (:109)
  - sensei-bootstrap has `pub mod config;` and `pub fn config()` at crate root
Both legal — rust separates field/type/value namespaces. Merging them makes a
WRONG edge, which R4 ranks worse than a missing one. 24 field/method collisions
measured in this repo alone.

Fixed with `Ns { Ty, Val, Field, Macro }` on every form. I mutation-verified it:
collapsing all four labels to one string makes the test FAIL, so the guard is
load-bearing, not decorative. The spec's §2 sketch had the same gap and is
amended.

## The verification that matters

`the_reference_count_equals_an_independent_count_of_use_sites` — a counter with
no knowledge of the resolver, run over this repo's own rust, asserting >10,000
use sites and naming every file that disagrees. That is what makes "no reference
is dropped" a measurement instead of a claim.

## NOT done — steps 5-7

Six agents died on a session limit. Resolution ladder, relations and persistence
were NOT STARTED — not half-built. Nothing to clean up.

## Next

Steps 5-7: `indexer/resolve.rs` (shared ladder + reason codes), relations from
the same walk, then the persistence round-trip. Then step 8 differential
harness, step 9 cutover, step 10 retire. Rust only until §6 acceptance passes;
js/ts/svelte follow one at a time (D4).
