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

## Numbering Convention

| Range | Category |
|-------|----------|
| 01–09 | Core design (architecture, skills, MCP, llmspec, indexing, compression, drift, benchmarking) |
| 10–19 | MCP tool extensions |
| 20–29 | Language-specific extractors |

## Related

- [Features](../features/) — What and why (needs, scenarios, status)
- [Plans](./plans/) — Implementation plans
