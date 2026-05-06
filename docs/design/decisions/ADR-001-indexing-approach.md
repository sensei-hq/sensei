# ADR-001: Graph DB + AST Parsing for Indexing

**Status:** Accepted
**Date:** 2026-04-09
**Supersedes:** `05-indexing.md` (regex-based extraction), flat vector search approach

---

## Context

Sensei needs to index codebases and serve that index to AI agents at query time. The index must support:
- Symbol lookup by name, file, and type
- Caller/callee traversal (who calls this function? what does it call?)
- Import/type dependency traversal
- Doc-to-code traceability (does this doc claim match any implementation?)
- Token-efficient context loading (agents have limited context windows)

The original design used regex-based symbol extraction writing to `.index/symbol-map.json`. We then evaluated several approaches to improve semantic quality.

---

## Approaches Tried and Abandoned

### 1. Regex-based symbol extraction (original approach)

Pattern-matched exports per language. Fast, zero dependencies. The output was a flat list of symbol signatures per file.

**Why abandoned:** Regex can extract names and signatures but cannot represent relationships. There is no way to express "function A calls function B" or "module X imports from module Y" in a flat map. The structure that makes a codebase navigable — the call graph, the import graph, the type dependency graph — is invisible to regex.

---

### 2. cocoindex (tried, abandoned)

