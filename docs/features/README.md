# Features

## Objective

Sensei makes AI agents more effective in software development by solving the orientation, context, and continuity problems that make unassisted agents slow and expensive. An agent with sensei orients in one tool call, loads exactly the context it needs, delegates deterministic work to MCP tools, and carries knowledge forward across sessions — spending tokens on reasoning, not rediscovery.

## Vision

- **Index Once, Orient Fast** — Scan a repo once and produce structured artifacts. Future agents load a 500-token spec instead of reading dozens of files.
- **Right Resolution for the Task** — Code has four representations. Agents load signatures for discovery, logic flows for understanding, and full source only when editing. Never more than needed.
- **Docs Stay in Sync** — Design docs, code, and public documentation are tracked together. Drift is detected automatically and reported before it becomes a problem.
- **Deterministic Work Belongs in Tools** — Generating llms.txt, checking drift, listing exports — these are repeatable, deterministic tasks. MCP tools handle them so the LLM's context stays clear for judgment.
- **Context Is Managed, Not Accumulated** — Agents load targeted slices, checkpoint before switching tasks, and never carry stale context into new work.
- **Improvements Are Measured** — Skills are worth nothing if their impact can't be quantified. A benchmark system compares token usage, interaction counts, and task completion with and without skills.
- **One Command to Get Started** — `skills init` sets up any repo in seconds. Developer profiles carry personal standards across all projects. Company profiles enforce shared standards without manual configuration.

## Modules

- [01 — Indexing](01-indexing.md) — Scan, extract, and produce orientation artifacts; incremental updates; multi-modal search; symbol graph
- [02 — Resolution](02-resolution.md) — Token-efficient code representations at four levels (L0–L3)
- [03 — Workflow](03-workflow.md) — Session protocol, project memory, decision capture, and cross-session continuity
- [04 — Traceability](04-traceability.md) — Doc-to-code coverage map, git-based drift detection, pre-commit and CI integration
- [05 — Context](05-context.md) — Targeted slice loading, token budget reporting, and task-scoped context prescriptions
- [06 — Benchmarking](06-benchmarking.md) — A/B comparisons, metrics collection, CLI prompt comparison, and improvement loop
- [07 — CLI](07-cli.md) — Repo setup, profile management, context switching, shared library cache
- [08 — Documentation](08-documentation.md) — Doc guide skill, find_doc tool, scaffold, doc-doctor, and external doc references
- [09 — Patterns](09-patterns.md) — Detect, capture, search, and export recurring patterns as local repo skills
- [10 — Caching](10-caching.md) — Persist and retrieve notable Claude responses across sessions
- [11 — Implementation Sync](11-implementation-sync.md) — Post-implementation doc enrichment, traceability sync, plan archival, incremental llms.txt

## Traceability

Feature items, status, and design coverage are tracked in [`docs/traceability.yaml`](../traceability.yaml). Status lives there only — do not add status tables to feature docs.

## Feature Status

