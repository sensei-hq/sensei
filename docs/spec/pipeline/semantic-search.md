# 探 · Pipeline · Semantic search

> **Status: roadmap (partial).** Semantic (pgvector) + structural (call-graph) + RRF fusion
> are real and beat grep for embedded symbols, but as of 2026-08-05 the owner files/tables
> below **do not exist** (`search/hybrid.rs`, `sensei.node_fts`, `node_embeddings`): there is
> **no FTS**, the lexical arm is a `name LIKE`, the MCP `search` is a keyword router, and
> embedding coverage is 26%. Verify against the code. See the
> [indexer capability roadmap](../../analysis/2026-08-05-indexer-capability-coverage.md).

**Owner files:**
- Search: `crates/senseid/src/search/hybrid.rs`
- Full-text index: `sensei.node_fts` (pg tsvector)
- Vector index: `sensei.node_embeddings` (pgvector)
- Structural: derived from `sensei.nodes` + `sensei.edges`
- Grep fallback: `crates/senseid/src/search/grep_fallback.rs`
- MCP: `crates/mcp/src/tools/search.rs`
- Hook interception: `marketplace/plugins/sensei/hooks/pre_tool_use.rs`

**Companion design doc:** `docs/archive/ideas/31-semantic-search-layer.md`.

## Purpose

Grep is the assistant's default reflex, but grep is a lousy
teacher. It finds a substring, not a concept. Semantic search is
the layer that lets the assistant ask *"where do we do X?"* and
get the right file even if the substring isn't a literal match.

Three modes in one query:

1. **Full-text (FTS)** — Postgres tsvector; fast, literal.
2. **Semantic (vector)** — pgvector cosine similarity against
   node embeddings; concept match.
3. **Structural** — walk `sensei.edges` from a symbol / community
   / pattern; matches by graph relation.

Hybrid ranking merges results with a confidence chip per hit.
When the daemon can't find anything meaningful, it explicitly says
so and the caller falls back to grep — sensei is **never worse
than grep**.

Kanji is 探 — *to search / seek*.

## Data invariants

### Indexes

- `sensei.node_fts` — one row per node with `tsvector` over
  name + doc + inline comments + body summary.
- `sensei.node_embeddings` — one row per node with an embedding
  (via `pipeline/inferencing` `embedding` chain — currently
  `nomic-embed-text` locally).
- Incremental — capture / watcher invalidates changed nodes;
  re-embed happens on the analyzer tick for changed nodes only.

### Query shape

    search(
        query: string,
        project?: uuid,
        mode?: "auto" | "fts" | "semantic" | "structural",
        max_results?: int,
    ) -> SearchResults {
        hits: [ {
            node_id, name, path, line, snippet,
            confidence: 0..1,
            score_fts, score_semantic, score_structural,
            match_mode: "fts" | "semantic" | "structural" | "hybrid"
        } ],
        overall_confidence: "high" | "medium" | "low" | "none",
        fallback_hint?: string          // when overall_confidence = none
    }

- `mode: "auto"` (default) runs all three and merges — this is
  the recommended path.
- `overall_confidence` collapses the individual scores:
  `high` = ≥ 1 hit with combined score ≥ 0.8;
  `medium` = hits with 0.5–0.8; `low` = hits below;
  `none` = no hits or all below noise threshold.
- On `none`, the response includes a `fallback_hint` (e.g. the
  grep command the caller should run) so the assistant doesn't
  hallucinate a match.

### Grep fallback

When `overall_confidence == none`, the caller (or the pre-tool-
use hook) reruns as a plain grep — sensei never blocks a search
just because its own index is thin.

### Hook-based routing (optional)

The Claude Code plugin's `pre_tool_use` hook can intercept `Bash
grep` calls and route through sensei's `search` first. The hook
returns whichever gives higher confidence — sensei's result when
it wins, plain grep output when sensei has nothing.

Config in Preferences → Assistants → Claude Code → Hooks:
`route_grep_through_sensei: true`.

## Signals produced

| Signal | Consumer |
|---|---|
| `search` results | MCP callers (assistants + Playground) |
| Search hit / miss telemetry | Insights ("search saved N grep calls this week") |
| Missing-embedding warnings | Analyzer signal (a project with < 60% node coverage should be re-analyzed) |
| Confidence trend | Insights (raising or falling accuracy over time) |

## Done gate

- `search(query="how do we handle auth refresh?", project=…)`
  returns high-confidence hits for the right code path — no
  substring match required.
- Every node with a `body` or `doc` field has an entry in
  `node_embeddings`.
- Latency for a warm hybrid query is ≤ 200ms.
- Grep fallback works when confidence is `none`.
- The Claude Code plugin's hook can be enabled from
  Preferences and observably routes Bash grep through sensei.
- Insights shows a running "search hits vs greps" chip.

Optional check:
```
mcp_call search --query="auth refresh" --project=sensei --max_results=5 \
  | jq '{overall: .overall_confidence, hits: [.hits[] | {name, confidence, mode: .match_mode}]}'

# What's index coverage?
psql -A -t -c "select
    (select count(*) from sensei.node_embeddings) * 100.0 /
    (select count(*) from sensei.nodes) as pct_embedded" -d sensei
```

## Wrong gate

- **`search` returns 0 hits on a query the codebase clearly
  answers.** Embeddings missing OR ranking broken.
- **`overall_confidence == high` when hits are actually
  irrelevant.** Threshold too loose OR the semantic model is
  hallucinating similarity.
- **Sensei blocks a search that would succeed as grep.** Grep
  fallback path skipped.
- **Every query returns the same top hit.** Score merge biased.
- **Re-analyzed project doesn't re-embed changed nodes.**
  Watcher didn't invalidate.
- **Hook interception rewrites the assistant's Bash command
  without telling it.** Should surface the swap so the assistant
  can course-correct if the sensei result is nonsense.

## Related

- [[pipeline/capture]] — node ingestion; watcher invalidates
- [[pipeline/inferencing]] — embedding chain
- [[pipeline/context-delivery]] — search results feed context
  packing (fast lookup for semantic fallback branch)
- [[pipeline/analyzer]] — re-embeds changed nodes on tick
- [[pipeline/mcp-surface]] — `search` tool declaration
- (archive: ideas/31-semantic-search-layer.md) — source design
