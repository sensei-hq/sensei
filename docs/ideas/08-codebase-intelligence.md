---
name: Codebase Intelligence
description: Index repositories — symbols, call graphs, dependencies — to power context delivery, code exploration, and quality analysis
date: 2026-04-17
status: idea
sources: features/01-codebase-intelligence.md, roadmap/06-graph-intelligence.md, gap-analysis.md
---

# Codebase Intelligence

## Problem

The AI needs to understand codebases deeply — not just grep for strings, but know the call graph, dependency structure, and architectural patterns. The Rust indexer handles parsing and symbol extraction, but gaps remain in call graph accuracy, incremental indexing, and advanced graph analysis.

## Current state

- Language-agnostic AST parsing: implemented (TS/JS via OXC, Python/Rust/Java via tree-sitter, SQL via sqlparser)
- Symbol extraction: implemented
- Call graph: partial — leaf function calls silently dropped (gap-analysis C2)
- Incremental re-indexing: partial — file-level change detection works, but doc files not in full indexer pass
- Stack detection: implemented
- Task queue: implemented (scan → repo → folder → file → resolve → connect, 233 tests)
- Graph DB: SQLite-based (Kuzu migration planned when linking stabilizes)

## What this idea covers

- **Call graph accuracy**: fix dropped edges for leaf functions, validate edge completeness
- **Incremental indexing reliability**: ensure all file types (code + docs) go through incremental pipeline
- **Advanced graph analysis**: community detection (Leiden clustering), "god node" hotspot detection, confidence tagging on edges
- **Pattern auto-detection**: identify recurring structures during indexing and surface them as candidate patterns
- **Cross-repo indexing**: resolve references across repos via OpenAPI, GraphQL, gRPC, async event schemas
- **Rationale extraction**: extract design rationale from code comments during indexing

## Open questions

- When does Kuzu become viable? What's blocking the migration from SQLite graph tables?
- Should pattern detection happen during indexing (automatic) or on-demand (user triggers)?
- How do cross-repo references work in practice? Workspace concept needed first?
