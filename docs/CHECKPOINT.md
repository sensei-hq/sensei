# Checkpoint

**Slice** — indexer-v2. **Stage 4 exists end to end**: a repository loads,
places, walks twice, and resolves. Detail: `/sensei:session`.

**Done** — `34ba2594` `index::index_repo`, the composition nothing had
(read → barrier → resolve) · `3b47cf19` `index::load_repo`, the IO half ·
`83593528` the no-filesystem guard · `b62545f7` shipped python walk bounded.

**Measured over THIS repository** — 2,309 files, 1,367 loaded, 942 unclaimed,
**0 unplaced, 0 unreadable**, 15 packages. 96,297 references resolved / 96,794
unresolved (ExternalBoundary 31,922 · ReceiverTypeUnknown 28,328 · Plumbing
22,466). No `Unplaced` survives the ladder; no module claimed by two files.

**Two defects the composition caught in its first hour** — python relative
imports resolved to the IMPORTING file (`from . import a` → a self-edge; 79 in
two corpora). Cause: the dot is python's separator AND its relative prefix, and
`segments()` drops empties, so the dots never reached the roots table. Depth is
now counted off the raw specifier pre-split (`Grammar::relative_depth_prefix`).
The 309 remaining self-edges are all `use super::*` in inline test modules —
correct, and the detector asserts the SHAPE so a new cause still fails.

**Gate at HEAD** — fmt clean, clippy `-D warnings` 0, workspace **3632/0**,
ignored 33/37 (4 pre-existing: dbd-rs path, gateway config, 2 installer hooks).

**Don't re-derive** — scan root → scan repo → files → `indexer(repo, file)`; an
adapter indexes ONE file and is TOLD its `(package, module)`. `TypeHomes`/`World`
are the repo-scoped lookups, built once between the two passes; within a pass no
file depends on another, so it is parallelisable as it stands. Two passes are
needed for IDENTITY, not existence — a stub is promotable only if both sides
spell the same string, and the member spelling carries the type's home. Prefer a
conservation property over a count, and a shape assertion over `is_empty()` when
a detector has known-benign hits.

**Next — persist + dispatch.** `persist::write(store, folder_id, facts)` is the
remaining wire; `RepoResult.folder_id` is the scope it wants. Then per
`docs/spec/indexer/10-cutover.md`: differential harness (S1) and BEFORE numbers
(S6) come before any switch, **rust only** (S3/D4), all three deletion triggers
re-pointed together (S4), graph WIPED not migrated (S4b).

**Also open** — a self-loop edge may be worth dropping at write time. kotlin/vue
deferred. TS/Svelte receiver typing: decomposed, **re-measure first**.
