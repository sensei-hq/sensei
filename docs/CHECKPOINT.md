# Checkpoint

**Slice** — indexer-v2, driving the per-kind `lost` column to zero. Method.

**Done**

- `59002bf9` a use site reaches the member a trait impl supplies. New
  `SuppliedMembers` table + `Rung::SuppliedByATraitImpl`. 395 trait-supplied
  methods existed and zero were linked; the use side could not mint the trait
  segment. rust Method lost 797 → 724.
- `04289f5f` a binding the source never typed is typed by the call that bound
  it. `Observation::BoundToTheResultOf` + `what_that_call_returns` +
  `considered_here`. `let s = pg_store().await` x306 in one file was the
  dominant shape. rust Method lost 724 → 591, Field 1480 → 1433.

**Remaining** — rust Method 591, ts Method 75, rust Field 1433, ts Field 2467.
Every remaining lost Method is ONE cause: a receiver nothing types. The three
sub-shapes, largest first, are in the report and each needs a decision:
cross-type field tables (`state.pg`), the `Arc<T>` / `Result<T>` unwrapping rule
in `simple_type_name`, and the same binding-callee fact for TypeScript.

**Next command**

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::every_source_node_is_reached_by_a_test_and_then_by_other_source

**Open questions** — the metric itself is still a by-NAME upper bound within a
language; `TraceRecorder::is_empty` is "named" by 906 sites spelling `is_empty`.
Narrowing further needs the receiver's type, which is the missing thing.

**Known broken** — none here. `indexer::lib_indexer::tests::
real_dbd_single_file_has_deploy_component` (`--ignored`) reads the hardcoded
path `/Users/Jerry/Developer/dbd-rs`, which is not checked out on this machine.
