# Checkpoint

**Slice** — indexer-v2. Final account: the per-kind table, every non-zero
`lost` named as a defect, every `not called` given a shape.

**Done** — the account is complete and nothing was fixed to flatter it. The one
fix was not in the indexer: the suite came back 1 failed, so the gate was red.
`get_library_docs` mapped a `DocChoice` back to a held version row by ROUTE
alone, and two versions of one library share a route whenever their
`source_type` agrees — so `.find()` took whichever row the unordered versions
query returned first and labelled it EXACT. A folder pinned at 1.2.0 got 3.0.0's
pages with no caveat: a confident answer about an API it does not have (R4).
Now `DocChoice::is_served_by`, asking the same `same_version` that called the
fit exact; `DocRoute::of_source_type` replaces two copies of that match.
Commit `e5765339`, three tests, three mutations, each red alone.

**Remaining — DEFECTS, not residue.** rust Field 1441, rust Method 603, ts Field
2118, ts Method 59, ts Property 3 — every one an untyped receiver.
`config().daemon_url()` (bootstrap/src/lib.rs:47) misses because the call chains
off a call, not a binding; `cfg.db_url` (api/server.rs:110) because `binding_of`
reads the HEAD of `sensei_bootstrap::SenseiConfig::from_env`, a crate name. The
ts rows are a by-name upper bound: 952 unresolved `.id` reads blame all 126
field declarations spelled `id`.

**Next command**

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::every_source_node_is_reached_by_a_test_and_then_by_other_source

**Open questions** — (1) grade the phase on `lost` (a by-name upper bound that
cannot reach zero) or on SITE counts? Three slices have raised it. (2) should a
bare identifier READ be a reference? Decides ts Const 2707 and rust Const 436;
both walks chose "no". (3) `Arc<T>` unwrapping in `simple_type_name`.

**Known broken** — `lib_indexer::tests::real_dbd_single_file_has_deploy_
component` (`--ignored`) reads a hardcoded `~/Developer/dbd-rs`; no such dir,
and it reads nothing here. The barrier counts its own 14 `#[ignore]` tests in
`acceptance.rs` as source — not a test path, no `#[cfg(test)]`, so
`test_boundary` returns `u32::MAX`. +14 rust Function `not called`, no `lost`.
