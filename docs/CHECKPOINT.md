# Checkpoint

**Slice** — indexer-v2, driving the per-kind `lost` column to zero. Sweep
round 1: the largest row was ts Field 2467, and it is a receiver-typing gap.

**Done**

- `d7f8d009` a TypeScript binding is typed by the call that bound it. The
  ladder rung has been language-neutral since the Rust twin; only the Rust WALK
  ever recorded `BoundToTheResultOf`, so in TS the rung got a bare `m` and fell
  through. `Flow` grows a callee table beside the type one — clears on rebind,
  intersects at a join, travels into a closure. Only `await f()`, `f()!` and
  `(f())` are peeled; a method hop is not, because the plumbing list holds
  `map`/`filter`/`find` and would type an array as its element (R4).
  ts Field lost 2467→2246, ts Method 75→59, ts TOTAL 2545→2308; rust TOTAL
  2025→2025. `ts declared_by_its_type` landed 1327→1794, DANGLING still 0.
- Earlier: `498b0f42` receiver narrowing, `5c06f35f` dynamic import,
  `b967c09b` one boundary set. The TypeScript Const/Static/Function tail is 0.

**Remaining** — ts Field 2246, rust Field 1433, rust Method 592, ts Method 59,
ts Property 3. Two causes were REFUTED by probe this round and should not be
re-tried: all 2470 lost ts fields are owned by a declared type something links
(so the types are reachable), and a non-exported-declaration narrowing reaches
only 95 of them.

**Next command**

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::every_source_node_is_reached_by_a_test_and_then_by_other_source

**Open questions** — (1) `type_segment` reduces `Promise<Project>` to
`Promise`, so an async TS callee types nothing; worth ~52 more (type, member)
pairs, and needs the peeler to say whether an `await` was stripped. (2) should
a bare identifier READ be a reference? Decides ts Const 2707 / rust Const 436.
(3) the metric is still a by-name upper bound within a language for the MEMBER
kinds — 1036 of the 2470 share their name with 10+ other field declarations.

**Known broken** — none here. `indexer::lib_indexer::tests::
real_dbd_single_file_has_deploy_component` (`--ignored`) reads the hardcoded
path `/Users/Jerry/Developer/dbd-rs`, which is not checked out on this machine;
verified failing identically on the pre-change tree.
