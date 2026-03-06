# Features

This skills repo is built around a single conviction: **AI agents should spend tokens reasoning, not orienting**. A developer agent dropped into an unfamiliar codebase should be able to orient in one tool call, load exactly what it needs for the current task, and offload all deterministic work to purpose-built tools.

## Vision

- **Index Once, Orient Fast** — Scan a repo once and produce structured artifacts. Future agents load a 500-token spec instead of reading dozens of files.
- **Right Resolution for the Task** — Code has four representations. Agents load signatures for discovery, logic flows for understanding, and full source only when editing. Never more than needed.
- **Docs Stay in Sync** — Design docs, code, and public documentation are tracked together. Drift is detected automatically and reported before it becomes a problem.
- **Deterministic Work Belongs in Tools** — Generating llms.txt, checking drift, listing exports — these are repeatable, deterministic tasks. MCP tools handle them so the LLM's context stays clear for judgment.
- **Context Is Managed, Not Accumulated** — Agents load targeted slices, checkpoint before switching tasks, and never carry stale context into new work.
- **Improvements Are Measured** — Skills are worth nothing if their impact can't be quantified. A benchmark system compares token usage, interaction counts, and task completion with and without skills.
- **One Command to Get Started** — `skills init` sets up any repo in seconds. Developer profiles carry personal standards across all projects. Company profiles enforce shared standards without manual configuration.

## Modules

- [Codebase Indexing](01-CodebaseIndexing.md) — Scan, extract, and produce orientation artifacts
- [Content Compression](02-ContentCompression.md) — Token-efficient code representations at four resolution levels
- [Agentic Dev Workflow](03-AgenticDevWorkflow.md) — Protocol for efficient agentic developer sessions
- [Doc Drift Detection](04-DocDriftDetection.md) — Keep design docs, code, and public docs in sync
- [Context Management](05-ContextManagement.md) — Load narrow, offload often, checkpoint before switching
- [Benchmarking](06-Benchmarking.md) — Quantify skill impact with A/B comparisons
- [CLI](07-CLI.md) — Set up repos, manage profiles, switch contexts, install hooks
- [Project Workflow](08-ProjectWorkflow.md) — Cross-session knowledge persistence, session resume, decision and pattern capture
- [Doc Reformatter](09-DocReformatter.md) — Reformat existing docs to match canonical templates
- [Incremental Indexing](10-IncrementalIndexing.md) — Fast subsequent index runs, only changed files re-processed

## Feature Status

| Module | Feature | Status |
|--------|---------|--------|
| Codebase Indexing | Repo scanner (file map, stack, shortcuts, symbols) | 🔲 Planned |
| | LLMSpec (.llmspec.yaml) generation | 🔲 Planned |
| | CLAUDE.md generation | 🔲 Planned |
| | llms.txt generation | 🔲 Planned |
| | Incremental re-indexing | 🔲 Planned |
| | Project-scoped skill generation | 🔲 Planned |
| Content Compression | L0–L3 resolution levels | 🔲 Planned |
| | Docstring stripping | 🔲 Planned |
| | Logic flow notation | 🔲 Planned |
| | IO pattern notation | 🔲 Planned |
| | MCP-served resolution (get_file_context) | 🔲 Planned |
| Agentic Dev Workflow | Session protocol (orient → load → work → checkpoint) | 🔲 Planned |
| | MCP offload patterns | 🔲 Planned |
| | Task-to-resolution mapping | 🔲 Planned |
| Doc Drift Detection | Doc layer fingerprinting | 🔲 Planned |
| | Drift reporting (on-demand) | 🔲 Planned |
| | Pre-commit hook integration | 🔲 Planned |
| | CI integration | 🔲 Planned |
| Context Management | Targeted context slice loading | 🔲 Planned |
| | Checkpoint and restore | 🔲 Planned |
| | recommend_next (task-to-context prescription) | 🔲 Planned |
| | Token budget guidance | 🔲 Planned |
| Benchmarking | Task corpus (representative developer tasks) | 🔲 Planned |
| | A/B evaluation (with-skills vs without-skills) | 🔲 Planned |
| | Metrics: tokens, interactions, tool calls, success | 🔲 Planned |
| | Results storage and comparison | 🔲 Planned |
| CLI | New repo setup (skills init) | 🔲 Planned |
| | Add to existing repo (skills add) | 🔲 Planned |
| | Upgrade (skills upgrade) | 🔲 Planned |
| | Personal profile create/edit | 🔲 Planned |
| | Company profile create/edit | 🔲 Planned |
| | Remote company MCP registration | 🔲 Planned |
| | Local companion MCP for remote caching | 🔲 Planned |
| | Context status (skills status) | 🔲 Planned |
| | Shared library cache (skills cache) | 🔲 Planned |
| | Pre-commit drift hook (skills hooks install) | 🔲 Planned |
| | Guidelines view/edit/query | 🔲 Planned |
| | get_guidelines MCP tool | 🔲 Planned |
| | Migration from agents/ folder (sensei migrate) | 🔲 Planned |
| Project Workflow | Session resume (get_session_context) | 🔲 Planned |
| | Decision capture (add_decision) | 🔲 Planned |
| | Pattern capture (add_pattern) | 🔲 Planned |
| | Session checkpoint with LLM distillation | 🔲 Planned |
| | Open items tracking (ask_question, close_item) | 🔲 Planned |
| | Context budget stays flat over time | 🔲 Planned |
| Doc Reformatter | Single file reformat (sensei reformat <file>) | 🔲 Planned |
| | Directory batch reformat | 🔲 Planned |
| | Template auto-detection from path | 🔲 Planned |
| | doc-reformatter skill | 🔲 Planned |
| Incremental Indexing | Incremental scan on subsequent runs | 🔲 Planned |
| | Deleted file removal from index | 🔲 Planned |
| | --force flag for full rescan | 🔲 Planned |
| | Index summary output | 🔲 Planned |
