# Checkpoint

**Slice:** graph locality view. Last commit `796a56a9`. Branch `develop`.

Recommended order was view → fabrication branches + stub GC → MCP reshape.
**The view is done.** The two follow-ups Jerry split out are filed as #147/#148.

## Done this run

| commit | what |
|---|---|
| `95ed30f1` | parsers: one grammar-driven import reader per language (five splitting copies removed) |
| `3bebd7a5` | docs: mark the four parser bugs fixed; separate collection from fabrication |
| `33e59a95` | `sensei.graph_nodes` — locality + hierarchy, read off the node |
| `796a56a9` | stop overwriting the manifest dependency list with the import-edge guess |

### The view

Locality is a property of the NODE, so `graph_nodes` reads what the writer
recorded (`kind`, `file_path`) instead of asking "did the edge fail to resolve?"
Three-valued on purpose — a boolean bins the stubs as external:

```
internal  338,027  78.4%
unknown    84,440  19.6%   <- the invariant for the next slice
external    8,517   2.0%
```

It is a **projection, not a rule copy**: `classify_import` parses specifier
strings and exercises judgement, so it stays in Rust with one owner; this view
reads that rule's recorded output. `nodes.resolved` is deliberately not the
signal (140,051 `section` rows are `resolved=false` inside real files) — pinned
by a test.

### The clobber

`build_connections` overwrote `resolve_libs`' manifest-derived list with the
proxy, by documented design. Live before/after, proxy vs called vs declared:

```
OmniRoute  5468 →  99 called / 124 declared
cluster    2673 →  10 called /  83 declared
sensei     1040 → 134 called / 113 declared
rokkit      890 →  32 called /  96 declared
```

Nothing observable regressed: both readers of `props.libs` are structurally
blind (verified live at 0 through each path) — evidence recorded on #148.

## Gates at 796a56a9

320 DB tests, 7 resolve, 6 graph_nodes, 2656 senseid, 174 bootstrap, 1698 app
unit / 118 files, clippy `-D warnings` 0, fmt clean. Mutation-probed: restoring
the proxy write fails the clobber test; restoring HEAD's parsers fails all three
corpus invariants.

## Next command

```
gh issue view 146
```

The identity-fabrication branches — `rust_lang.rs:981`, `rust_lang.rs:1171`,
`typescript.rs:1018`, `java.rs:620` — landing **with** stub GC in the same
commit. `delete_edges_from_sources` removes the in-edges that prove the fix
worked, and `prune_file_nodes` filters `file_path = $2` while stubs have
`file_path IS NULL`, so 84,440 stub nodes have no GC path.

The invariant is now a query, which is what the view was for:

```sql
select folder, count(*) from sensei.graph_nodes
 where locality = 'unknown' group by 1 order by 2 desc;
-- OmniRoute 11650, sensei 10618, cluster 8228, server 7479, scheduler 6561
```

Target: 0. NOT resolution rate — a correct fix makes that FALL.

`java.rs:620` confirmed by measurement: `import static
org.mockito.Mockito.when;` DOES populate `imports["when"]` (`java.rs:429-441`
strips `static` before taking the leaf) and the unqualified-call branch never
reads it.

## Open questions

- **Is fallback the right default?** Concluded no. "Fallback" names two opposite
  operations: *widening* (look in A then B for the SAME question, still able to
  return "don't know" — legitimate, e.g. `which_binary`) and *substituting*
  (answer a DIFFERENT question and present it as the answer — a fabrication).
  The mechanical test: **can the fallback path return "I don't know"?** If it
  cannot fail, it is a fabrication. All four bugs in #146 fail that test, as did
  the `libs` proxy and `get_project_summary`'s dead project→folder fallback.
  `CLAUDE.md` forbids this on *failure* paths (`Err`) but is silent on *miss*
  paths (`None`) — that is the gap every one of them fell through. Worth adding
  as a rule; not yet written.
- **#138 project/namespace** is a decision, not a task — the repository rung
  does not exist, so project aggregation would aggregate an empty set.
- **#137** blocks 4,329 user-scope sync rows (no writer sets
  `personas.principal_id`).

## Known-broken

- `docs/analysis/graph-end-state-sketch.md` §1–12 remain **NOT SAFE TO BUILD
  FROM**; its correction block is authoritative.
- **`references` targets are not resolvable data** — `/` alone has 2,000 edges;
  also `id`, `true`, `429`. `doc_indexer.rs:587` joins absolute paths, discarding
  the repo root (54,740 absolute targets). `references` does NOT unlock
  `rationale_for`: 0 of 1,700 rationale nodes have any out-edge.
- **`extends` is 0-resolved** across all 7,863 edges while `codebase.rs:55`
  already consumes it — #147.
- **`dojo_memberships.sync_status` is dead** — `memberships::set_sync_status` has
  zero callers, so the connections pane's "healthy" count is permanently 0.
- **Removing an exclusion enqueues a re-scan that races the prune** — harmless
  now that the scan honours exclusions, but the sequencing is unguarded.
- `/settings/projects` e2e fails on a fresh DB (#134).
- **The security-reminder hook false-positives** on the regex `match` sibling
  method in ANY file including Markdown.

## Filed, not started

#131–#136, #139–#142, #144, #145, #146, **#147** (post-index pattern
recognition), **#148** (MCP response shape: empty ≠ not-found).
