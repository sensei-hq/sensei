---
name: A path rooted in a sibling package has no rung
description: The ladder knows a sibling crate is ours and refuses to call it external, but has nowhere to place it
date: 2026-09-30
status: ready
---

# A path rooted in a sibling package has no rung

## The gap

After #207, `World::first_party` holds every package the project declares. The
ladder now *knows* `sensei_bootstrap` is ours. It still cannot place
`sensei_bootstrap::SenseiConfig::from_env`, because no rung handles a path whose
head names a **sibling** first-party package.

Walking `Ladder::place` for a fully-qualified path with **no `use` statement**:

| rung | outcome | why |
|---|---|---|
| `through_an_import` | Unbound | there is no `use`. A Cargo dependency puts the crate root in scope; nothing is written down |
| `rooted_in_this_package` | Unbound | `is_a_root` accepts only the root TOKENS — `crate`, `self`, `super` |
| `a_fully_qualified_external` | **declines** | `resolve.rs:1161` — `owned_by_this_scan(head).is_some()`, so it correctly refuses to mint a `lib·` fqn for a sibling |
| → | `NoImportInScope` | nothing is left |

The refusal at step three is right and must stay: minting `lib·sensei_bootstrap·…`
for our own crate is exactly the false-external defect `graph_nodes` was built to
end. The bug is that after refusing, there is nowhere to go.

## Measured

On sensei's converged index, 2026-09-30 (1,748/1,759 files, queue drained):

```
 sensei_bootstrap::SenseiConfig::from_env  | no_import_in_scope | 11
 sensei_bootstrap::util::which_binary      | no_import_in_scope |  7
 sensei_bootstrap::SenseiLocalConfig::load | no_import_in_scope |  5
 sensei_logger::Logger::noop               | no_import_in_scope |  5
 sensei_bootstrap::config::positive_or     | no_import_in_scope |  3
```

`cross_package` settled at **100** and stopped climbing — those 100 are the
references written **with** a `use`, which reach `owned_by_this_scan` through
`through_an_import` (`resolve.rs:1075`/`1376`). Everything spelled inline is lost.

`sensei_bootstrap::config::positive_or` is a function added to this repository
today, so this is current code, not stale index.

## A hypothesis recorded as WRONG

The obvious first guess is a separator mismatch: the manifest says
`sensei-bootstrap`, the source says `sensei_bootstrap`. It is not that —
`same_package` (`resolve.rs:1509`) already folds `-` to `_`, and
`owned_by_this_scan` uses it. Worth recording so it is not re-investigated.

## Design

A new rung, **`RootedInAScannedPackage`**, placed between
`rooted_in_this_package` and `a_fully_qualified_external`:

```rust
if let Placed::Proven(fqn) = self.rooted_in_a_scanned_package(&wanted) {
    return Resolution::Resolved { fqn, via: Rung::RootedInAScannedPackage };
}
```

It reuses machinery that already exists:

- `owned_by_this_scan(head)` — is the head a package we own, however spelled?
- `identity(package, segments, reach)` — mint an fqn in a NAMED package, which
  already takes the package as an argument rather than assuming the file's own.

So the implementation is: if the head names an owned package, mint the identity
in *that* package from the remaining segments.

### Precedence 42

| rung | precedence |
|---|---|
| `rooted_in_this_package` | 40 |
| **`rooted_in_a_scanned_package`** | **42** |
| `fully_qualified_external` | 50 |

Below `rooted_in_this_package`: that rung's path is rooted in the file's own
package, the narrowest and most certain scope. Above `fully_qualified_external`:
this target is ours and that one is a library, and being ours is the stronger
claim.

### The reason code must be seeded FIRST

`edge_verdict` reduces by precedence and **silently drops an unregistered code**
— which is how `settled_by_scope` came to have 0 rows in the live database
despite being emitted in code. The `reason_codes` row goes in, is imported, and
is verified present **before** the rung is emitted.

## Done gate

1. `sensei.reason_codes` holds `rooted_in_a_scanned_package` at precedence 42,
   verified by query, before any edge carries it.
2. Unit test: a path headed by a sibling package resolves to that package's fqn;
   mutation-probed by removing the rung.
3. A path headed by a package we do NOT own still reaches
   `fully_qualified_external` — the new rung must not swallow real libraries.
4. Forced re-index of **sensei only**.
5. `cross_package` in `structure_edges` rises above 100.
6. **`senseid → sensei-bootstrap` appears as a placed edge** — the specific pair
   this exists to fix, and the gate item inherited from #207.
7. No regression: `in_module` and `cross_module` counts do not fall.

## Wrong gate

- **A library swallowed as first-party.** If `owned_by_this_scan` is loosened
  rather than consulted as-is, `tokio::spawn` becomes a first-party edge into a
  package that has no source here.
- **Minting in the wrong package.** The identity must be minted in the package
  the HEAD names, not the file's own — otherwise the edge points at a
  same-named symbol in the caller's crate, which is the R4 defect the
  `declared_by_its_type` ordering comment already describes for `ArtifactKind`.
- **The rung placed above `through_an_import`.** An explicit `use` outranks an
  inline path; inverting them would let a sibling-rooted guess overrule a
  written-down import.

## Related

- #207 — first_party is every package the repo owns (closed; this is its
  follow-up)
- #205 — the Structure screen, whose cross-cutting edges this widens
