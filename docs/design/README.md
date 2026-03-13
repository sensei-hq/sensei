# Design Documentation

Architecture, implementation details, and recorded decisions for the sensei project.

> **Convention**: design docs answer *how* and *why*. Feature docs answer *what*. Every significant technical decision should record the reasoning here so future contributors understand the intent, not just the outcome.

## How to Use This Index

| You want to... | Start here |
|---|---|
| Understand the overall system | [01-architecture](./01-architecture.md) |
| See the implementation roadmap | [30-implementation-phases](./30-implementation-phases.md) |
| Understand the pipeline in depth | [20-pipeline-adapter](./20-pipeline-adapter.md) |
| See the CC-RLM analysis that informed the pipeline | [cc-rlm](./cc-rlm.md) |
| See the full future architecture with Mermaid flows | [architecture](./architecture.md) |

---

## Core Design (01–09)

Authoritative design documents. When these conflict with older documents (10–19), these take precedence.

| Document | Description | Status |
|----------|-------------|--------|
| [01-architecture](./01-architecture.md) | Package structure, interfaces, data flows, architectural decisions (ADRs), technology choices | Current |
| [02-skills](./02-skills.md) | Skill file format, naming conventions, testing requirements | Archived — see superpowers plugin system |
| [03-mcp-server](./03-mcp-server.md) | MCP server tool contracts (Claude-facing) | Superseded by `40-mcp-tool-contracts.md` |
| [04-llmspec](./04-llmspec.md) | LLMSpec format — superseded by three-layer metadata model | Superseded by `40-metadata-model.md` |
| [05-indexing](./05-indexing.md) | Indexer design (regex-based, v1) — superseded by pipeline adapter | Superseded by `20-pipeline-adapter.md` + `40-metadata-model.md` |
| [06-compression](./06-compression.md) | Resolution levels (L0–L3), storage schema, serving logic | Current — L0–L3 still used by load_context |
| [07-drift](./07-drift.md) | Git-diff + traceability-based drift detection, hook integration | Current |
| [08-benchmarking](./08-benchmarking.md) | Benchmark architecture, task corpus schema, metrics, A/B setup | Current |
| [09-cli](./09-cli.md) | CLI design, command modules, config schemas | Partially superseded — see 01-architecture for updated package structure |

---

## Feature Extensions (10–19)

Earlier design iterations. Superseded sections are noted. The pipeline adapter (20) and architecture (01) take precedence where there is conflict.

| Document | Description | Status |
|----------|-------------|--------|
| [10-project-memory](./10-project-memory.md) | Cross-session knowledge: checkpoint distillation, decisions, open items | Current — now stored in Supabase |
| [11-doc-tools](./11-doc-tools.md) | Doc guide skill, find_doc, doc new scaffold, doc-doctor | Current |
| [12-incremental-indexing](./12-incremental-indexing.md) | Git-diff change detection, incremental update algorithm | Current |
| [13-traceability-matrix](./13-traceability-matrix.md) | Doc-to-code traceability: schema, population, drift cross-reference | Current — extended by Layer 3 cross-repo traceability |
| [14-server-package](./14-server-package.md) | Package split, server API, deployment models | Superseded by `01-architecture.md` |
| [15-package-adapters](./15-package-adapters.md) | Glob-based package discovery, folder-map | Superseded by `40-metadata-model.md` |
| [16-local-model-indexer](./16-local-model-indexer.md) | ModelBackend interface, local model inference | Current — ModelBackend is in packages/shared |
| [17-pattern-store](./17-pattern-store.md) | Pattern detection, capture, search, skill export | Current |
| [18-response-cache](./18-response-cache.md) | Cross-session response cache, TTL, semantic retrieval | Current |
| [19-context-manager](./19-context-manager.md) | Targeted slice loading, token budget, recommend_next | Current — extended by 20-pipeline-adapter Rank/Slice/Assemble |

---

## Reference & Analysis

| Document | Description |
|----------|-------------|
| [cc-rlm](./cc-rlm.md) | CC-RLM architecture analysis: flow, token reduction mechanics, what sensei borrows vs improves |
| [architecture](./architecture.md) | Three-layer feature architecture with Mermaid component maps and data flow diagrams |

