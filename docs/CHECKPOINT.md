# Checkpoint

**Slice** — indexer-v2 **stage 11** (`docs/spec/indexer/11-file-index.md`), rust
only (§11). The walk is TABLE-FREE, `index_file` exists, rust identity
collisions are ZERO, and **§10's done-gate is MET: 97,261 resolved against
≥96,304, a margin of 957.**

**HEAD** `a57dd050`. Gate: fmt clean, clippy `--all-targets -D warnings` 0,
senseid **3287/0**, indexer **452/0**, every ignored indexer barrier green, A7
rust **0**. Corpus: 97,261 resolved / 96,719 unresolved / 193,980 total.

## Done

S8 · impl anchoring · table deleted · S7 Named/Candidate · S8 narrowed · I7 ·
I8+I9 · I6a · I6b · I6c · method-body locals · cfg variants · anonymous
`const _` · `a57dd050` a `super`-rooted import names a home (+2,334 refs;
split-impl anchoring 655/5,474 → 30/5,519; dangling 1,260/358 → 1,151/235).

    baseline 96,386  table out 96,336  S7 97,049  I6b 93,801  I6c 94,851
    const _  94,927  super-root 97,261 ← gate met

## Remaining

1. **I11** conservation · **I12** measure + record.
2. **`Form::MemberVariant`** — a `cfg`-gated MEMBER cannot carry its condition.
   Design + blast radius in `docs/backlog.md`. Costs a missing fact, not a wrong
   edge (all seven are single-arm).
3. **Two spec corrections, no code**: §5/§8 draw `TraitImpl` trait→type while
   the code and the shipped `implements` column run type→trait; §4.2's `via`
   list omits `NamedByThisFile`, which §5's own worked output uses.

## Open questions

None blocking. **The §7-vs-§10 contradiction DISSOLVED** — the gate was met
without §7's deferred wildcard export list, so both clauses hold. Why +2,334
exceeded the 999 predicted is NOT measured (likely a `BoundToTheResultOf`
cascade). Root causes and the full post-mortem are in `docs/backlog.md`.

## Next commands

    cargo test -p senseid --bin senseid -- indexer::
    cargo test -p senseid --bin senseid -- --ignored --nocapture index::corpus
    cargo test -p senseid --bin senseid -- --ignored --nocapture barrier_necessity

## Known-broken

None in the indexer. 4 ignored tests fail environmentally (`dbd-rs` sibling repo
not checked out, gateway config, 2 installer hooks). `prune_empty_projects_*` is
NON-DETERMINISTIC in a full run — green in isolation and on re-run (3287/0), the
fourth instance of the shared-test-DB sweep hazard in `docs/backlog.md`.
