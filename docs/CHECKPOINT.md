# Checkpoint

**Slice** — indexer-v2. **File-module steps 1-11 COMPLETE**, and the processor
seam under them exists. Detail: `/sensei:session`.

**Done since d0dfdd8f** — step 11 `RefKind::Imports` (`verdict()` returns an
edge kind, so a module import is an edge instead of a short-circuit) · java
file-module · `a9d5c427` the container row counts only, no "not imported" ·
`8e01c92e` a python file outside an `__init__.py` chain is still in a package
(SHIPPED indexer — it was filing first-party code as a library) · `9c49a87b`
**`indexer::placement`**, the seam that answers "which package is this file in".

**Measured** — the seam was pinned against the `#[cfg(test)]` harness over this
repo's rust and 6 of 390 files disagreed. The harness was wrong: `module_of`
split on `/src/`, so `build.rs` and `tests/*.rs` got an EMPTY module — the
library crate root's — and `build.rs`'s `fn main` minted `src/main.rs`'s
identity. A7 rust collisions **5 → 3**.

**Gate at HEAD** — fmt clean, clippy `-D warnings` 0, workspace **3595/0**,
ignored 31/35, python 30/30. The 4 ignored failures are pre-existing and proven
so (dbd-rs path, gateway config, two installer-hook tests).

**Don't re-derive** — `placement` composes `adapters::manifest` (10 ecosystems)
rather than adding an eleventh reader. A manifest stating no name yields NONE:
a virtual workspace root and a private `package.json` are both legitimate, and
a folder-derived name is one no dependency edge spells. `build.rs` keeps module
`"build"` DELIBERATELY — with `""` its declarations collide with lib's.

**Next — port python to a v2 adapter.** Ranked #1 of the remaining six: ~29k
files, the largest corpus outside rust/ts, and the seam it was waiting on is
now in. v2 has 4 adapters (rust, js, ts, svelte); the SHIPPED `languages/`
indexer has 11 and still produces the graph, so it must keep working.

**Open for the user** — should `.git` be a last-resort python import-root
marker? It would place ~251 of the 308 markerless files, but a VCS boundary is
not a python import root. Not decided quietly.

**Also open** — kotlin/vue deferred, sql/c/swift/go/scala/dart dropped.
TS/Svelte receiver typing: decomposed, not started; **re-measure first**.
