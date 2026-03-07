# sensei

A universal AI skills library and toolchain. Scan a repo once, produce structured orientation artifacts, and expose an MCP server so agents load only what they need — fewer tokens, fewer tool calls, better results.

## Core Idea

AI agents dropped into an unfamiliar codebase waste most of their context window on orientation. Sensei solves this:

- **Index once** — `sensei init` scans your repo and writes `.sensei/` artifacts: symbol map, package hierarchy, traceability graph, llmspec
- **Local model analysis** — a small on-device model (Transformers.js ONNX or Ollama) extracts symbols, summaries, flows, examples and relations — no regex, no AST, no API key
- **Right resolution** — code stored at four levels: signature (L0, ~10 tokens), description (L1), logic flow (L2), full source (L3). Agents request the minimum needed
- **Docs stay in sync** — traceability matrix built from model-extracted relations + embedding similarity. `git diff` against last index commit flags exactly which docs need attention
- **Context stays flat** — project memory (decisions, patterns, open items) persists across sessions via MCP tools. `get_session_context()` loads ~300 tokens regardless of project age
- **Improvements are measured** — benchmark task corpus compares token usage and interaction counts with and without skills

## Skills

Install to `~/.claude/skills/` (or equivalent). Teach agents when and how to use the tools.

| Skill | Purpose |
|---|---|
| `codebase-indexer` | Orient on a new repo in one tool call |
| `content-compression` | Load code at the right resolution level |
| `agentic-dev-workflow` | Session protocol: orient → load → work → checkpoint |
| `doc-drift-detector` | Detect and resolve stale docs using git diff + traceability |
| `context-manager` | Load narrow, offload often |
| `project-workflow` | Cross-session knowledge: decisions, patterns, open items |
| `benchmark-runner` | Measure skill impact with A/B task corpus |
| `doc-doctor` | Reformat existing docs to match canonical templates |

## MCP Server (19 tools)

| Category | Tools |
|---|---|
| Query | `get_llmspec`, `get_file_context`, `list_exports`, `find_pattern`, `get_shortcuts` |
| Reindex | `reindex_repo` |
| Context | `load_context`, `recommend_next` |
| Drift | `check_drift` |
| Project memory | `get_session_context`, `checkpoint`, `add_decision`, `add_pattern`, `ask_question`, `get_open_items`, `close_item` |

## CLI

```bash
sensei setup --mcp                 # register MCP server in ~/.claude/mcp.json
sensei init                        # full scan + create .llmspec.yaml, CLAUDE.md, llms.txt, .sensei/
sensei add                         # non-destructive add to existing repo
sensei status                      # index age, symbol count, drift summary
sensei index [--force]             # incremental re-index (git diff) or full rescan
sensei drift [--fail-on-drift]     # check doc drift via traceability matrix
sensei doctor <path> [--dry-run]   # reformat docs to match canonical templates
sensei benchmark doctor --source <input> --dest <output> [--template] [--examples] [--sample N]
                             3-strategy doc conversion benchmark
sensei migrate                     # convert agents/ folder to .sensei/checkpoints/
```

## Repo Structure

```
/
├── packages/
│   ├── shared/ @sensei/shared — types, constants, API contracts (no deps)
│   ├── tools/  @sensei/tools  — tool logic: reindex, query, drift, context, memory
│   ├── server/ @sensei/server — inference engine + telemetry HTTP server
│   ├── mcp/    @sensei/mcp    — thin MCP adapter (wraps @sensei/tools)
│   └── cli/    @sensei/cli    — thin CLI binary (sensei command)
│
│   Dependency graph: shared ← tools ← mcp
│                     shared ← server   cli → tools + server
│
├── skills/                   Skill markdown files (8 skills)
├── tasks/                    Benchmark task corpus
├── docs/templates/           Canonical doc templates (design.md, feature.md)
├── docs/features/            What and why — Gherkin scenarios, status tables
├── docs/design/              How — architecture, schemas, algorithms
├── docs/plans/               Implementation plans
└── README.md                 This file
```

## Getting Started

```bash
# Install sensei globally
cd packages/cli
bun install && bun run build
bun link          # or: bun i -g .

# Register MCP server with Claude Code
cd your-repo
sensei setup --mcp

# Set up a new repo
sensei init

# Add to an existing repo (non-destructive)
sensei add

# Migrate an existing agents/ folder
sensei migrate
```

Skills in `skills/` can be symlinked to `~/.claude/skills/` manually or via any skill manager.

## Development

```bash
bun install          # install dependencies
bun test             # run all 118 unit tests (vitest 4 projects)
bun run build        # build all packages
```

## Docs

- [Features](docs/features/) — what each module does and why
- [Design](docs/design/) — architecture, schemas, algorithms
  - [14-server-package.md](docs/design/14-server-package.md) — server-mediated inference, package split
  - [16-local-model-indexer.md](docs/design/16-local-model-indexer.md) — local model analysis, format decisions
  - [15-package-adapters.md](docs/design/15-package-adapters.md) — glob-based package discovery
- [Plans](docs/plans/) — implementation history
