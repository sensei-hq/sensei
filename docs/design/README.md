# Design Documentation

Architecture, structure, and implementation details for the AI skills repo.

## Core Design Documents

| Document | Description |
|----------|-------------|
| [01-architecture](./01-architecture.md) | Overall system architecture, repo structure, component relationships |
| [02-skills](./02-skills.md) | Skill file format, naming conventions, CSO, testing requirements |
| [03-mcp-server](./03-mcp-server.md) | MCP server design, tool categories, APIs, tool contracts |
| [04-llmspec](./04-llmspec.md) | LLMSpec format (.llmspec.yaml), fields, generation, querying |
| [05-indexing](./05-indexing.md) | Indexer design, extraction targets, storage format, incremental updates |
| [06-compression](./06-compression.md) | Resolution levels, storage schema, serving logic, notation formats |
| [07-drift](./07-drift.md) | Fingerprint system, drift detection algorithm, hook integration |
| [08-benchmarking](./08-benchmarking.md) | Benchmark architecture, task corpus schema, metrics, A/B setup |
| [09-cli](./09-cli.md) | CLI design, layered profile system, command modules, config schemas, hooks |
| [10-project-memory](./10-project-memory.md) | Cross-session knowledge layer: checkpoint distillation, memory/patterns/open-items schemas, MCP tools, migration from agents/ |
| [11-doc-reformatter](./11-doc-reformatter.md) | Doc reformatter: template detection, prompt structure, CLI interface, doc-reformatter skill |
| [12-incremental-indexing](./12-incremental-indexing.md) | Incremental indexing: change detection algorithm, force flag, summary output |

## Numbering Convention

| Range | Category |
|-------|----------|
| 01–09 | Core design (architecture, skills, MCP, llmspec, indexing, compression, drift, benchmarking, CLI) |
| 10–19 | MCP tool extensions |
| 20–29 | Language-specific extractors |

## Related

- [Features](../features/) — What and why (needs, scenarios, status)
- [Plans](./plans/) — Implementation plans
