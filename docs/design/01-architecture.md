---
id: architecture
type: design
implements: []
---

# Architecture

## Overview

The repo has four layers: **skills** (markdown guidance files), an **MCP server** (compute and data layer), a **CLI** (`skills` binary, setup and profile management), and **repo artifacts** (files written into indexed codebases). Skills teach agents the protocol. The MCP server handles execution. The CLI manages the developer experience. Repo artifacts persist the index between sessions.

## Non-Functional Requirements

| NFR | Requirement |
|-----|-------------|
| maintainability | Component boundaries must be clear enough that any module can be replaced without changing others |
| scalability | Architecture must support adding new language adapters without modifying core indexer |

---

## Repo Structure

Bun workspaces monorepo. Root is kept clean — only `package.json`, `bun.lockb`, `README.md`, `.gitignore`. All config, packages, and apps live in dedicated subdirectories.

```
/                               ← repo root (clean)
  package.json                  ← bun workspace root ("workspaces": ["packages/*", "apps/*"])
  bun.lockb
  README.md
  .gitignore

  config/                       ← shared configuration (not a workspace package)
    tsconfig.base.json          ← base TypeScript config (strict, ESNext, bundler resolution)
    eslint.config.js            ← shared ESLint rules
    vitest.config.base.ts       ← base vitest config (extended per package)

  packages/
    sensei/                     ← MCP server + sensei CLI
      package.json              ← "name": "sensei", "bin": { "sensei": "./dist/cli.js" }
      tsconfig.json             ← extends ../../config/tsconfig.base.json
      vitest.config.ts          ← extends ../../config/vitest.config.base.ts
      playwright.config.ts      ← e2e test config
      src/
        index.ts                MCP server entry point, tool registration
        cli.ts                  sensei CLI entry point
        index-reader.ts         Reads .index/ and .llmspec.yaml
        types.ts                LlmSpec, SymbolMap, ResolutionLevel types
        commands/               CLI command modules (init, add, upgrade, status, …)
        tools/                  Shared implementations (query, reindex, drift, context, …)
      src/**/*.spec.ts          Unit tests (Vitest)
      e2e/
        *.e2e.ts                End-to-end tests (Playwright)

  apps/                         ← Applications (empty for now)
    site/                       ← Documentation / marketing site (future)

  skills/                       ← Skill markdown files (not a workspace package)
    codebase-indexer/           SKILL.md + extractor.md + llmspec-template.yaml
    content-compression/        SKILL.md
    agentic-dev-workflow/       SKILL.md
    doc-drift-detector/         SKILL.md
    context-manager/            SKILL.md
    benchmark-runner/           SKILL.md

  tasks/
    sample.yaml                 Representative developer task corpus

  results/                      Benchmark run summaries (*.json gitignored, *.md committed)

  docs/
    features/                   What and why
    design/                     How (this directory)
    plans/                      Implementation plans

~/.skills/                      Developer profile directory (created by sensei CLI)
  config.yaml
  profiles/
    personal/                   profile.yaml, guidelines.md, skills.yaml
    companies/<name>/           profile.yaml, guidelines.md, skills.yaml
  cache/<lib-name>/             Indexed external libraries
```

---

## Component Relationships

```
Agent (Claude or other)
       │
       │  reads skills from ~/.claude/skills/
       ▼
Skills (markdown)
  ├── codebase-indexer    teaches: when/how to run reindex_repo
  ├── content-compression teaches: which resolution level to request
  ├── agentic-dev-workflow teaches: session protocol, MCP offload rules
  ├── doc-drift-detector  teaches: when/how to use check_drift
  ├── context-manager     teaches: load_context, checkpoint, recommend_next
  └── benchmark-runner    teaches: how to set up and run evaluations
       │
       │  calls tools via MCP
       ▼
MCP Server (repo-index-server)
  ├── query tools         read from .index/ and .llmspec.yaml
  ├── reindex tool        scan repo → write .index/ + .llmspec.yaml
  ├── context tools       load_context, checkpoint, recommend_next
  ├── drift tools         check_drift compares current state vs fingerprints
  ├── generation tools    generate_llms_txt, generate_changelog
  └── benchmark tools     run_benchmark, compare_results
       │
       │  reads from / writes to
       ▼
Repo Artifacts (in each indexed repo)
  .llmspec.yaml           primary orientation spec
  llms.txt                llmstxt.org standard summary
  CLAUDE.md               Claude Code project context
  .index/
    symbol-map.json       exports at L0–L2 per file
    patterns.md           detected conventions
    shortcuts.md          dev commands
    stack.md              tech stack
    doc-index.json        doc layer fingerprints
    checkpoints/          agent context checkpoints
```

