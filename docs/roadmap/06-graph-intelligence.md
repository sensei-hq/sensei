# Graph Intelligence — Analysis of Graphify and the Karpathy Knowledge Base Model

> Reference: [safishamsi/graphify](https://github.com/safishamsi/graphify) (10.6k stars, April 2026)
> Inspired by: [Andrej Karpathy's LLM Knowledge Bases](https://x.com/karpathy/status/2039805659525644595)
> and [llm-wiki gist](https://gist.github.com/karpathy/442a6bf555914893e9891c11519de94f)

---

## What Karpathy Proposed

The idea that spawned graphify (and several other implementations) is a three-layer architecture
for personal knowledge management with LLMs:

```
Raw Sources  →  LLM-compiled Wiki  →  Query / Lint operations
(immutable)     (persistent, compounding artifact)
```

The key insight: **don't use RAG to rediscover information on every query — compile it
once into a structured wiki that grows richer with each source added.**

The LLM handles the bookkeeping (cross-references, consistency, backlinks). Humans handle
curation and strategic questioning. The wiki becomes a "persistent, compounding artifact"
rather than a retrieval index.

Karpathy keeps a `/raw` folder with papers, tweets, screenshots, notes.
The LLM compiles them into a wiki: summaries, concept pages, entity pages, activity log.

---

## What Graphify Built

Graphify implements this pattern specifically for code + mixed media corpora:

**Pipeline:**
```
detect() → extract() → build_graph() → cluster() → analyze() → report() → export()
```

**Two-pass extraction:**
1. AST pass (deterministic, no LLM) — code structure, call graph, imports, docstrings
2. LLM pass (parallel subagents) — docs, PDFs, images, design rationale

**Graph structure (NetworkX + Leiden):**
- Nodes: concepts, classes, functions, symbols, paper concepts, image entities
- Edges typed as `EXTRACTED` (certain), `INFERRED` (with confidence score), or `AMBIGUOUS`
- Hyperedges: 3+ node relationships (all functions in an auth flow, all classes implementing a protocol)
- Community detection: **topology-based (Leiden algorithm)** — no embeddings, graph edge density drives clustering

**Key outputs:**
- `graph.json` — persistent, queryable across sessions without re-reading files
- `GRAPH_REPORT.md` — god nodes, surprising connections (cross-type edges rank highest), suggested questions
- Interactive HTML, Obsidian vault, wiki (index.md + article per community), Neo4j export
- `--mcp` mode: starts an MCP stdio server over the graph

**Claimed performance:** 71.5x fewer tokens per query vs reading raw files on a 52-file mixed corpus.

---

## Sensei vs Graphify — Honest Comparison

These are not competing tools. They have different primary jobs:

| | Graphify | Sensei |
|---|---|---|
| **Primary job** | Build the knowledge graph | Navigate the graph to assist AI sessions |
| **Input** | Any folder — code, docs, PDFs, images, papers | Primarily code + library docs |
| **Graph structure** | NetworkX, Leiden clustering, hyperedges | Call graph + symbol map in SQLite |
| **LLM use** | LLM compiles the graph (extraction phase) | LLM uses the graph (session phase) |
| **Session continuity** | None | Core feature (FTR, snapshots, recovery) |
| **Token budgeting** | Graph compression (71.5x reduction) | L0-L3 resolution + ranked context packs |
| **Phase/workspace model** | None | Core feature (cards, phases, maturity) |
| **Coordinator integration** | Skill file per coordinator | Full CoordinatorAdapter (MCP + hooks + events) |
| **Confidence tagging** | EXTRACTED / INFERRED / AMBIGUOUS | Not implemented |
| **Community detection** | Leiden algorithm | Not implemented |
| **God node analysis** | Yes (highest-degree nodes) | Not implemented |
| **"Why" extraction** | `rationale_for` nodes from comments | Not implemented |
| **Multimodal** | PDFs, images, diagrams (Claude vision) | Not implemented |
| **Cross-type edges** | Code ↔ paper ↔ image ↔ doc | Code ↔ library doc only |

---

## What Sensei Should Adopt

These are not wholesale rewrites — they are additions to the existing graph that use
data already being collected. Ordered by value vs effort:

### 1. Confidence tagging on edges (Low effort, High value)

Graphify's `EXTRACTED` / `INFERRED` / `AMBIGUOUS` labeling is simple and immediately
useful. Sensei already produces call edges (EXTRACTED) and semantic similarity scores
(INFERRED). The tag just needs to be written.

Applied to sensei:
- `EXTRACTED`: direct imports, explicit function calls, documented library usage
- `INFERRED`: embedding similarity above threshold, BM25 co-occurrence, pattern matching
- `AMBIGUOUS`: relationships flagged during drift detection or gap analysis

This makes the graph honest. Agents navigating the graph know which edges to trust fully
and which to treat as signals.

---

### 2. "Why" extraction from comments (Low effort, High value)

Code tells you what. Comments tell you why. The why is often the most important thing
for an agent trying to understand an architectural decision.

Graphify extracts `rationale_for` nodes from inline comments tagged:
`# NOTE:`, `# IMPORTANT:`, `# HACK:`, `# WHY:`, `# TODO:`, `# REASON:`

Sensei should do the same during the parse stage. These become a new node type:
`Rationale` — linked to the symbol they annotate with a `rationale_for` edge.

When an agent asks "why is auth structured this way?", the answer is often in a 3-line
comment that the agent would never find by reading symbol signatures.

---

### 3. God nodes — surface high-degree concepts (Medium effort, High value)

The highest-degree nodes in the graph are the "load-bearing" concepts — everything
connects through them. Knowing these is the fastest way to orient in an unfamiliar codebase.

Sensei already has the graph in SQLite. Calculating degree is a single query:
```sql
SELECT symbol_id, COUNT(*) as degree FROM call_edges GROUP BY symbol_id ORDER BY degree DESC LIMIT 20
```

Surface these in:
- The `/analyze-repo` command output
- The workspace project view (a "core concepts" panel)
- `get_session_context` response (top 5 god nodes as orientation)

---

### 4. Leiden community detection (Medium effort, High value)

Graphify uses Leiden (a graph topology algorithm) to cluster nodes into communities
without embeddings. Communities correspond to modules, subsystems, or cross-cutting concerns.

Sensei can do this in TypeScript using the existing SQLite graph. A pure-JS Leiden
implementation exists (`graphology-communities-leiden`). The communities become a new
attribute on symbols — useful for:
- `/phase-summary` grouping
- Context pack assembly (load whole community vs. individual symbols)
- The workspace graph visualisation (colour by community)
- Identifying "bridge nodes" that connect communities — often the most architecturally important

---

### 5. Graph narrative report (Medium effort, High value)

Graphify's `GRAPH_REPORT.md` is the output developers actually read. It answers:
- What are the god nodes?
- What are the surprising connections (cross-type edges ranked by surprise)?
- What questions is this graph uniquely positioned to answer?

Sensei should generate an equivalent during `/analyze-repo`. This becomes the
orientation document for a new developer or a new AI session — richer than the
current `get_session_context` response.

---

### 6. Karpathy's wiki model maps to the card system (Design alignment)

This is not a technical change — it is a conceptual alignment that confirms the
card/phase direction is correct.

Karpathy's three layers map directly:

| Karpathy | Sensei |
|---|---|
| `/raw` folder (immutable sources) | Indexed code + library docs + attached files |
| LLM-maintained wiki (compounding artifact) | Cards system — requirements, analysis, decisions |
| Schema / CLAUDE.md | llmspec.yaml + CoordinatorAdapter context |
| Ingest operation | `/analyze-repo` command — generates Analysis cards from the codebase |
| Query operation | Prompt bar with citations |
| Lint operation | `/gap-analysis`, `/find-orphans`, `/design-review` |

The card system is sensei's version of the wiki. Cards should compound — each ingest
(running `/analyze-repo` on a new version) updates existing cards rather than duplicating
them. This is an important detail to get right in Phase 3.

---

### 7. Hyperedges (Low-medium effort, Medium value)

Some relationships genuinely involve 3+ nodes simultaneously:
- All functions in an auth flow
- All implementations of an interface
- All files changed together in a migration
- All requirements addressed by a single session

Pairwise edges miss the group nature of these relationships. A `hyperedge` table in
SQLite (many-to-many via a junction table) handles this. Surface in:
- The graph visualisation (dashed boundary around a group)
- `/trace` output (show the full flow, not just pairwise hops)

---

## What Sensei Should NOT Adopt

### Full multimodal extraction (PDFs, images)

Graphify's PDF and image extraction is genuinely good. It is also graphify's specialty.
Sensei should link to graphify as an optional upstream step rather than reimplementing
this. If a developer runs graphify on their `docs/` folder and produces `graph.json`,
sensei could **import** that graph as a set of additional nodes and edges rather than
doing the extraction itself.

This is the integration story: **graphify builds the knowledge graph from raw materials,
sensei imports it and uses it for session assistance.**

### Replacing embeddings with topology-only clustering

Graphify deliberately avoids a separate vector store. Sensei already has SQLite-vec.
The semantic similarity edges that come from embeddings are valuable signal — don't
remove them in favour of topology-only clustering. Use both: embeddings for semantic
search, Leiden on the full graph (which includes semantic edges) for community detection.

### Python/NetworkX rewrite

Graphify is Python + NetworkX. Sensei is TypeScript. The right response is:
- Import graphify's `graph.json` output natively (it is plain JSON)
- Use `graphology` (TypeScript/JavaScript NetworkX equivalent) for graph operations
- Use `graphology-communities-leiden` for community detection

No Python dependency required.

---

## Integration Architecture

Two modes of coexistence:

### Mode A: Graphify as upstream input (Recommended for rich corpora)

```
Graphify runs on raw/ (code + papers + images + docs)
     ↓ produces graph.json
Sensei imports graph.json nodes and edges into SQLite
     ↓ merges with its own symbol graph
Sensei serves the unified graph via MCP
```

Command: `sensei import-graph ./graphify-out/graph.json`

This gives sensei cross-type edges (code ↔ paper ↔ image) without reimplementing
multimodal extraction.

### Mode B: Sensei implements graphify concepts natively (For code-only projects)

Sensei's own engine implements:
- Confidence tagging (Phase 0 — trivial)
- "Why" extraction from comments (Phase 0 — parser addition)
- God node detection (Phase 2 — single SQL query)
- GRAPH_REPORT.md generation (Phase 4 — `/analyze-repo` output)
- Leiden community detection (Phase 3 — graphology library)
- Hyperedges (Phase 3 — new SQLite table)

Both modes can coexist. Mode A adds value when the developer has a mixed corpus.
Mode B is the always-on foundation.

---

## Recommended Next Steps (for the roadmap)

Add to Phase 0:
- Confidence tagging (`EXTRACTED` / `INFERRED` / `AMBIGUOUS`) on all graph edges
- "Why" extraction from annotated comments during the parse stage

Add to Phase 2 (workspace model):
- God node detection surfaced in project view and `get_session_context`

Add to Phase 3 (card system):
- Leiden community detection on the symbol graph
- Hyperedge table in SQLite
- Graph narrative report as `/analyze-repo` output
- Card system treated as the "compounding wiki" — updates existing cards on re-index

Add to Phase 5 (local inference):
- `sensei import-graph <path>` command to ingest graphify's graph.json output

---

## Summary Verdict

Graphify validates the graph direction. Its core innovations — confidence tagging,
god nodes, community detection, the graph narrative report, and the "why" extraction —
are all things sensei should implement. Several require minimal engineering because the
underlying graph data already exists in sensei's SQLite store.

The Karpathy model — raw materials → compounding wiki → query/lint — maps directly onto
sensei's phase system. The insight about LLMs being excellent at bookkeeping (cross-references,
consistency, backlinks) is exactly what the prompt-first card system should leverage.

Graphify also confirms the coordinator-agnostic design: it already ships skill files for
Claude Code, Codex, OpenCode, OpenClaw, and Factory Droid. The coordinator adapter
pattern is validated by a real, widely-used tool.
