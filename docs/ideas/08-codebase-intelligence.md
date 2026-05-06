---
name: Codebase Intelligence
description: Index repositories — symbols, call graphs, dependencies, design patterns, duplicates — to power context delivery, code exploration, and quality analysis
date: 2026-04-17
status: idea
sources: features/01-codebase-intelligence.md, roadmap/06-graph-intelligence.md, gap-analysis.md
dependency-of: blueprints/01-workflow-engine.md (locate step needs get_patterns)
---

# Codebase Intelligence

## Problem

The AI needs to understand codebases deeply — not just grep for strings, but know the call graph, dependency structure, design patterns, and code quality. The Rust indexer handles parsing and symbol extraction, but gaps remain in call graph accuracy, pattern recognition, duplicate detection, and advanced graph analysis.

## Current state

- Language-agnostic AST parsing: implemented (TS/JS via OXC, Python/Rust/Java via tree-sitter, SQL via sqlparser)
- Symbol extraction: implemented
- Call graph: partial — leaf function calls silently dropped (gap-analysis C2)
- Incremental re-indexing: partial — file-level change detection works, but doc files not in full indexer pass
- Stack detection: implemented
- Task queue: implemented (scan → repo → folder → file → resolve → connect, 233 tests)
- Graph DB: PostgreSQL (sensei.nodes + sensei.edges)
- **Design pattern recognition: not implemented**
- **Duplicate/similarity detection: not implemented**

## What this idea covers

### Design pattern recognition

The indexer parses ASTs but doesn't recognize structural design patterns (adapter, factory, observer, builder, strategy, etc.). The workflow engine's locate step (`get_patterns()` MCP tool) depends on this.

**Phased approach:**

| Phase | Approach | What it detects | Effort |
|-------|----------|-----------------|--------|
| **Phase A** | Naming heuristics | `*Adapter`, `*Factory`, `*Observer`, `*Builder`, `*Strategy`, `*Middleware`, `*Hook`, `*Handler` | Low — name matching on class/struct/trait definitions. AST already parsed. |
| **Phase B** | Structural heuristics | "Class implements interface + wraps another = adapter." "Static method returns new instance = factory." Analyze class hierarchies and composition. | Medium — AST analysis already available, need additional passes. |
| **Phase C** | Semgrep integration | Custom rules for each pattern type. Matches AST shapes across languages. Post-indexing pass. | Medium — external dependency but high accuracy. |
| **Phase D** | LLM-assisted (Gemma4 via Ollama) | Novel patterns, complex compositions, domain-specific patterns. | High — on-demand analysis, not every index run. |

Phase A is sufficient to unblock the workflow engine's locate step. Phases B-D are enhancements.

**Graph representation:** Detected patterns stored as tagged nodes in the graph:
```
Node: SqlAdapter  kind=class  patterns=["adapter"]  wraps=["SqlParser"]
Node: TaskFactory  kind=function  patterns=["factory"]  creates=["ScanTask", "IndexTask"]
```

### Duplicate and similarity detection

Identify duplicated code, similar structures, and copy-paste patterns. Critical for `/sensei:review` quality checks.

**Approach:**

| Phase | Tool | What it detects | Integration |
|-------|------|-----------------|-------------|
| **Phase A** | qlty CLI or jscpd | Exact and near-exact duplicates | Run during indexing, store results in graph. External dependency. |
| **Phase B** | Semgrep rules | Structural similarity (not just text) | Same integration as pattern detection Phase C. |
| **Phase C** | Custom AST hashing | Normalized AST subtree comparison | Built into Rust indexer. No external dependency. |

Phase A gives immediate value with minimal effort. qlty or jscpd produce JSON output that the indexer can ingest and store as graph edges (`DUPLICATES` relationship between file nodes).

### Other codebase intelligence improvements

- **Call graph accuracy**: fix dropped edges for leaf functions, validate edge completeness
- **Incremental indexing reliability**: ensure all file types (code + docs) go through incremental pipeline
- **Advanced graph analysis**: community detection (Leiden clustering), "god node" hotspot detection, confidence tagging on edges
- **Cross-repo indexing**: resolve references across repos via OpenAPI, GraphQL, gRPC, async event schemas
- **Rationale extraction**: extract design rationale from code comments during indexing

## Impact on MCP tools

Pattern detection and duplicate detection would enhance existing MCP tools and enable new ones:

| MCP tool | Current | With pattern detection |
|----------|---------|----------------------|
| `get_patterns(pattern)` | Matches file names only | Matches structural patterns (adapter, factory, etc.) |
| `search(query)` | Finds by symbol name/signature | Also finds by pattern type ("find all adapters") |
| (new) `get_duplicates(file?)` | N/A | Returns duplicate clusters for a file or project |
| (new) `get_similar(symbol)` | N/A | Returns structurally similar functions/classes |

## Open questions

- Should pattern detection run on every index, or only on demand/first-index?
- qlty vs jscpd vs semgrep for Phase A duplicate detection — which has best JSON output for ingestion?
- How do cross-repo references work in practice? Workspace concept needed first?
- Should semgrep be a required dependency or optional (graceful degradation)?
