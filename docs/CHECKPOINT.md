# Checkpoint

**Slice** — indexer-v2. **Stage 11 is COMPLETE** (§10 met, 97,460 resolved
against ≥96,304). Now in **§11 — the other adapters**, TypeScript first.

**HEAD** `112cd44a`. Gate: fmt clean, clippy `--all-targets -D warnings` 0,
senseid **3291/0**, indexer **456/0**, every ignored indexer barrier green but
the known environmental `dbd-rs` one.
Corpus: 97,460 resolved / 96,760 unresolved / 194,220 total.
Ratchets: A7 rust **0**, dangling **1,153/235** (ceiling 1,300, headroom 147),
split-impl **30/5,530**, A8 typescript back to **16** distinct targets.

## Done since stage 11 closed

`b17731fc` the barrier decomposition splits by LANGUAGE — the instrument §11 is
sequenced on · `9ad53f43` one place reads a rooted path against a module
(`resolve::rooted_against`; I had duplicated the ladder's arithmetic in the rust
walk and was about to write a third copy) · `112cd44a` TypeScript's relative
import names a home (+109 resolved, zero new ghosts).

## §11 state, per adapter

| adapter | table dropped? | import rung | grade |
|---|---|---|---|
| rust | YES (S5) | yes | `Named` |
| typescript / javascript / svelte | **no** | **yes, relative only** | `Candidate` |
| python | no | no | — |
| java | no | no | — |

**TypeScript is NOT "the same three changes" and the corpus says so 1:1.** The
third change — grading a file-stated home `Named` — buys +378 resolved and
+378 dangling, i.e. every extra reference it resolves names an identity nothing
declares. Full measurement, the three cross-file causes, and the `$lib` problem
are in `docs/backlog.md`.

## Next — and it needs a decision, with a recommendation

Dropping TypeScript's table needs the `$lib` alias map: **1,008 relative
specifiers against 891 `$lib` + 46 `$app`**, and `import_origin` files an alias
as EXTERNAL because the walk is never handed the bundler config. Recommended:
the SCAN reads `svelte.config.js` / `tsconfig` paths once and hands the map to
the adapter as a fact on `Source`, the way an adapter is already TOLD its
package and module and never climbs a directory (keeps R7 intact). Alternatives
and their costs are in the backlog entry.

**CORRECTION — python and java are NOT the easy forward work.** I wrote that
before checking: **this repository contains ZERO `.py` and ZERO `.java` files**,
so neither appears in the per-language decomposition and neither can have §10's
gate ("resolved-reference count over this repo") evaluated at all. The java
module says so in as many words and calls it "the right constraint rather than a
limitation".

Both are measurable only against an EXTERNAL corpus via `SENSEI_CORPUS`, which
already exists as a mechanism:

    SENSEI_CORPUS=~/Work/Dayamed cargo test -p senseid --bin senseid -- \
      --ignored --nocapture indexer::lang::java::corpus

That corpus is present on this machine — 8,007 `.java` files against 56 `.py`
— so **java is measurable and python effectively is not**. So the §11 order
("typescript/javascript/svelte, python, java") is by corpus size in the abstract
and NOT by what can be verified here; java should come before python on
evidence, and python needs a corpus chosen before its rung is worth writing.

## Open questions

- The `$lib` alias map above — the one blocking decision.
- Why `a57dd050`'s +2,334 exceeded the 999 predicted. Likely a
  `BoundToTheResultOf` cascade; UNVERIFIED, check recorded in the backlog.
- A gated member of an `impl Trait for Type` needs a fifth tail segment. ZERO
  in this corpus, excluded on grammatical ground by the walk and `count_gated`.

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
