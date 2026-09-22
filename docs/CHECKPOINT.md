# Checkpoint

**Slice** — indexer-v2. Stage 11 COMPLETE (§10 met, 97,460 resolved vs
≥96,304). **GOAL, decided 2026-09-22: retire v1 and make v2 the only indexer.**
Inventory, couplings and rejected alternatives: `docs/backlog.md`.

**HEAD** `112cd44a` + docs. Gate: fmt clean, clippy `--all-targets -D warnings`
0, senseid **3291/0**, indexer **456/0**, all ignored indexer barriers green but
the environmental `dbd-rs` one. Corpus 97,460 / 96,760 / 194,220. Ratchets: A7
rust **0**, dangling **1,153/235** (headroom 147), split-impl **30/5,530**.

## The sequence — nothing may be deleted before step 4

1. **Wire v2's persistence** ← NEXT. Pieces all exist (`index_file`,
   `persist::write`, `reconcile`); missing is the IO half joining stage 3 to
   4-6. R14: structure barrier, then ONE parse task per file, each running
   `index_file` → `persist::write` → `reconcile`. `index_file` (stage 11 I8) IS
   that unit. Mirror `tasks/handlers/process.rs`, which does this for v1.
2. Port the five adapters v1 has and v2 lacks: **SQL, Swift, Kotlin, Vue, C**
   (decided: all five before deleting v1, not a per-language fallback).
3. Stage 10's differential harness — does not exist. S1/S2: v2's resolved set
   must be a SUPERSET of v1's, regressions blocking.
4. Cut over, re-index, run acceptance. 5. Delete `languages/` and break its
   two couplings (`is_test_path` ×2, `fqn::is_external` ×1).

**v1 is still production** — nothing outside `indexer/` calls v2's core.
"Stage 12" in stage 11's prose is a forward reference to an unwritten spec;
persistence is stage 6 and is built.

## §11 adapter state (paused at step 1's request)

| adapter | table dropped | import rung | grade |
|---|---|---|---|
| rust | YES (S5) | yes | `Named` |
| typescript/javascript/svelte | no | relative only | `Candidate` |
| python, java | no | no | — |

TypeScript's `Named` grade is 1:1 against ghosts (+378 resolved, +378
dangling), so it stays weak until the cross-file lookup exists. `$lib` is
DECIDED: the manifest scan detects aliases and passes them to the file scan;
the conventional `$lib`→`lib` needs no config read (v1 measured it).

## Open questions

- Why `a57dd050`'s +2,334 exceeded the 999 predicted (likely a
  `BoundToTheResultOf` cascade). UNVERIFIED; check in the backlog.
- `import_target.rs` is v1-only and holds the alias measurements; v2 has a
  second import classifier. Reconcile before writing a third.

## Next commands

    cargo test -p senseid --bin senseid -- indexer::
    cargo test -p senseid --bin senseid -- --ignored --nocapture index::corpus

## Known-broken

None in the indexer. 4 ignored tests fail environmentally (`dbd-rs` not checked
out, gateway config, 2 installer hooks). `prune_empty_projects_*` is
NON-DETERMINISTIC in a full run — green in isolation; the shared-test-DB sweep
hazard in `docs/backlog.md`.