[cocoindex](https://github.com/cocoindex-io/cocoindex) is a Python-based pipeline for building flat vector indexes over code. We evaluated it as a drop-in replacement for the regex extractor.

**Assessment:**
- Setup works. Indexing runs. Queries return results.
- Results were poor in practice. Semantic similarity over code embeddings surfaces files that mention similar words, not files that are structurally connected.
- The fundamental problem: flat vector search loses graph structure. If you ask "what calls `processPayment`?", cosine similarity over embeddings gives you files that talk about payments — not the actual callers. The call graph is the answer; vector search cannot represent it.
- Embeddings are useful as one signal (semantic search at L4 in our depth model), but they cannot be the primary index structure for code navigation.

**Decision:** Tried and abandoned. cocoindex is not in the codebase.

---

### 3. MemPalace (evaluated, rejected)

MemPalace is a graph-augmented memory system built on ChromaDB. It reports 96.6% recall on knowledge retrieval benchmarks.

**Assessment:**
- Recall numbers are real and impressive for document-style retrieval.
- ChromaDB is a heavy dependency — server process, complex install, Docker recommended.
- The "memory palace" spatial metaphor is designed for personal knowledge management, not code graph navigation.
- Install issues encountered during evaluation.

**Decision:** Rejected. The ChromaDB dependency makes it unsuitable for a desktop app that must bundle as a Tauri sidecar with zero external server requirements.

---

### 4. FalkorDB (evaluated, rejected for desktop)

FalkorDB is a graph database with a Redis-based runtime, Cypher query language, and good TypeScript bindings. It is genuinely capable — the query model and schema design that works for FalkorDB translates directly to Kuzu.

**Assessment:**
- Excellent graph query capabilities via Cypher.
- The runtime dependency is Redis (or FalkorDB's fork of it). This means a server process that must be started, managed, and kept running.
- For a team server deployment, this is acceptable. For a desktop app that must work out of the box with zero configuration, a Redis dependency is a blocker.

**Decision:** Rejected for desktop. Worth reconsidering for a future team server product. The schema design from this evaluation was carried forward to the Kuzu implementation.

---

## Decision: Kuzu + tree-sitter

### Why graph DB (general)

Code is already a graph. The call graph, import graph, and type dependency graph are the primary structures that matter for code navigation. A flat index (vector or otherwise) destroys this structure and then tries to reconstruct it with approximate similarity — a lossy process.

A graph database stores the structure directly. Queries like "give me all callers of `processPayment` up to 2 hops" are first-class graph traversals, not approximations.

The graph also enables drift detection that is structurally meaningful: a `DOCUMENTS` edge with no corresponding `IMPLEMENTS` edge is a verifiable claim with no supporting evidence — not a similarity score threshold.

### Why Kuzu specifically

[Kuzu](https://kuzudb.com/) is an embedded graph database:
- **No server process.** Like SQLite for relational data, Kuzu is a library that opens a file. Zero daemon, zero Docker, zero configuration.
- **Embeddable in Tauri.** Bundles as a sidecar binary. Ships with the desktop app.
- **Apache 2.0 license.** No commercial licensing issues.
- **TypeScript native bindings.** First-class Node/Bun support.
- **Cypher query language.** The same language used by Neo4j, FalkorDB, and most graph DB tooling. Query knowledge transfers.
- **Single file per project.** Stored at `~/.sensei/graph/<project>.kuzu`. Portable, backupable, deleteable.

---

## Graph Schema

**Nodes:**

| Label | Key fields |
|-------|-----------|
| `Function` | name, file, signature, docstring |
| `File` | path, language |
| `Type` | name, file, kind (class/interface/enum) |
| `Doc` | path, section, content |
| `Comment` | text, file, line, kind (NOTE/WHY/HACK/TODO) |

**Edges:**

| Label | Meaning |
|-------|---------|
| `CALLS` | Function → Function (direct call) |
| `IMPORTS` | File → File (import statement) |
| `EXPORTS` | File → Function/Type |
| `USES_TYPE` | Function/Type → Type |
| `DOCUMENTS` | Doc → Function/Type (doc claims to describe symbol) |
| `ANNOTATES` | Comment → Function/Type |
| `IMPLEMENTS` | Function/Type → Doc (symbol implements a documented spec) |

**Drift detection:** A `DOCUMENTS` edge with no corresponding `IMPLEMENTS` counter-edge is an unverified doc claim. The `check_drift` MCP tool surfaces these.

---

## The L0–L5 Depth Model

The key insight for token efficiency: not every query needs the same amount of context. A symbol lookup needs a name and file path. A refactoring task needs callers, callees, and rationale comments. Serving everything at full depth on every query is wasteful.

The depth model defines six resolution levels:

| Level | Content | Approx tokens |
|-------|---------|---------------|
| L0 | name + kind + file | ~10 |
| L1 | + signature + docstring | ~50 |
| L2 | + direct callers/callees | ~200 |
| L3 | + cross-file neighbors + doc edges | ~500 |
| L4 | + semantic search hits | ~1K |
| L5 | + rationale comments + git ref | ~3K |

MCP tools accept a `depth` parameter. The agent requests the depth it needs. Default is L1 for broad searches, L3 for focused navigation.

This replaces the earlier L0–L3 model in `06-compression.md`. L4 and L5 are new; L4 adds the semantic search signal (embeddings are still useful, just not primary), L5 adds rationale comments extracted from `# WHY:`, `# NOTE:`, `# HACK:` annotations.

---

## MCP Tool Surface

| Tool | What it does |
|------|-------------|
| `get_symbol` | Look up a symbol by name or file, returns L0–L5 |
| `get_callers` | Return all callers of a function (graph traversal) |
| `search_code` | Semantic search (L4 — embedding-based, secondary) |
| `check_drift` | Find DOCUMENTS edges with no IMPLEMENTS counter-edge |
| `get_community` | Return the Leiden community a symbol belongs to |

---

## Parser: tree-sitter (replaces regex)

Symbol extraction now uses [tree-sitter](https://tree-sitter.github.io/) via its Node bindings. Tree-sitter produces a full AST with precise node types and source ranges.

**Why this matters over regex:**
- Regex cannot reliably distinguish a function declaration from a function call in all contexts.
- Tree-sitter handles nested structures, multiline signatures, destructuring, generic types.
- Call graph extraction requires understanding what is inside a function body — regex cannot do this without becoming a partial parser anyway.
- tree-sitter grammars exist for TypeScript, JavaScript, Python, Go, Rust, and most other target languages.

---

## Storage Location

```
~/.sensei/graph/<project-slug>.kuzu    ← Kuzu graph DB per project
~/.sensei/graph/<project-slug>.vec     ← SQLite-vec for L4 semantic search
```

Separating the graph store from the vector store keeps each layer independent. The graph handles structural queries; the vector index handles semantic similarity. Both are embedded, no server required.

---

## Consequences

**Positive:**
- Call graph and import graph queries are exact, not approximate.
- Drift detection is structurally grounded (missing IMPLEMENTS edges), not threshold-based.
- The desktop app ships with zero external dependencies for the index.
- Depth model makes token usage predictable and controllable.

**Negative:**
- Kuzu is less mature than PostgreSQL/Supabase. API surface is smaller.
- tree-sitter grammar files add binary dependencies per language.
- The graph schema must be defined upfront; schema changes require re-indexing.

**Neutral:**
- Supabase/pgvector is removed from the indexing path. It may remain for other purposes (auth, project metadata) in a team server product.
