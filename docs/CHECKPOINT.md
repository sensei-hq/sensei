# Checkpoint

**Slice** — indexer-v2, reach correctness. Plan + breaking tests:
`/sensei:session status`.

**Done** — 27 commits. rust: `binding_of` segment fix (582ce311), container
reporting (e7bf8ae6, a9d5c427), one-binding destructured params (e256f572,
lost 2048→1855). **java reach fix (86543e96)** — a field declaration mints at
`Reach::Field` but every member use site was minted at `Reach::Item`, so a
field read could never meet its own declaration: **Field calls 0→3190**, TOTAL
lost 8697→8550. File-module step 1/10 (b983367a): re-anchored R10.3's brake,
which `now.is_empty()` would have killed silently once every file claims
itself. Gate: fmt clean, clippy 0, workspace 3580/0, java ratchets 4/4.

**Corrected** — "+1262 file-modules" was WRONG: 373 of 389 rust file identities
already come from `mod x;`, so rust gains ~16. Web 946→622 distinct.

**Unpredicted cost** — java EnumVariant calls 280→38: the move strands nothing
in the WALK but does in the LADDER (`refer_to_member`'s unresolved branch still
emits `Reach::Item`, deliberately — threading Field hits the field guard before
`through_an_import`). Net +2933 for −242; fix the guard, not the reach.

**File-module 3/10** — brake (b983367a) + `Ladder::blocks` filter (a78ef0c6: a
whole-file span doubled the last module segment everywhere). `enumerable`'s lock
was mutation-probed, found VACUOUS, deleted — covered transitively.

**Next — steps 4-10**, ordered, red-first: emit in rust (389
files, 0 collisions) → emit in javascript (272 fqns multi-claimed) → add
`RelationKind::Contains`, no emission → emit it → wire `persist::owners()` with
explicit Owns-over-Contains → guard that Contains never becomes a declared
member → re-measure every `#[ignore]d` ratchet by running it.

`Contains` feeds `parent_id` ONLY, no edge row: `edge_kind.ddl` has no
'contains' and graph.rs:1810 already deleted 7,916 rows of that shape.
**Step 11 (imports)** — decided, not built: `can_be_named()==false` restated as
"call-shaped", `ReachedBy::Import` gets real columns, module lost derives from
Imports evidence. Needs a per-shape target rule — `use a::b::C` names an ITEM.
Java excluded. **TS/Svelte** — decomposed, not started, RE-MEASURE first: the
numbers predate 7 indexer commits including one attacking the same bucket.
