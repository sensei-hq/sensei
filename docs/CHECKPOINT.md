# Checkpoint

**Slice:** rust receiver resolution + anchoring (#151, #152). Branch `develop`.

## Landed — gate verified by me each time: fmt 0, clippy 0, 3,187 tests / 0 failed

- `b466c3e5` `expected_files` denominator · `09d41f6d` `folder_completeness` view
- `5aeb5caf` `FqnDefinition.return_type` carried by the minting pass
- `c4fe6fcc` transitive receiver resolution, qualified by module
- `43641ba4` `let` bindings carry provenance (`Binding::Type | ReturnOf`)
- `4f4791f7` an unplaceable type is a MISS, not the caller's module

## Measured live — full reindex of all 48,654 files after 4f4791f7

| | before | after |
|---|---:|---:|
| ghost-stub edges (resolved onto a node with no file) | 7,267 | **4,542** (−37%) |
| REAL resolved (file-bearing targets) | 46,487 | **46,545** (+58) |
| reported resolved | 53,754 | 51,087 |
| unresolved | 28,237 | 29,875 |
| hinted edges | 1,990 | 1,900 |
| `get_callees(scan_root)` resolved | 25 | 23 |

READ THIS THE RIGHT WAY. "Resolved" fell by 2,667 and ghosts fell by 2,725 — those
are the same edges. No real link was lost (real resolved went UP 58). The graph got
2,725 edges MORE HONEST: they pointed at invented nodes and now say unresolved.

## What did NOT work, stated plainly

`43641ba4` (let-binding provenance) was expected to reach a chunk of the 12,385
hintless references. Measured, hints went 1,990 -> 1,900: essentially flat, slightly
down. The anchoring fix removed the ghost anchors many hints were riding on, and
provenance did not add enough to offset it. The change is still correct — it just did
not buy reach on this corpus. Do not claim otherwise.

Cumulative honest position: the architecture is right and the graph is measurably
more truthful, but receiver resolution has NOT delivered a meaningful increase in
resolved first-party calls. ~2% reach was the ceiling and the ghost cleanup consumed
the visible part of it.

## Two tests were found PINNING the ghost behaviour

`rust_ref_fqn_self_local_bounded` asserted `let g = Gadget::new(); g.spin()` resolves
to `rust·senseid·engine·Gadget·spin` while never declaring `Gadget` — that fqn WAS the
ghost, and the test had guarded the bug for as long as it existed. Same hole in
`rebinding_a_name_replaces_its_provenance`. Fixtures fixed, no expectation relaxed.

## Known and NOT closed

- 4,542 ghost-stub edges remain. Sources: glob imports, `pub use` re-exports,
  macro-generated impls — none placeable from one file.
- A BARE return-type name belonging to a dependency (`use reqwest::Client;
  fn f() -> Client`) still falls to the name path. Prerequisite for the guard:
  **record a language on lib nodes** (all 21,937 have `language = NULL`), then ~6
  lines in the `Bare` arm.
- `ReceiverHint::Type` was removed, not wired (would have relocated correctly
  resolved calls while hop 2 matched on bare name).
- #152: rust IMPORT resolver ignores `local_modules`.
- `cluster:scheduler` folder fails to index; undiagnosed.

## Next

The remaining mass is unresolved receivers whose type no single file can name. That
needs either cross-file type propagation at index time, or accepting the ceiling and
investing in the other languages instead — TS/JS/Python/Java, where an import names a
file and the same machinery should pay off far better.
