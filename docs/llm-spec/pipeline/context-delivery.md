# 送 · Pipeline · Context delivery

**Owner files:**
- Assembly: `crates/senseid/src/context/pack.rs::context_pack`
- Ranking: `crates/senseid/src/context/rank.rs`
- Resolution levels: `crates/senseid/src/context/resolution.rs`
- Session dedup: `crates/senseid/src/context/dedup.rs`
- MCP: `crates/mcp/src/tools/context_pack.rs`

**Companion design doc:** [`docs/archive/ideas/14-context-delivery.md`](../../archive/ideas/14-context-delivery.md).

## Purpose

Context delivery answers a hard question — *"what code does the
assistant need to see right now, at what resolution, to answer the
current task without wasting tokens?"* Grep is cheap and loose;
loading whole files is expensive; the graph knows the middle
ground. Context delivery is the middle ground.

Four levers:

1. **Resolution levels** — every code node is servable at multiple
   depths. L0 = signature; L1 = signature + IO pattern; L2 =
   signature + IO + logic-flow summary (LLM-generated); L3 = full
   source. The caller (or the daemon) picks the smallest depth
   that answers the task.
2. **Token budgeting** — a hard cap per response; assembly walks
   the graph up to the budget, most-relevant paths first.
3. **Task-relevant ranking** — diff-first BFS (start from what's
   changed), traceability boost (docs → code paths), semantic
   fallback (vector similarity when structural walk runs out).
4. **Session dedup** — the assistant doesn't need the same code
   sent twice; the pack knows what it's already handed over this
   session.

Kanji is 送 — *to send / deliver*.

## Data invariants

### Resolution levels

- `sensei.node_summaries` — one row per (node, level) with:
  - `node_id`, `level` (`l0 | l1 | l2 | l3`), `body` text,
    `tokens` int, `generated_by` (`extraction | ollama-gemma4 | none`),
    `generated_at`.
- **L0 signature** — extracted, no inference. Function name +
  parameters + return type. Tiny.
- **L1 IO pattern** — signature + inferred IO shape (side
  effects, throws, mutations). Extracted where possible; falls
  back to L0 when the language / AST can't infer.
- **L2 logic flow** — LLM-generated one-paragraph summary via
  [[pipeline/inferencing]] `reasoning` chain (gemma4). Regenerated
  on file change.
- **L3 full source** — raw file slice. Expensive but sometimes
  necessary.

Higher levels **cost more tokens**. The pack function chooses
level per node based on:

- Task type (edit → likely need L3 for the touched functions,
  L1 for the neighbours).
- Distance in the graph from the task's focal point.
- Token budget remaining.

### `context_pack` — the assembly

    context_pack(
        task_description,
        focal_nodes,          // starting points — files in diff, mentioned symbols
        max_tokens,
        session_id?,          // for dedup
        options: { rank_mode: "diff_bfs" | "semantic" | "hybrid" }
    ) -> ContextPack {
        nodes: [ { node_id, level, body } ],
        total_tokens,
        summary,              // model-generated overview of what's in the pack
        omitted: [ … ]        // nodes that were dropped due to budget
    }

Algorithm:

1. Compute or read the summary for each focal node.
2. BFS along `sensei.edges` starting from focal nodes;
   prioritise: (a) direct callers/callees, (b) callers of callers
   for diff-first, (c) doc-linked references (see
   [[pipeline/traceability]]).
3. For each node visited, decide the resolution level using the
   heuristics above.
4. Accumulate token usage; stop when the budget is exceeded.
5. If BFS runs out before the budget, fall back to semantic
   similarity against `sensei.node_embeddings`.
6. Deduplicate against `sensei.session_context_log` if
   `session_id` is provided.

### Session dedup

- `sensei.session_context_log` — per session, list of
  `(node_id, level, sent_at)` rows.
- On a subsequent `context_pack` call for the same session, nodes
  already sent at the same or higher level are skipped (or
  downgraded — if the user asked for L1 last time and we're
  giving L3 now, we still need to send).
- Retention: 24h TTL; a long-running session gets its cache
  pruned when a new turn is more than 24h older than the previous.

## Signals produced

| Signal | Consumer |
|---|---|
| Assembled ContextPack | assistant call (via MCP) |
| Token efficiency delta | Insights ("token spend down 30% since ranking landed") |
| L2 summary quality (feedback) | Learning signal for the gemma4 summary prompt |
| Dedup hit rate | Perf metric |

## Done gate

- Every code node has an L0 summary within the incremental
  window after it lands / changes.
- L2 summaries are generated for the top-N most-referenced nodes
  per project (background job); requested-but-missing summaries
  are generated on demand within a bounded latency (500ms via
  gemma4 warm).
- `context_pack` returns a pack that fits within `max_tokens`
  (± 100).
- Dedup skips content the assistant already saw this session.
- The pack's node list has a `summary` line the assistant can
  read to understand what's in the pack without re-reading every
  node.
- BFS-first ranking is measurably better than random for the
  test set (established baseline before rolling into
  production).

Optional check:
```
mcp_call context_pack \
  --task="add a new health signal for tool timeouts" \
  --focal_nodes='["sensei.tool_signals::derive_signals"]' \
  --max_tokens=8000 \
  | jq '{n_nodes: (.nodes | length), total_tokens, omitted_count: (.omitted | length)}'
```

## Wrong gate

- **Pack returns 20k tokens on an 8k budget.** Budget enforcement
  broken.
- **L2 summary is stale — file changed but summary not
  regenerated.** Watcher didn't invalidate.
- **BFS returns callers when the task is doc-driven** (a design
  doc referencing a symbol). Traceability boost missing.
- **Dedup lets the assistant see the same node twice at the
  same level.** Session log query wrong.
- **Every task returns the same nodes regardless of task
  description.** Ranking not consulted.
- **Falls back to semantic when a clear BFS path exists.**
  Ranking order inverted.
- **L2 generation blocks the request.** Should be background;
  on-demand generation has a bounded timeout with fallback to
  L1.

## Related

- [[pipeline/inferencing]] — L2 generation + task-relevant
  reasoning
- [[pipeline/capture]] — indexed code + embeddings
- [[pipeline/analyzer]] — invalidates L2 summaries on change
- [[pipeline/traceability]] — traceability boost for ranking
- [[pipeline/memory]] — memory context competes for the same
  token budget on session-start assembly
- [[pipeline/mcp-surface]] — `context_pack` tool
- [[archive/ideas/14-context-delivery]] — the source design
