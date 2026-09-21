# Checkpoint

**Slice** — indexer-v2 **stage 11** (`docs/spec/indexer/11-file-index.md`), rust
only (§11). **COMPLETE.** The walk is TABLE-FREE, `index_file` exists, rust
identity collisions are ZERO, every §10 done-item cites the test that settles
it, and the gate is met: **97,317 resolved against ≥96,304**, 1,013 to spare.

**HEAD** `e34d7e5a`. Gate: fmt clean, clippy `--all-targets -D warnings` 0,
senseid **3289/0**, indexer **454/0**, every ignored indexer barrier green.
Corpus: 97,317 resolved / 96,755 unresolved / 194,072 total.
Ratchets: A7 rust **0**, dangling **1,151/235** (ceiling 1,300, headroom 149),
split-impl anchoring **30/5,529** (0%).

## Done

S8 · impl anchoring · table deleted · S7 Named/Candidate · S8 narrowed · I7 ·
I8+I9 · I6a · I6b · I6c · method-body locals · cfg variants · anonymous
`const _` · `a57dd050` a `super`-rooted import names a home · `656af9d0` spec
corrections · `e34d7e5a` `Form::MemberVariant`.

    table out 96,336  S7 97,049  I6b 93,801  I6c 94,851  const _ 94,927
    super-root 97,261 ← gate met    MemberVariant 97,317

## Remaining

**Nothing open in stage 11.** Next is §11: the other adapters, ordered by
corpus size — typescript/javascript/svelte, then python, then java. Each is the
same three changes (drop the table parameter, read the type's home from the
file, grade `Named` vs `Candidate`) and is done when §10 passes for it. The
trait-level `LanguageAdapter::read` parameter goes when the LAST one lands.

Stage 12 (persistence) may now start; it is no longer gated.

## Open questions

Two, both recorded in `docs/backlog.md` with the check that settles them, and
both explicitly UNVERIFIED — not blocking:

- Why `a57dd050`'s +2,334 exceeded the 999 the decomposition predicted. Likely a
  `BoundToTheResultOf` cascade once a constructor resolves.
- A gated member of an `impl Trait for Type` needs a fifth tail segment and a
  form of its own. ZERO in this corpus; excluded on grammatical ground by both
  the walk and `count_gated`, independently.

Stage 12 still owes the S7 obligation: a target no first-party declaration
mints must resolve or be marked EXTERNAL, never left a ghost.

## Next commands

    cargo test -p senseid --bin senseid -- indexer::
    cargo test -p senseid --bin senseid -- --ignored --nocapture index::corpus
    cargo test -p senseid --bin senseid -- --ignored --nocapture barrier_necessity

## Known-broken

None in the indexer. 4 ignored tests fail environmentally (`dbd-rs` sibling repo
not checked out, gateway config, 2 installer hooks). `prune_empty_projects_*` is
NON-DETERMINISTIC in a full run — green in isolation and on re-run, the fourth
instance of the shared-test-DB sweep hazard in `docs/backlog.md`.
