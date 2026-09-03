# Checkpoint

**Slice:** capability-refactor slices 0 + 1 DONE (`0a31582e`, `e49b1ac1`).
Branch `develop`. Gates: rust 2,705/0, app 1,698/118, clippy 0, fmt clean.
Plan: `docs/spec/2026-09-02-capability-refactor-map.md` (+ the ADR beside it).

## Slice 0 — 45% of the graph was not your code

A vendored Node/OpenSSL tree was indexed from a path that HAD an exclusion
entry. The entry read `find-me-board/docs/proposal/deck-node`; the real path was
`pre-sales/find-me-board/…`. `root_exclusion_prefixes` correctly joins
`root + entry`, so the resolved prefix pointed nowhere, matched nothing, and
**nothing said so**.

The fix is the silence: `check_exclusions` resolves each entry through the shared
resolver, probes disk, warns, and returns `exclusionChecks` from
`add_watch_root`. Also unified the matcher — `add_watch_root` registered the
watcher with RAW relative entries while `scan.rs` uses resolved absolute ones,
which is why `should_watch_path` had its own two-form copy that could disagree.

| | before | after |
|---|---|---|
| nodes | 728,742 | **399,722** (−45.1%) |
| files | 51,973 | 46,516 |
| nodes under excluded path | 329,087 | **0** |
| largest kind | `const` 292,555 | section 141,088 · function 95,168 |

## Slice 1 — one registry, and a matrix that cannot lie

`extensions()` is declared per adapter with **no default**; `all_adapters()` is
the single list; `adapter_for_ext` derives from both. Frameworks declare
`host_language()` (svelte, vue → typescript), which is why a `.svelte` file's
symbols carry `typescript·` fqns. `capability_matrix()` is surfaced as
`languageCapabilities` on `get_project_summary`, beside `graphHealth`.

It reads a **declared** `supports_fqn` — cheap enough to serve — but the
declaration is not trusted: `declared_fqn_support_matches_a_real_probe` builds a
per-language fixture and CALLS `fqn_output`, failing the build on disagreement.

```
python rust typescript javascript java sql svelte vue   fqn=true
swift kotlin c                                          fqn=FALSE
svelte, vue                                             host=typescript
```

Those three gaps were invisible before: `fqn_output`'s `None` default could not
distinguish "no such concept" from "not written yet", so their symbols stayed
findable by NAME while an fqn lookup could never see them.

## Next — slice 1b

FQN for **swift, kotlin, c**. `fqn_support_gaps_are_named_and_must_only_shrink`
pins the list at exactly those three; it may only shrink, and empty is done.
Kotlin is closest to Java (package declarations); C needs a file-scoped scheme
(`c·<package>·<file>·<name>`), constructible and more precise than name-matching.

Then slice 2 (`Inheritance` — java/python/rust already EXTRACT it, so that slice
is persistence, not parsing) and the rest of the map.

## Graph state

399,722 nodes / 46,516 files / 8,855 folders. Edge resolution: calls 65.1% ·
references 30.8% · imports 18.9% (every LOCAL one) · extends 0% — and `extends`
is misnamed containment, not inheritance (see the map).

## Traps

Run the full gate with the daemon STOPPED (`metrics_pipeline_end_to_end` is
timing-sensitive under contention). Never `git checkout --` to undo a mutation —
use a `cp` backup. Never pipe a gate through `| head` (SIGPIPE truncation masked
a real `fmt` failure). The leak hook rejects `/Users/x/` — use `/Users/dev/`.
`sensei_test` has no automated schema provisioning, so DDL changes need applying
to both DBs. Verifying an indexer fix needs `delete from sensei.scan_state`
first, and stale nodes for now-excluded files need explicit deletion.

## Lesson worth keeping

I misdiagnosed slice 0 twice before printing what the resolver ACTUALLY produces
and testing it against disk. Trace the value end-to-end first; theorising from
one layer cost three wrong answers.
