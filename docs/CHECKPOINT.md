# Checkpoint

**Slice** — indexer-v2. Stage 11 COMPLETE (§10 met, 97,460 resolved vs
≥96,304). **GOAL, decided 2026-09-22: retire v1 and make v2 the only indexer.**
Inventory, couplings and rejected alternatives: `docs/backlog.md`.

**HEAD** `112cd44a` + docs. Gate: fmt clean, clippy `--all-targets -D warnings`
0, senseid **3291/0**, indexer **456/0**, all ignored indexer barriers green but
the environmental `dbd-rs` one. Corpus 97,460 / 96,760 / 194,220. Ratchets: A7
rust **0**, dangling **1,153/235** (headroom 147), split-impl **30/5,530**.

## The work order — FLOW FIRST, RUST ONLY, then a language at a time

Revised 2026-09-22 on the user's call, and it is the better order: wiring the
flow first proves the whole path end-to-end on ONE language at the lowest risk,
and every adapter after that lands behind a working pipeline and can be
verified against real data the moment it exists. It is also what
`10-cutover.md` already prescribes — "switch **RUST ONLY**, re-index, run
acceptance".

**A. Connect the flow, both R14 modes, and wrap the adapter call in the
file-parser task.**

    FULL         scan root -> repos -> manifest -> files   <-- structure barrier
                                                -> enqueue ONE parse task per file
    INCREMENTAL  on change -> files -> reparse the changed set

`TaskKind::ProcessFile` runs v2: `index_file` → `persist::write` →
`reconcile`. The task kind already exists and is v1's per-file unit;
`index_file` (stage 11's I8, "one file in, nodes and edges out") is R14's
parse-task unit. Stages 1-3 exist as `pipeline::scan_and_write_structure` and
stop AT the barrier by design ("writes NO nodes and NO edges, and enqueues no
parse task"); `incremental.rs` already classifies a change as OLD+NEW.

**B. Route RUST to v2 and leave every other language on v1.** Per-language
dispatch in the ProcessFile handler.

**C. Migrate one language at a time, and DROP ITS v1 PARSER AS IT GOES.** Four
already have v2 adapters and need only proving — typescript/javascript, svelte,
python, java. Five need porting: **SQL, Swift, Kotlin, Vue, C**.

**NO v1 FALLBACK. NOT ANYWHERE.** Decided 2026-09-22 and it overrides an
earlier note of mine that proposed one. The new flow asks v2's
`adapter_for_ext` and, for a language v2 does not claim, **the file is simply
not indexed** — an honest, visible gap. Two reasons, and the first is
decisive:

- **The two mint DIFFERENT identities.** v1 has `languages/fqn.rs`, v2 has
  `indexer/fqn.rs` with its own grammar. A fallback would put two identity
  schemes in ONE graph, so an edge minted by one could never meet a node
  minted by the other — the "two half-symbols" failure v2's fqn module exists
  to prevent, reintroduced at the seam.
- **The models do not compose.** v2 is single-file-independent with no
  cross-file table (the whole of stage 11); v1 is multi-pass with shared
  state. A row's meaning would depend on which engine produced it and nothing
  downstream could tell.

So coverage during the transition is a SCHEDULING question — which language
migrates next — never a dispatch question.

**D. When the last language has migrated**: re-index, run acceptance, delete
the v1 PARSERS and break the two couplings (`is_test_path` ×2,
`fqn::is_external` ×1).

**Note the split:** v1's `languages/` is not only parsers. `language_for_path`,
`is_test_path` and `import_target` serve non-indexing callers
(`classifiers.rs`, `graph_facts.rs`, `api/handlers/codebase.rs`,
`db/pg_store/graph.rs`, `tasks/processors/code.rs`). Language DETECTION is a
different concern from parsing; only the parsers are being retired here, and
what remains needs its own home rather than being deleted with them.

**The stage-10 differential harness is WAIVED** by the user — "I don't need to
compare v1 vs v2". It was the spec's cutover gate; the cost and what stands in
for it are in `docs/backlog.md`. Consequence: acceptance after re-index is now
the only empirical check, so it is not optional.

**The stage-10 differential harness is WAIVED** by the user — "I don't need to
compare v1 vs v2". It was the spec's cutover gate; the cost and what stands in
for it are recorded in `docs/backlog.md`. Consequence: acceptance after
re-index is now the only empirical check, so it is not optional.

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