---

## Pipeline & Adapters (20–29)

| Document | Description | Status |
|----------|-------------|--------|
| [20-pipeline-adapter](./20-pipeline-adapter.md) | Pipeline stages, LanguageAdapterRegistry, ranking strategy chain, Supabase schema, agent adapter pattern, all resolved design decisions | Current |
| 21-adapter-js-ts | JS/TS adapter: TypeScript signatures, React/hooks detection | Planned |
| 22-adapter-python | Python adapter: pyproject.toml, FastAPI/Django patterns | Planned |
| 23-adapter-go | Go adapter: go.mod, exported function conventions | Planned |
| 24-adapter-rust | Rust adapter: Cargo.toml, pub fn/struct/enum | Planned |
| 25-adapter-markdown | Markdown adapter: section parsing, traceability links | Planned |
| 26-adapter-subprocess | Subprocess adapter protocol: external parser script contract | Planned |
| 27-adapter-openapi | OpenAPI/REST contract adapter for cross-repo boundary resolution | Planned |
| 28-adapter-protobuf | Protobuf/gRPC contract adapter | Planned |
| 29-adapter-asyncapi | AsyncAPI/event schema adapter (Kafka, SQS, Pub/Sub) | Planned |

---

## Reference Docs (40–49)

Implementation-specific design docs: data models, API contracts, CLI design, deployment, documentation workflows.

| Document | Description | Status |
|----------|-------------|--------|
| [40-metadata-model](./40-metadata-model.md) | Supabase schema, orientation artifacts (llmspec/llms.txt), symbol resolution levels, package discovery | Current |
| [40-mcp-tool-contracts](./40-mcp-tool-contracts.md) | MCP tool contracts: get_session_context, search, load_context + Phase 2+ roadmap | Current |

---

## Implementation Phases (30–39)

| Document | Description | Status |
|----------|-------------|--------|
| [30-implementation-phases](./30-implementation-phases.md) | Phase-by-phase delivery plan: 9 phases from Foundation to Identity/Auth, each with TDD strategy and verifiable acceptance criteria | Current |

---

## Numbering Convention

| Range | Category |
|-------|----------|
| 01–09 | Core design — architecture, package structure, tool contracts, resolution levels, drift, benchmarking, CLI |
| 10–19 | Feature design — project memory, doc tools, incremental indexing, traceability, server, adapters, local model, pattern store, response cache, context manager |
| 20–29 | Pipeline & language/contract adapters |
| 30–39 | Implementation planning — phases, migration guides, deployment runbooks |
| 40–49 | Reference docs — data models, API contracts, CLI design, deployment |

---

## Document Relationships

```
01-architecture ──────────── package structure, interfaces, ADRs (authoritative)
  │
  ├── 20-pipeline-adapter    pipeline stages, Supabase schema, all resolved decisions
  │     ├── 21–26            language adapters (per language)
  │     └── 27–29            contract adapters (OpenAPI, Protobuf, AsyncAPI)
  │
  ├── 30-implementation-phases  phase-by-phase build plan (references 01 + 20)
  │
  ├── 40-metadata-model.md     Supabase schema, orientation artifacts, symbol levels
  └── 40-mcp-tool-contracts.md MCP tool API contracts

  └── architecture.md        Mermaid component maps + three-layer visual

cc-rlm.md ────────────────── reference analysis that informed 20-pipeline-adapter
```

---

## Traceability

Feature → design → code coverage is tracked in `sensei.traceability` (Supabase — source of truth).
`docs/traceability.yaml` is a generated export for CI and offline use. To regenerate:

```
sensei traceability export
```

## Related

- [Features](../features/) — What and why (10 functional modules, Gherkin scenarios)
- [Traceability](../traceability.yaml) — Generated export of feature/design/code coverage
- [Plans](../plans/) — Session-scoped implementation plans
- [Superpowers specs](../superpowers/specs/) — Approved design specs from brainstorming sessions