| Module | Feature | Status |
|--------|---------|--------|
| Indexing | Repo scanner (file map, stack, shortcuts, symbols) | 🔲 Planned |
| | LLMSpec (.llmspec.yaml) generation | 🔲 Planned |
| | CLAUDE.md generation | 🔲 Planned |
| | llms.txt generation | 🔲 Planned |
| | Symbol map at L0–L2 | 🔲 Planned |
| | Multi-modal search (semantic + full-text + symbol) | 🔲 Planned |
| | Symbol graph (callers, dependencies, impact analysis) | 🔲 Planned |
| | Full scan on first run | ✅ Done |
| | Incremental scan on subsequent runs | 🔲 Planned |
| | Deleted file removal from index | 🔲 Planned |
| | --force flag for full rescan | 🔲 Planned |
| | Index summary output | 🔲 Planned |
| Resolution | L0–L3 resolution levels | 🔲 Planned |
| | Docstring stripping | 🔲 Planned |
| | Logic flow notation | 🔲 Planned |
| | IO pattern notation | 🔲 Planned |
| | Task-to-level mapping via recommend_next | 🔲 Planned |
| Workflow | Session orientation protocol (LLMSpec first) | 🔲 Planned |
| | Targeted context loading | 🔲 Planned |
| | MCP offload protocol | 🔲 Planned |
| | Task transition with checkpoint | 🔲 Planned |
| | Plan-to-implementation efficiency | 🔲 Planned |
| | Analysis-before-implementation gate | 🔲 Planned |
| | Session resume (get_session_context) | 🔲 Planned |
| | Decision capture (add_decision) | 🔲 Planned |
| | Pattern capture (add_pattern) | 🔲 Planned |
| | Session checkpoint with distillation | 🔲 Planned |
| | Open items (ask_question, close_item) | 🔲 Planned |
| | Migration from agents/ folder (sensei migrate) | 🔲 Planned |
| Traceability | Manual coverage declaration in .llmspec.yaml | 🔲 Planned |
| | Auto-detection from filename and symbol references | 🔲 Planned |
| | .index/traceability.json generation | 🔲 Planned |
| | Cross-reference drift (code changed, linked doc didn't) | 🔲 Planned |
| | On-demand drift reporting (check_drift) | 🔲 Planned |
| | Pre-commit hook integration | 🔲 Planned |
| | CI integration (--fail-on-drift) | 🔲 Planned |
| Context | Targeted slice loading (load_context) | 🔲 Planned |
| | Token budget reporting per slice | 🔲 Planned |
| | Context summary (get_context_summary) | 🔲 Planned |
| | Named and timestamped checkpoints | 🔲 Planned |
| | recommend_next (task-to-context prescription) | 🔲 Planned |
| Benchmarking | Task corpus (representative developer tasks) | 🔲 Planned |
| | A/B evaluation (with-skills vs without-skills) | 🔲 Planned |
| | Metrics: tokens, interactions, tool calls, success | 🔲 Planned |
| | Results storage and comparison | 🔲 Planned |
| | CLI prompt comparison (sensei benchmark prompt) | 🔲 Planned |
| | Prompt comparison result storage | 🔲 Planned |
| CLI | New repo setup (skills init) | 🔲 Planned |
| | Add to existing repo (skills add) | 🔲 Planned |
| | Upgrade (skills upgrade) | 🔲 Planned |
| | Personal profile create/edit | 🔲 Planned |
| | Company profile create/edit | 🔲 Planned |
| | Remote company MCP registration | 🔲 Planned |
| | Context status (skills status) | 🔲 Planned |
| | Shared library cache (skills cache) | 🔲 Planned |
| | Pre-commit drift hook (skills hooks install) | 🔲 Planned |
| | Guidelines view/edit/query | 🔲 Planned |
| | Migration from agents/ folder (sensei migrate) | 🔲 Planned |
| Documentation | doc-guide skill | 🔲 Planned |
| | find_doc MCP tool | 🔲 Planned |
| | New feature scaffold (sensei doc new) | 🔲 Planned |
| | External doc fetch and cache (fetch_doc_ref) | 🔲 Planned |
| | Single file doctor (sensei doctor <file>) | 🔲 Planned |
| | Directory batch doctor (sensei doctor <dir>) | 🔲 Planned |
| | Template auto-detection from path | 🔲 Planned |
| Patterns | Pattern detection from code (2+ usages) | 🔲 Planned |
| | Pattern detection from design docs | 🔲 Planned |
| | Pattern capture (add_pattern MCP tool) | 🔲 Planned |
| | Pattern search (find_pattern MCP tool) | 🔲 Planned |
| | Pattern-to-skill export (sensei pattern export) | 🔲 Planned |
| Caching | Manual response capture (cache_response MCP tool) | 🔲 Planned |
| | Proactive cache offer for significant responses | 🔲 Planned |
| | Retrieval by semantic query (find_cached_response) | 🔲 Planned |
| | Cache hints in get_session_context() | 🔲 Planned |
| | TTL-based expiry with retrieval extension | 🔲 Planned |
| | Pin/delete via CLI (sensei cache pin / delete) | 🔲 Planned |