---

## Data Flow: Indexing

```
reindex_repo(path)
       │
       ├── detectStack()        → reads package.json, pyproject.toml, etc.
       ├── detectShortcuts()    → reads package.json scripts, Makefile, justfile
       ├── buildSymbolMap()     → globs code files, extractExports() per file
       ├── buildDocIndex()      → globs doc files, records mtime + size
       │
       ├── write .index/stack.md
       ├── write .index/shortcuts.md
       ├── write .index/symbol-map.json
       ├── write .index/doc-index.json
       ├── write .index/patterns.md  (placeholder, human-reviewed)
       │
       ├── if no .llmspec.yaml → write template
       ├── write llms.txt
       └── if no CLAUDE.md → write template
```

---

## Data Flow: Agent Session

```
Agent starts
  → get_llmspec()                          ~500 tokens, full orientation
  → recommend_next(task)                   prescription: scope + level
  → load_context(scope) or get_file_context(path, level)
  → work on task
  → for specifics: query_index / find_pattern / list_exports
  → task done → checkpoint()
  → recommend_next(next_task)
  → load new slice
  → ...
```

---

## MCP Server Configuration

The server reads `REPO_PATH` from environment to know which repo to serve. Set at registration time in install.sh:

```json
{
  "mcpServers": {
    "repo-index-server": {
      "command": "bun",
      "args": ["run", "/path/to/packages/repo-index-server/src/index.ts"],
      "env": { "REPO_PATH": "/path/to/target/repo" }
    }
  }
}
```

For multi-repo support (future): the server can accept `repoPath` as an optional parameter on each tool call, falling back to `REPO_PATH`.

---

## Bootstrap / Install

First-time setup (before CLI is available globally):

```bash
bun install
bun run build
bun link   # makes `skills` available as a global command
skills init
```

Once `skills` is globally available, all setup is handled through the CLI.

The CLI (`skills init`) handles:
1. Creates `~/.claude/skills/` if missing
2. Symlinks each `skills/<name>/` into `~/.claude/skills/<name>`
3. Builds and registers the MCP server in `~/.claude/mcp.json`
4. Writes `.skills/project.yaml` in the target repo

Symlinking means skills stay current with repo updates without reinstalling.

---

## Technology Choices

| Component | Choice | Reason |
|---|---|---|
| Language | TypeScript | Native MCP SDK support, type safety for tool contracts |
| Runtime / package manager | Bun | Fast installs, built-in bundler, workspace support |
| Monorepo | Bun workspaces | Single lockfile, shared dependencies across packages |
| MCP SDK | `@modelcontextprotocol/sdk` | Standard, maintained by Anthropic |
| CLI prompts | `@clack/prompts` | Beautiful, accessible prompts — select, multiselect, confirm, spinner, note |
| YAML parser | `js-yaml` | Lightweight, well-maintained, handles .llmspec.yaml |
| File globbing | `fast-glob` | Fastest glob implementation for Node.js |
| Unit tests | Vitest | Fast, ESM-native, `*.spec.ts` files co-located with source |
| E2E tests | Playwright | Full CLI and MCP tool integration tests, `e2e/*.e2e.ts` |

---

## Open Questions

| Question | Status |
|---|---|
| Incremental re-indexing: full rescan vs git-diff-based | Resolved — see `12-incremental-indexing.md` |
| Multi-repo support: single MCP instance per repo vs shared | Deferred — per-repo for V1 |
| L2 generation: pre-computed vs on-demand LLM summarisation | Resolved — local model approach, see `16-local-model-indexer.md` |
| Benchmark runner: manual A/B vs automated test harness | Manual setup for V1 |

---

## Architecture Evolution

This document describes the v1 architecture (single `packages/sensei/` package, CLI + MCP co-located).

Subsequent design decisions that extend or supersede sections above:

| Document | What it changes |
|---|---|
| [14-server-package](./14-server-package.md) | Splits into CLI + inference server + MCP packages; server-mediated model inference; local/org/cloud deployment |
| [15-package-adapters](./15-package-adapters.md) | Adds folder-map artifact and glob-based package discovery layer above the symbol map |
| [16-local-model-indexer](./16-local-model-indexer.md) | Replaces regex symbol extraction with local model inference (Ollama + ONNX) |
