# Checkpoint

**Slice:** unified-walk architecture + rust transitive receiver resolution. Branch `develop`.

## The plan we agreed

`code.rs` parses every file **THREE** times — `adapter.parse`, `adapter.parse_to_ir`,
`adapter.fqn_output` — and no pass sees the others' work. Collapse 3 → 1 so nothing
a later stage needs is dropped. Doing it big-bang across 8 adapters would be reckless,
so it goes in increments, each independently useful and gated.

Rust's payoff from it, measured: **all 28,069 unresolved rust calls are lowercase**
(method calls, which no import can name), while `imports` is already 3,718 resolved /
**0 unresolved**. Import-side resolution is DONE; the receiver side is the whole gap.

The transitive chain, of which only one hop was missing:

    ctx.pg().method()
      ctx             -> TaskContext                                   bindings has it
      TaskContext·pg  -> rust·senseid·tasks::executor·TaskContext·pg    constructible today
      its return type -> &crate::db::pg_store::PgStore                  <- WAS MISSING
      PgStore·method  -> rust·senseid·db::pg_store·PgStore·method       constructible today

## Done — all gated: fmt 0, clippy 0, 3,156 tests / 0 failed

- `b466c3e5` `ReindexPlan.expected` + `set/get_folder_expected_files` — the
  completeness denominator, recorded by the walk (the only thing that knows it).
- `09d41f6d` `sensei.folder_completeness` view — "fully indexed" derived from FILES,
  recursing only over `parent_id`, never reading `folders.status`. That is what makes
  ONE update settle the hierarchy instead of a fixpoint loop. Has a `drifted` column.
- `5aeb5caf` `FqnDefinition.return_type` — carried by the pass that MINTS the node.
  Rust reuses the IR walk's `extract_return_type`; every other adapter records an
  honest `None`. Kept verbatim (normalising here would discard the module path).

## Next, in order

1. **Persist `return_type`** — it is now carried to the emit path but NOT written to
   the DB. Put it in node `props` (no DDL change; `expected_files` set the precedent).
   Without this the chain still cannot be walked at query time.
2. **Structured unresolved hint** — when `resolve_call` fails on a receiver, emit what
   it SAW (receiver expression, use-path in scope) instead of a bare `target_name`.
3. **Link phase** resolving those hints by fqn lookup, fired when
   `folder_completeness.subtree_complete` first turns true. This is the piece
   `folder_completeness` exists to unblock — DetectCommunities is NOT a safe hook
   (`analyzer_scheduler.rs:252` enqueues it unblocked).
4. Continue the 3→1 parse collapse for the remaining adapters.

## Notes / refuted

- Keying fqns on the imported FILE was considered and rejected for rust: `use` does not
  name a file (`mod.rs` vs `x.rs`, inline mods, `pub use` re-exports), and it cannot
  touch method calls anyway. It IS a good fit for TS/JS/Python/Java.
- Order-independent merge already exists (`OnMiss::CreateStub` + `upsert_node_by_fqn`);
  there is no reconcile pass to remove.
- Search threshold 0.45 is measured; tightening to 0.40 was refuted live. Do not retry.
- Open: #151 (receiver resolution), #152 (rust import resolver ignores `local_modules`,
  5 survivors). 1 folder failed the last reindex: `cluster:scheduler`, undiagnosed.
