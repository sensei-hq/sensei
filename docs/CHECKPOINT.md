# Checkpoint

**Slice** — indexer-v2. **File-module steps 1-10 COMPLETE.** Every file declares
its own module; containment feeds `nodes.parent_id`. Detail: `/sensei:session`.

**Done** — 33 commits: b983367a brake · a78ef0c6 `Ladder::blocks` filter ·
65b3c844 omittable-only extensions · e6dff723 rust emission · ef194750
js+svelte · 8e4e803f `Contains`, unrepresentable as an edge · bcd39d22 walks
emit it, `owners()` reads it, anti-widening guard · d0dfdd8f per-app packages.

**Measured** — rust Module 325→382 (one per file), typescript 0→633. Both
callable tables UNCHANGED (rust lost 1855, ts lost 2180): a container
contributes to neither. Web identities 619→925 (267 collisions→0). The A7
identity ratchet came DOWN, 525→514.

**Gate at HEAD** — fmt clean, clippy `-D warnings` 0, workspace **3586/0**,
ignored 30/34, java corpus 4/4 (`SENSEI_CORPUS=~/Work/Dayamed`). The 4 ignored
failures are pre-existing and PROVEN so — stashed, re-ran at HEAD, all 4 still
fail (dbd-rs path, gateway config, two installer-hook tests).

**Don't re-derive** — steps 7+8 had to be one commit (between them a `Contains`
reaches the DB nowhere). Containment is NOT a reach; counting it in
`from_source` marks every module reached by anything inside it. The A7 break was
the harness, not the emission: all three front ends were one package, so two
apps' `src/app.d.ts` were already one identity. Two repo guards fire on
COMMENTS, not code — describe a forbidden literal, never spell it.

**Next — step 11 (`RefKind::Imports`)**, decided, NOT built. Keep
`can_be_named()==false` restated as "call-shaped"; give `ReachedBy::Import` real
columns; derive the module lost count from Imports evidence. **Needs a
per-shape target rule first** — `use a::b::C` names an ITEM, so emitting it at
`Reach::Mod` dangles once per `use` line. Only a glob / grouped `self` / bare
mod / JS specifier names a module. Java excluded.

**Then the remaining languages.** v2 has 4 adapters; the SHIPPED legacy indexer
has 11 (python, kotlin, sql, swift, c, vue) and produces the graph today, so it
must keep working. Corpora: python ~29k, sql ~865, kotlin 245, vue 41, swift 6.

**Also open** — TS/Svelte receiver typing: decomposed, not started; re-measure.
