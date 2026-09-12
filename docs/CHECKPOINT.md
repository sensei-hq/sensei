# Checkpoint — indexer v2

**State: STAGES 0–9 LANDED. Stage 10's GATE IS BUILT AND RUN, AND IT BLOCKS:
777 regressions over 380 rust files. Do not cut over until they are dispositioned
— that decision is the next thing needed and it is a judgement call, not code.
Suite GREEN: 3058 passing, 0 failing, fmt clean.**

Read in order: `docs/plans/indexer-v2-sequence.md` (master plan), then
`docs/spec/indexer/10-cutover.md`, then `docs/design/indexer-v2.md`.
`docs/plans/indexer-v2-rust.md` is SUPERSEDED (marked at its top).

## Slice

Retire v1, implement v2. Pre-release: `dbd reconcile`, never hand-written
migrations. DDL is applied to `sensei` and `sensei_test`.

## Done / remaining

| | |
|---|---|
| 0–3 structure, 2b libraries | `git log --grep=indexer-v2` |
| **4–7 — the graph reads through `file_id`** | `6b5fe44f` |
| **8 — the v2 stages reach the progress stream** | `c13ae29c` |
| **9 — incremental, pure half** | `68c88f8f` |
| **10 S1/S2 — the differential gate, run** | `5aa79e60` |
| 10 S3–S7 — switch rust, re-index, retire v1 | BLOCKED on the gate |

## Next command

    # re-run the gate (~15s after a warm build):
    cargo test -p senseid --bin senseid -- --ignored --nocapture --exact \
      "indexer::differential::corpus::v1_and_v2_over_this_repos_rust"

## THE GATE BLOCKS — the open decision

    IMPROVEMENT 7,805 | REGRESSION 777 | EXPLAINED 3,861 | UNCLASSIFIED 0
    v1 resolved 7,899 -> v2 resolved 11,066
    v2 also emits 72,063 UNRESOLVED-with-reason that v1 dropped silently.

Of the 777, measured (not assumed — the module-path hypothesis was tested and
explains only 42):

    486  ReceiverTypeUnknown   v2 saw the call, could not name the receiver
    146  NoImportInScope       v2 saw the name, no import placed it
      1  Denylisted
    144  the name appears at NO v2 use site   <- the genuinely open class

For the 633 v2 SAW and declined, the question is whether v1's edge was CORRECT.
R4 says a wrong edge is worse than a missing one, so if v1 was guessing these
are improvements in disguise; if v1 was right, v2 has lost reach. The harness
deliberately does not decide that. Sampling a dozen by hand answers it.

The 144 v2 never saw are a different question: a use-site form its walk does not
emit.

## Known-broken / known-wrong — do not build on

- **`delete_folder` issues a path-prefix `DELETE`** (`process.rs`), which 09 S7
  forbids. The fix is N file RECONCILES; blocked on stage 10 wiring v2's
  reconcile. This is stage 9's one unmet DoD item.
- **S4b's wipe is MOOT and the spec's premise is stale.** The code graph is
  already empty — nodes 0, edges 0, embeddings 0, all 510 folders `discovered`
  — because stage 0 wiped it and nothing has re-indexed since. So "the existing
  indexer keeps running until the switch, so the graph never goes stale" (10 §1)
  is false, and the 326,716 embeddings S4b budgets were spent at stage 0.
  BEFORE-numbers (S6), recorded: repositories 364, folders 510, files 56,907,
  libraries 1,222, library_content 371, commands 0.
- **The watcher has no manifest/lockfile branch.** A changed `Cargo.lock` is
  enqueued as a plain `ProcessFile`, parses as nothing, and the graph keeps
  yesterday's dependency set until a full re-scan runs (09 S9, live today).
  `incremental::retrigger_for` fixes it. OPEN QUESTION: `manifest_dirs` is
  derivable from `folders` (D11), but LOCKFILE PATHS ARE PERSISTED NOWHERE —
  either stage 2 stores them or the watcher probes per event.
- **`folder_kind.standalone`** is dead in v2's model; v1 scan paths still write
  it (`scan.rs` x4, `project_detail.rs`). Goes at stage 10.
- **`demote_v2_symbol`** keeps a node and nulls its file, so `target_id` stays
  non-null and consumers read it as resolved (67,839 edges). Fixed by
  `07-reconcile.md` S3-S6.
- **`v2_edges_contributed_by`** narrows with `AND (s.fqn = ANY($3) OR
  s.resolved = false)`, so an edge whose source this file deleted, resolving
  elsewhere, is never revisited. Fixed by S7.
- **`library_content.package_name`** has no writer: nothing states which package
  a page documents, and inferring it from the component name is the R4 guess.
- **`sensei_test` accumulates fixture rows.** Nothing cleans `_test:` metrics or
  `test/` repositories, and `metric_status` cross-joins them — it reached 88M
  rows and a summary query failed on disk. `DELETE FROM sensei.metrics WHERE key
  LIKE '\_test:%'` plus the same for `repositories.repo_key LIKE 'test/%'`.
- **Clippy: 88 findings on `develop`**, all `never used` in v2 modules with no
  caller yet. Stage 10 is where that count should go to ZERO — it is now the
  tenth capability built ahead of its caller, and the pattern is the design's
  most repeated defect.

## Open questions

Only the lockfile-persistence one above. Everything else is resolved in git
history.
