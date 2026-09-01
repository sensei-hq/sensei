# Checkpoint

**Slice:** stored-derived-value cleanup. Last commit `d79cfc9f`. Branch `develop`.

Five commits. The governing rule: *if a value is a pure function of current
stored state, it is a view — not a row somebody writes.* It held four times out
of five, and the fifth is the useful correction.

## Done this run

| commit | what |
|---|---|
| `796a56a9` | retired `folders.props.libs` — the proxy was 75–91% this repo's own code |
| `89e46395` | `covers` → the `doc_coverage` view; 601 pairs reproduced exactly, 0 divergent |
| `bc59b4c4` | dropped `nodes.degree` + deleted the `build_connections` barrier (−474 lines) |
| `d79cfc9f` | stopped re-detect destroying 1,545 model-authored community descriptions |

## The correction, recorded accurately

`d79cfc9f`'s message says "measurement is what caught it" about `god_node_ids`
not being viewable. **That is false and the commit message is wrong.** The
11.3s/21.9s degree-in-SQL numbers are in `bc59b4c4` — the PREVIOUS commit. The
measurement was already in hand; the failure was writing a forward plan
("communities' four derived columns become a view") that the measurement had
already invalidated, since `god_node_ids` is ranked by degree. Noticing that a
turn later was re-reading my own result, not a new finding.

The technical conclusion is still right: `inference.communities` legitimately
materialises `god_node_ids` (too expensive to derive) and `description` (not
derivable), with `label`/`node_count` cheap by-products of the same pass.

**Sharpened rule:** "if it's derivable, make it a view" is only true when
deriving it is affordable. Measure the derivation cost BEFORE planning the view,
and re-check any forward plan against measurements already taken.

## Gates at d79cfc9f

2657 senseid pass / 0 fail / 6 ignored, clippy `-D warnings` 0, fmt clean.
Every behavioural change mutation-probed: the god-node test placed the hub last
in natural-key order and was verified red against the old implementation; the
description-discard test fails if the CASE is relaxed to preserve
unconditionally (left: 1, right: 0).

One flake seen mid-run — `full_bridge_publishes_status_segments` — passes in
isolation; parallel interference, unrelated.

## Next command

```
gh issue view 146
```

Identity-fabrication branches (`rust_lang.rs:981`, `rust_lang.rs:1171`,
`typescript.rs:1018`, `java.rs:620`) landing WITH stub GC in the same commit —
`delete_edges_from_sources` removes the in-edges that prove the fix worked, and
`prune_file_nodes` filters `file_path = $2` while stubs have `file_path IS NULL`.

The invariant is now a query, which is what the view was for:

```sql
select folder, count(*) from sensei.graph_nodes
 where locality = 'unknown' group by 1 order by 2 desc;
-- 84,442 total. OmniRoute 11650, sensei 10618, cluster 8228, server 7479
```

Target 0 — NOT resolution rate, which a correct fix makes FALL.

`java.rs:620` confirmed: `import static org.mockito.Mockito.when;` DOES populate
`imports["when"]` and the unqualified-call branch never reads it.

## Open questions

- **Fallback is the wrong default.** The word names two opposite operations:
  *widening* (look in A then B for the SAME question, still able to answer
  "don't know" — legitimate, e.g. `which_binary`) and *substituting* (answer a
  DIFFERENT question and present it as the answer — a fabrication). Mechanical
  test: **can the fallback path return "I don't know"?** If it cannot fail, it
  is a fabrication. All four #146 branches fail that test, as did the `libs`
  proxy and `get_project_summary`'s dead project→folder fallback. `CLAUDE.md`
  forbids this on *failure* paths (`Err`) but is silent on *miss* paths
  (`None`) — the gap every one of them used. Proposed as a rule; not yet written.
- **#149** — does label propagation earn its keep? 62% of communities restate the
  file boundary, 36% are stub singletons. Deliberately not actionable until #146
  removes the stub artifacts and the numbers can be re-measured fairly.
- **#138** project/namespace is a decision, not a task.
- **#137** blocks 4,329 user-scope sync rows.

## Known-broken

- `docs/analysis/graph-end-state-sketch.md` §1–12 remain **NOT SAFE TO BUILD
  FROM**; its correction block is authoritative.
- **`references` targets are not resolvable data** — `/` alone has 2,000 edges;
  also `id`, `true`, `429`. `doc_indexer.rs:587` joins absolute paths, discarding
  the repo root (54,740 absolute targets). 0 of 1,700 rationale nodes have any
  out-edge, so `references` does not unlock `rationale_for`.
- **`extends` is 0-resolved** across all 7,863 edges while `codebase.rs:55`
  already consumes it — #147.
- **`dojo_memberships.sync_status` is dead** — `set_sync_status` has no callers,
  so the connections pane's "healthy" count is permanently 0.
- `/settings/projects` e2e fails on a fresh DB (#134).
- **The security-reminder hook false-positives** on the regex `match` sibling
  method in ANY file including Markdown.

## Filed, not started

#131–#136, #139–#142, #144, #145, #146, #147, #148, #149.
