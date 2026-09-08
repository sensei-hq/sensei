# Checkpoint

**Slice:** rust transitive receiver resolution (#151). Branch `develop`.

## Landed — gate verified by me, not taken on report: fmt 0, clippy 0, 3,185 tests / 0 failed

- `b466c3e5` `expected_files` — the folder-completeness denominator.
- `09d41f6d` `sensei.folder_completeness` view — completeness from FILES, recursing only
  over `parent_id`, never reading `folders.status`. One UPDATE settles the hierarchy.
- `5aeb5caf` `FqnDefinition.return_type` carried by the minting pass.
- `c4fe6fcc` transitive receiver resolution, qualified by module.

## Measured payoff — SMALL, and that is the honest number

Live, after a full reindex of all 48,654 files:

| | before | after |
|---|---:|---:|
| nodes carrying a return_type | 0 | **8,509** |
| rust call edges carrying a receiver hint | 0 | **1,990** |
| `get_callees(scan_root)` resolved | 24 | **25** |

The mechanism is live end to end and correct. The RESOLUTION gain is about one call
per symbol. Stored `edges.target_id` does not move because resolution is on the READ
path by design (order-independent, no reindex needed to benefit).

Why so small: qualification (needed to stop wrong-merges) refuses every case it cannot
prove, and ~97% of this repo's return types are written BARE, where only the count
gates apply. The 12,385 hintless references are chains deeper than one link — they
need resolved types fed back into the binding map, which is the next level down.

## Three HIGH wrong-merge defects were found AFTER the suite was green

All by adversarial review; a 3,179-test green suite caught none of them.
1. Uniqueness gate counted MEMBERS not TYPES — two same-named types with disjoint
   members both passed. Proven live: `Verdict::as_str`, `Session::wall_ms` (cross-crate).
2. External return types reduced to a bare name and hunted among first-party nodes
   (`fn client() -> reqwest::blocking::Client` -> `"Client"`).
3. `call_coverage` healed both directions but `get_callers_by_name` did not — reported
   `complete: true` beside `resolved: false` on the same row.
Root cause of 1+2 was the same and both reviewers found it independently: the return
type is stored verbatim so the module path can disambiguate, then discarded before use.
Fixed by qualifying (`SelfType | Qualified{module,name} | Bare`), not merely refusing.

## Known and NOT closed

- A BARE return-type name belonging to a dependency (`use reqwest::Client;
  fn f() -> Client`) still falls to the name path. The obvious guard was rejected on
  measurement: all 21,937 lib nodes have `language = NULL`, so it would refuse a Rust
  `Session` because a Python package exports that name. **Prerequisite: record a
  language on lib nodes**; then it is ~6 lines in the `Bare` arm.
- 2,142 rust edges resolve onto a STUB. `ReceiverHint::Type` was removed rather than
  wired, because wiring it while hop 2 matched on bare name would have relocated
  correctly-resolved calls onto wrong targets.
- #152: rust IMPORT resolver ignores `local_modules` (5 phantom survivors).
- `cluster:scheduler` folder fails to index; undiagnosed.

## Next

The remaining mass is the 12,385 hintless references. Feeding resolved receiver types
back into the binding map is what reaches them — same defect one level down.
