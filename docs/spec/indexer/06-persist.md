# Stage 6 — persistence: lossless at the boundary

Whole-system spec: `docs/design/indexer-v2.md` R3, R4, R9, R13, D1, D9.
Depends on stage 5.

## 1. Purpose

Write `FileFacts` to the database without losing anything the walk captured.
The walk-to-database seam is where data was lost before — a declared type read
correctly from the AST and then dropped by a positional insert with no slot for
it — so D1 makes the persistence path part of the scope rather than an
afterthought.

## 2. Inputs and outputs

    write(store: &PgStore, folder_id, file_id, facts: &FileFacts) -> Result<Written, WriteError>   IO
    read_back(store, folder_id, file_id) -> FileFacts                                              IO

    struct Written { nodes_upserted, edges_merged, claims_written, collisions }

`read_back` exists FOR THE TESTS and is not an optimisation — it is what makes
"nothing was lost" checkable by round trip rather than by reading the code.

## 3. Requirements

- **S1** (R9). Rows are TYPED STRUCTS, destructured exhaustively, so adding a
  field is a COMPILE ERROR at every site that must learn about it. **No
  positional-argument inserts.** An 8-argument call with no slot for a new
  field drops it silently, and that is how the declared type was lost.
- **S2** (R3). Every field the walk captured reaches a column or a prop —
  declared types, return types, spans, visibility, docstrings, unresolved
  reasons and their evidence.
- **S3** (R2, R4). An `Unresolved` reference produces a ROW, not an absence.
  The edge carries `target_id IS NULL` with `target_name` set, which is a
  recorded gap; dropping it is a lie of omission the graph cannot recover from.
- **S4** (§2, the merge contract). A definition arriving AFTER a reference
  merges onto the existing node by fqn. It must not create a second node. This
  is the property that makes indexing order-independent.
- **S5** (D9). Every declaration writes its file's key into `props.claims`
  using `jsonb_set(props, '{claims}', coalesce(props->'claims','{}') || …)`.
  **NOT `props || …`**, which replaces the whole object.
- **S6** (R13). `file_id` is LOOKED UP and FAILS CLOSED if absent. No
  get-or-create — stage 3 S2 states why, and this is where the temptation
  actually appears.
- **S7.** Edge occurrences merge per file, never wholesale. Two files can
  produce one `(folder, source, target, kind)` row; `jsonb ||` on the whole
  props replaces another file's contribution.

## 4. Failure modes

| input | this stage does |
|---|---|
| `file_id` lookup misses | `Err`. Never create the row (S6). |
| two declarations mint one identity | record a COLLISION in `Written.collisions` and fail the write. An A7 violation is not a state to support gracefully — which of two colliding declarations wins is otherwise scan-order dependent. |
| a node kind the enum does not accept | `Err`, named. Stage 0 widened the enum; a rejection means the walk emits a kind nobody declared. Do not substitute a placeholder. |
| an edge whose target is unresolved | write it with `target_id NULL` and `target_name` set (S3). Normal, not an error. |
| a props merge conflict | the jsonb idioms in S5/S7 make it impossible by construction. If one appears, a whole-props write has been reintroduced. |
| the transaction fails midway | roll back the whole file. A half-written file is worse than an unwritten one, because reconcile will diff against it. |

**Never `.unwrap_or_default()` a fallible write and report success.** A read
handler that turns a failure into `[]` and a writer that turns one into "0 rows
written" are the same defect.

## 5. Checkpoint output

    {"stage":"06-persist","at":"<iso8601>","repo":"…",
     "files_written":372,"write_errors":0,
     "nodes_upserted":<n>,"nodes_merged_onto_existing":<n>,"nodes_created":<n>,
     "edges_merged":<n>,"edges_with_null_target":<n>,
     "claims_written":<n>,"collisions":[],
     "round_trip_verified":372,
     "sample_round_trip":{"fqn":"rust·senseid·…·Reach·Field·item",
                          "wrote":{"declared_type":null,"visibility":"pub","span":[89,3,89,8]},
                          "read":{"declared_type":null,"visibility":"pub","span":[89,3,89,8]},
                          "identical":true}}

`collisions` must be an empty array. If it is not, A7 has failed and the run
should stop rather than record which declaration happened to win.

## 6. Verification

| test | mutation that must break it |
|---|---|
| **round trip** — write a fixture file's facts, read back, assert EVERY field of every `Symbol` and `Reference` survived | drop any field from the insert |
| specifically: a DECLARED TYPE reaches the database and comes back | drop `declared_type` — this is the seam that lost data before |
| an `Unresolved` reference produces a row with `target_id NULL` and `target_name` set | skip unresolved references |
| a reference written FIRST, then its declaration, yields ONE node with both sets of facts | make the definition insert unconditional |
| two files declaring one fqn produce one node with TWO keys in `props.claims` | use `props \|\| …` instead of `jsonb_set` |
| two files contributing one edge produce one row with TWO occurrence keys | use `props \|\| …` on the edge |
| persisting with an absent `file_id` returns `Err` | add get-or-create |
| a collision is REPORTED and the write fails | let the last writer win |
| **the round trip reads COLUMNS, not the props the same writer wrote** | change a column's value after writing and watch the test still pass — if it does, the test is self-verifying and proves nothing |

That last row is a live defect, not a hypothetical: **11 unverified column
mappings exist in the committed code, 6 of them in `v2_symbol_unchanged`.** The
round trip reads back props the same writer wrote, so the column values verify
against themselves. Fix the CLASS — assert against the column — not the eleven
fields.

## 7. Watch out

**The 8-argument insert is the disease, not a style preference.** Positional
arguments have no slot for a field nobody added, so the compiler cannot help.
A typed row struct with exhaustive destructuring turns the same mistake into a
build failure. This pattern is worth auditing for across the whole codebase,
not just here.

**A symbol owning its own persistence is better than a free function taking
eight arguments.** If a `NodeRow` type carries its own upsert, there is one
place to change when a field appears and one place to test.

**`props || EXCLUDED.props` and `jsonb_set(props, '{k}', v)` are not
interchangeable.** The first replaces `claims`/`occurrences` wholesale. Nearly
every bug in this area is a variant of that one confusion.

**Do not add a garbage collector here.** A stub with no edges and no children
belongs to `prune_orphan_stubs_scoped`, which already runs every reconcile
tick. Two owners of deletion is how a race gets built.

## 8. Definition of done

- No positional-argument insert exists in the v2 persistence path.
- The round trip passes over a fixture AND asserts against columns, not props.
- The 11 self-verifying column mappings are fixed as a class.
- Claims and occurrences both merge per file, proven by two-file fixtures.
- `file_id` fails closed, proven by test.
- `Written.collisions` is empty over the whole corpus (A7).
