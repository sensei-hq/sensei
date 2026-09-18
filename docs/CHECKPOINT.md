# Checkpoint

**Slice** — indexer-v2. **Python is now a v2 language**, and the seam under it
places real checkouts. Detail: `/sensei:session`.

**Done since 8e01c92e** — `9c49a87b` `indexer::placement`, the seam that answers
"which package is this file in" · `04daaa3e` the python adapter (grammar, walk,
identity rules) · `ecb9718f` **shipped** manifest fix: poetry states its name in
`[tool.poetry]` · `f221b76a` one directory, two manifests, one answer every run
· `fbb0ac84` the walk's double traversal, and the corpus test that found it.

**Measured** — five python checkouts, 239 files, all placed, all read. The
conservation property holds on every one: **every `def`/`class` line produces
exactly one Class/Function/Method symbol** (Ethico 367/367, ai-hedge-fund 577,
llm-gateway 248, revamp 53, 202410 42, green-card 41). A7 rust collisions 5 → 3.

**Gate at HEAD** — fmt clean, clippy `-D warnings` 0, workspace **3620/0**,
ignored 32/36. The 4 ignored failures are pre-existing and proven so (dbd-rs
path, gateway config, two installer-hook tests).

**Don't re-derive** — my fixtures could not catch the double traversal: every
walk test used `.find()` and a duplicate reads like the original. Twelve tests,
four mutation-probed, all blind. The corpus caught it (37,973 refs / 103 files).
**Prefer a conservation property over a count.** `build.rs` keeps module
`"build"` deliberately — with `""` its declarations collide with lib's. Python's
`Name` binding names a module (unlike Rust's) because `import x` REQUIRES x to
be a module; an item arrives as `from a import b`.

**Next — wire the seam into a production processor.** `placement` and the four
adapters are still `#![allow(dead_code)]`: `pipeline.rs` stops at the structure
barrier and nothing outside `indexer/` calls any of it. That is the cutover, and
it is what makes any of this reach the graph.

**Open for the user** — should `.git` be a last-resort python import-root marker
for the SHIPPED indexer? It would place ~251 of 308 markerless files, but a VCS
boundary is not a python import root. Not decided quietly.

**Also open** — kotlin/vue deferred, sql/c/swift/go/scala/dart dropped.
TS/Svelte receiver typing: decomposed, not started; **re-measure first**.
