# Checkpoint

**Slice** — indexer-v2, driving the per-kind `lost` column to zero. Sweep
round 2: the largest row was ts Field 2246, and the cause was a discarded
array element type.

**Done**

- `f600d7e5` an array's element type, and the callbacks it is handed to.
  `type_segment` refuses `T[]` (correctly — `.length` is Array's member, not
  `T`'s), and the OTHER reading the annotation carries went with it. `Flow`
  grows a third table for the element type, beside the binding's own type and
  never merged with it; every `Array` iteration method hands its callback an
  element at the position `Array`'s signature puts it — `(element, index,
  array)` for `map` and kin, two for `sort`, the second for `reduce`. A
  receiver with no recorded element type hands over nothing, which keeps a
  `Bag` with its own `map` out of it.
  ts Field lost 2246→2121, calls 670→797; ts TOTAL 2308→2183. Dangling
  unchanged at EVERY rung of both languages while ts landed rose 191.
- Earlier: `d7f8d009` binding typed by its call, `5c06f35f` dynamic import,
  `b967c09b` one boundary set. ts Const/Static/Function tail is 0.

**Remaining** — ts Field 2121, rust Field 1433, rust Method 593, ts Method 59,
ts Property 3. Refuted by probe and not to be re-tried: the owning type is
already linked for all 2246 (540 distinct types, head 23, so no special case);
an app-boundary narrowing of the by-name match reaches only 35.

**Next command**

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::every_source_node_is_reached_by_a_test_and_then_by_other_source

**Next cause, already sized** — a svelte `{#each xs as x}` binding is untyped
exactly as a callback parameter was, and names 1,491 unresolved field reads
(the largest remaining bucket). It lives in `lang/svelte.rs`, not the JS walk.
After that: a chained `xs.filter(..).map(..)` keeps its element type through
the element-preserving methods, and 2,798 sites have a lambda-parameter
receiver the walk still cannot reach.

**Open questions** — (1) `type_segment` reduces `Promise<Project>` to
`Promise`, so an async TS callee types nothing. (2) should a bare identifier
READ be a reference? Decides ts Const 2707 / rust Const 436. (3) the metric is
still a by-name upper bound within a language for the MEMBER kinds: 138 `id`
declarations are all blamed by one pool of 975 unresolved `.id` reads, and
only 699 distinct names carry the whole 2246.

**Known broken** — none here. `indexer::lib_indexer::tests::
real_dbd_single_file_has_deploy_component` (`--ignored`) reads the hardcoded
path `/Users/Jerry/Developer/dbd-rs`, which is not checked out on this machine;
verified failing identically on the pre-change tree.
