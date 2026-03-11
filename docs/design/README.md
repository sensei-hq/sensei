# Design Documentation

Architecture, structure, and implementation details for the AI skills repo.

## Core Design (01–09)

| Document | Description |
|----------|-------------|
| [01-architecture](./01-architecture.md) | Overall system architecture, repo structure, component relationships |
| [02-skills](./02-skills.md) | Skill file format, naming conventions, CSO, testing requirements |
| [03-mcp-server](./03-mcp-server.md) | MCP server: tool categories, APIs, tool contracts (Claude-facing) |
| [04-llmspec](./04-llmspec.md) | LLMSpec format (.llmspec.yaml), fields, generation, querying |
| [05-indexing](./05-indexing.md) | Indexer design, extraction targets, storage format |
| [06-compression](./06-compression.md) | Resolution levels (L0–L3), storage schema, serving logic |
| [07-drift](./07-drift.md) | Git-diff + traceability-based drift detection, hook integration |
| [08-benchmarking](./08-benchmarking.md) | Benchmark architecture, task corpus schema, metrics, A/B setup |
| [09-cli](./09-cli.md) | CLI design, layered profile system, command modules, config schemas |

## Feature Extensions (10–19)

| Document | Description |
|----------|-------------|
| [10-project-memory](./10-project-memory.md) | Cross-session knowledge: checkpoint distillation, decisions, open items |
| [11-doc-tools](./11-doc-tools.md) | Doc guide skill, find_doc fallback, sensei doc new scaffold, doc-doctor, external doc refs |
| [12-incremental-indexing](./12-incremental-indexing.md) | Git-diff change detection, force flag, incremental update algorithm |
| [13-traceability-matrix](./13-traceability-matrix.md) | Doc-to-code traceability: schema, population, drift cross-reference |
| [14-server-package](./14-server-package.md) | Inference server: package split, server API, deployment models (local/org/cloud) |
| [15-package-adapters](./15-package-adapters.md) | Glob-based package discovery, folder-map, README link extraction |
| [16-local-model-indexer](./16-local-model-indexer.md) | Local model inference: ModelBackend interface, FileAnalysis schema, analysis cache |
| [17-pattern-store](./17-pattern-store.md) | Pattern detection, capture, search, and skill export; .sensei/patterns.md storage |
| [18-response-cache](./18-response-cache.md) | Cross-session response cache: TTL management, semantic retrieval, session hints |
| [19-context-manager](./19-context-manager.md) | Targeted slice loading, token budget, checkpoints, recommend_next prescription |

## Language-Specific Adapters (20–29)

_Planned — not yet written._

| Document | Description |
|----------|-------------|
| 20-adapter-js-ts | JS/TS adapter: package.json, TypeScript signatures, React/hooks detection |
| 21-adapter-python | Python adapter: pyproject.toml, FastAPI/Django patterns |
| 22-adapter-go | Go adapter: go.mod, exported function conventions |
| 23-adapter-rust | Rust adapter: Cargo.toml, pub fn/struct/enum |

## Numbering Convention

| Range | Category |
|-------|----------|
| 01–09 | Core design — architecture, skills, MCP tools, llmspec, indexing, compression, drift, benchmarking, CLI |
| 10–19 | Feature extensions — project memory, doc tools, incremental indexing, traceability, server package, package adapters, local model, pattern store, response cache, context manager |
| 20–29 | Language-specific adapters |
| 30–39 | Reserved |

## Document Relationships

```
01-architecture ──────────────────── overall system
  └── 14-server-package            package split: CLI / server / MCP
        ├── 16-local-model-indexer  inference engine: ModelBackend, FileAnalysis
        └── 15-package-adapters    folder map: glob discovery, README links

05-indexing ──────────────────────── symbol map (current regex approach)
  ├── 12-incremental-indexing      git-diff change detection
  └── 16-local-model-indexer       replaces regex with local model

13-traceability-matrix ───────────── doc→code coverage (manual @covers)
  ├── 15-package-adapters          extends with README link graph
  └── 16-local-model-indexer       extends with embedding similarity

03-mcp-server ────────────────────── MCP tool contracts (Claude-facing)
  └── 14-server-package            inference server is separate from MCP server
```

## Traceability

Feature → design → code coverage is tracked in [`docs/traceability.yaml`](../traceability.yaml).

Query patterns:
- Features with no design: feature ids not referenced in any `design[].implements[].feature`
- Designs with no code: design ids not in any `code[].implements-design`
- Backlog: all `items` with `status: planned`

## Related

- [Features](../features/) — What and why (needs, scenarios, status)
- [Traceability](../traceability.yaml) — Feature/design/code coverage matrix
- [Plans](../plans/) — Implementation plans
