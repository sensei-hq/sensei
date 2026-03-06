# Architecture

## Overview

The repo has four layers: **skills** (markdown guidance files), an **MCP server** (compute and data layer), a **CLI** (`skills` binary, setup and profile management), and **repo artifacts** (files written into indexed codebases). Skills teach agents the protocol. The MCP server handles execution. The CLI manages the developer experience. Repo artifacts persist the index between sessions.

---

## Repo Structure

Bun workspaces monorepo. One package today (`repo-index-server`), structured for additional packages later.

```
/                               ← workspace root
  package.json                  ← bun workspaces config ("name": "sensei")
  bun.lockb
  README.md

  packages/
    repo-index-server/          ← MCP server + skills CLI
      package.json              ← "bin": { "sensei": "./dist/cli.js" }
      tsconfig.json
      src/
        index.ts                MCP server entry point, tool registration
        cli.ts                  skills CLI entry point
        index-reader.ts         Reads .index/ and .llmspec.yaml
        types.ts                LlmSpec, SymbolMap, ResolutionLevel types
        commands/               CLI command modules
          init.ts               skills init
          add.ts                skills add
          upgrade.ts            skills upgrade
          status.ts             skills status
          profile.ts            skills profile create/edit/list/use
          company.ts            skills company create/edit/register-mcp
          guidelines.ts         skills guidelines [edit|show]
          cache.ts              skills cache add/list/update
          hooks.ts              skills hooks install
          index-cmd.ts          skills index
          drift-cmd.ts          skills drift
        tools/
          query.ts              get_llmspec, get_file_context, list_exports, find_pattern, get_shortcuts
          reindex.ts            reindex_repo (scanner + writer)
          context.ts            load_context, checkpoint, recommend_next
          drift.ts              check_drift
          generate.ts           generate_llms_txt, generate_changelog
          benchmark.ts          run_benchmark, compare_results, get_metrics_summary
          guidelines.ts         get_guidelines, get_profile
          cache.ts              query_cache
      src/**/*.spec.ts          Unit tests (Vitest)
      e2e/                      End-to-end tests (Playwright)
        *.e2e.ts

  skills/                       Skill markdown files
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

~/.skills/                      Developer profile directory (created by skills CLI)
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
| Incremental re-indexing: full rescan vs git-diff-based | Deferred — full rescan for V1, diff-based in V2 |
| Multi-repo support: single MCP instance per repo vs shared | Deferred — per-repo for V1 |
| L2 generation: pre-computed vs on-demand LLM summarisation | Deferred — placeholder for V1, LLM-generated in V2 |
| Benchmark runner: manual A/B vs automated test harness | Manual setup for V1 |
