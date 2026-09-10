# Stage 10 — cutover: differential harness, switch Rust only, retire v1

Whole-system spec: `docs/design/indexer-v2.md` §7 (build order), §6 (A1–A9),
R10.5 (the three deletion triggers), D4. Depends on stage 9.

## 1. Purpose

Prove v2 is better than v1 on the real corpus, switch RUST ONLY, re-index,
run acceptance, and retire the superseded module. The existing indexer keeps
running until the switch, so the graph never goes stale.

## 2. Inputs and outputs

    differential(v1: &Facts, v2: &Facts) -> DiffReport   PURE

A TOOL that prints a report, not a test. Its output is read by a person who
decides whether to cut over.

## 3. Requirements

- **S1.** Run v1 and v2 over the same corpus and diff the facts. Every
  difference is classified: IMPROVEMENT, REGRESSION, or EXPLAINED. A regression
  blocks cutover.
- **S2.** v2's RESOLVED set must be a superset of v1's, or each exception
  justified in writing. Compare resolved edges and node identity — **not raw
  reference counts**, because v1 drops references silently and v2 will show far
  more. That is the intended result, not a regression.
- **S3** (D4). Switch RUST ONLY. js, ts, svelte follow one at a time, and the
  trigger to move on is the current language passing §6, not elapsed time.
- **S4** (R10.5). Re-point all THREE deletion triggers at `reconcile` in the
  SAME change, or the graph gets two removal semantics:
  - `tasks/handlers/process.rs::process_git_folder`'s `plan.removed` loop
  - `tasks/handlers/scan.rs::prune_vanished`
  - the `delete_file` / `delete_folder` handlers (the fs-watcher path)

  All three call `delete_nodes_by_file` today, whose `file_path` predicate
  deletes child module nodes and cascades their edges (stage 7 S8).
- **S4b** (D13). **WIPE THE CODE GRAPH, THEN RE-INDEX. Do not migrate it.**
  `TRUNCATE sensei.nodes, sensei.edges CASCADE`, then a full v2 run.

  The identity grammar changed, so this is not a preference: **194,355 of
  194,376 fqns carry no reach segment**, and `reach` plus the `<module>`
  segment both come from re-parsing. A v2 index over the old rows never
  matches the merge contract and builds a SECOND disjoint graph beside the
  first.

  | destroyed and rebuilt | |
  |---|---:|
  | nodes | 386,884 |
  | edges | 810,216 |
  | symbol_names | 134,866 |
  | embeddings to regenerate | **326,716** |
  | `inference.drift_items` | 1,727 — `doc_node_id` CASCADEs, `code_node_id` nulls |

  **Do NOT touch these — they are not parsed from source:** repositories
  (68), folders (9,378), files (48,665), libraries (1,121), library content
  (146), commands (572). "Indexed data" is `nodes` + `edges` + what derives
  from them, not the whole `sensei` schema.

  ORDER: wipe BEFORE dropping `lib_symbol`/`lib_package` from `node_kind`
  (D12) — an enum value cannot be dropped while rows use it, and after the
  wipe none do.

  **Re-embedding 326,716 nodes is the real cost of this stage.** Verify the
  embedding backfill sustains that volume BEFORE cutover depends on it, not
  during. Record the throughput.
- **S5.** Run A1–A9 against the LIVE graph after a full re-index.
- **S6.** Record the BEFORE numbers before deploying. They cannot be recovered
  afterwards.
- **S7.** Retire: move the superseded Rust module to `to_be_discarded/`, only
  after S5 passes.

## 4. Failure modes

| input | this stage does |
|---|---|
| a regression in the diff | STOP. Do not cut over and do not explain it away. A regression is the one output this harness exists to produce. |
| an acceptance threshold missed | STOP, and name which. A1–A9 are gates, not aspirations. |
| the re-index does not pick up the change | `sensei scan` will not notice an indexer change — force it by resetting `daemon.last_version` and restarting, then set it back. |
| the re-index exceeds a shell timeout | a full run is ~20 minutes plus embedding backfill and a command may cap at 10; poll across several rather than assuming failure. |
| A7 reports collisions on the live corpus | STOP. Colliding declarations mean one silently overwrites the other and which one wins is scan-order dependent. |
| the diff shows far more references in v2 | EXPECTED (S2). Compare resolved edges, not raw counts. |

## 5. Verification

| check | mutation that must break it |
|---|---|
| the harness classifies every difference; none is unclassified | let an unknown difference default to "explained" |
| a deliberately introduced regression in v2 is REPORTED as a regression | make the comparison count raw references instead of resolved edges |
| all three deletion triggers call `reconcile`; `delete_nodes_by_file` has no caller | leave one trigger unchanged — verified with `rg --no-ignore -g '!target'`, and the result count confirmed not truncated |
| only Rust is switched; js/ts/svelte still use v1 | switch the dispatch unconditionally |
| A1–A9 each run against the live graph and each can FAIL | assert `true` |
| the before-numbers line precedes the deploy in the progress file | write it after |

The unresolved-caller check is the one to do properly. "This has no callers" is
exactly the claim that needs `rg --no-ignore -g '!target'` and a confirmed
non-truncated count, not a quick grep.

## 6. Watch out

**Raw reference counts will look alarming and are the intended result.** v1
drops references silently — that is the defect. v2 emits an `Unresolved` for
every one of them. Comparing totals will show a large increase that is a fix,
not a regression. S2 exists because this is the easiest possible
misinterpretation of the harness's own output.

**Two removal semantics is the worst intermediate state available.** If only
some triggers are re-pointed, the graph deletes nodes one way through
reconcile and another way through `delete_nodes_by_file`, and which one runs
depends on how the file happened to be removed. S4 is all-three-or-none.

**Do not cut over more than one language.** Rust first (D4), and the gate to
move on is §6 passing, not the calendar. Mixing languages is how the first
attempt produced "three scans and a follow-up reconcile".

**Retirement is last.** The superseded module keeps running until S5 passes, so
there is never a window where the graph has no indexer.

## 7. Definition of done

- The diff report has ZERO unexplained regressions and v2's resolved set is a
  proven superset.
- All three deletion triggers go through reconcile;
  `delete_nodes_by_file` has no remaining caller, verified properly.
- A1–A9 pass against the live graph after a full re-index.
- Field and enum-variant nodes are non-zero for Rust.
- Before-numbers recorded prior to deploy.
- The superseded Rust module is in `to_be_discarded/`, and js/ts/svelte are
  untouched and still on v1.
