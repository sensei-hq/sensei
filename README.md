# sensei

A universal AI skills library and toolchain. Scan a repo once, produce structured orientation artifacts, and expose an MCP server so agents load only what they need — fewer tokens, fewer tool calls, better results.

## Core Idea

AI agents dropped into an unfamiliar codebase waste most of their context window on orientation. Sensei solves this:

- **Index once** — `sensei init` scans your repo and writes `.llmspec.yaml`, `CLAUDE.md`, `llms.txt`, and `.index/`
- **Right resolution** — Code stored at four levels: signature (L0, ~10 tokens), IO pattern (L1), logic flow (L2), full source (L3, ~2000 tokens). Agents request the minimum needed
- **Docs stay in sync** — Traceability matrix maps each design doc to the code it covers. `git diff` against the last index commit flags exactly which docs need attention — no false positives
- **Context stays flat** — Project memory (decisions, patterns, open items) persists across sessions via MCP tools. `get_session_context()` loads ~300 tokens regardless of project age
- **Improvements are measured** — Benchmark task corpus compares token usage and interaction counts with and without skills

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
sensei init                        # full scan + create .llmspec.yaml, CLAUDE.md, llms.txt, .index/
sensei add                         # non-destructive add to existing repo
sensei status                      # index age, symbol count, drift summary
sensei index [--force]             # incremental re-index (git diff) or full rescan
sensei drift [--fail-on-drift]     # check doc drift via traceability matrix
sensei doctor <path> [--dry-run]   # reformat docs to match canonical templates
sensei migrate                     # convert agents/ folder to .index/checkpoints/
```

## Repo Structure

```
/
├── packages/sensei/
│   ├── src/
│   │   ├── index.ts          MCP server entry (19 tools)
│   │   ├── cli.ts            sensei CLI entry
│   │   ├── commands/         init, add, status, doctor, migrate
│   │   └── tools/            reindex, query, drift, context, project-memory
│   └── src/**/*.spec.ts      Unit tests (Vitest, 50 tests)
│
├── skills/                   Skill markdown files (8 skills)
├── tasks/sample.yaml         Benchmark task corpus
├── docs/templates/           Canonical doc templates (design.md, feature.md)
├── docs/features/            What and why — Gherkin scenarios, status tables
├── docs/design/              How — architecture, schemas, algorithms (13 docs)
├── docs/plans/               Implementation plans
└── install.sh                Symlink installer + MCP server registration
```

## Getting Started

```bash
# Install dependencies and build
bun install
bun run build

# Install skills + register MCP server with Claude Code
./install.sh --claude

# Set up a new repo
cd your-repo
sensei init

# Add to an existing repo (non-destructive)
cd existing-repo
sensei add

# Migrate an existing agents/ folder
sensei migrate
```

## Development

```bash
bun install          # install dependencies
bun test             # run unit tests (50 tests)
bun run build        # build MCP server + CLI
```

## Docs

- [Features](docs/features/) — what each module does and why
- [Design](docs/design/) — architecture, schemas, algorithms
- [Plans](docs/plans/) — implementation history
