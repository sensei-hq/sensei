# Checkpoint

**Slice** — indexer-v2, driving the per-kind `lost` column to zero. The
TypeScript tail: Const, Static, Function. All three are at ZERO.

**Done**

- `498b0f42` a name reached through a receiver is no evidence about a binding.
  Third narrowing of the bare-name match, after reach and language. `xs.slice()`
  asks a type for a member; a const, a `let` and a free function are members of
  nothing. MEASURED first: 100% of the carriers for all three kinds were
  `StaticMemberExpression`, in every language — `arr.map` 542, `arr.filter` 288,
  `Date.now` 47. ts Const 145→0, Static 22→0, Function 146→5, rust Function 8→0.
  No edge moved: `calls` and `not called` byte-identical.
- `5c06f35f` a dynamic import is an import. `const { go } = await import('./m')`
  states its dependency in a literal, so it is recorded as one; bound to one
  name it is a namespace. All five `exact` losses — the strongest evidence the
  barrier has — were this shape. ts Function lost 5→0, test calls 537→598.
- `b967c09b` one boundary set (`resolve::member_names_of`). It was built four
  ways; two omitted `Property`, so the barrier fixtures and the Java harness
  resolved against a narrower boundary than the measurement beside them.

**Remaining** — ts Field 2467, rust Field 1433, rust Method 592, ts Method 75,
ts Property 3. Const/Static `lost` is zero but `not called` is 2707/320: nothing
names them at all, because neither walk emits a reference for a bare identifier
read. That is a deliberate spec decision (javascript.rs:1965, rust/walk.rs:922),
not a resolver gap, and overturning it is the open question below.

**Next command**

    cargo test -p senseid --bin senseid -- --ignored --nocapture \
      indexer::acceptance::every_source_node_is_reached_by_a_test_and_then_by_other_source

**Open questions** — (1) should a bare identifier READ be a reference? Decides
whether ts Const 2707 / rust Const 436 are defects or definitions. (2) `$lib`
resolves to a fabricated library node for 1,456 references; fixing it needs the
app/dojo/website package split first. (3) the metric is still a by-name upper
bound within a language for the MEMBER kinds.

**Known broken** — none here. `indexer::lib_indexer::tests::
real_dbd_single_file_has_deploy_component` (`--ignored`) reads the hardcoded
path `/Users/Jerry/Developer/dbd-rs`, which is not checked out on this machine.
