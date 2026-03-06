# Architecture

## Overview

The repo has four layers: **skills** (markdown guidance files), an **MCP server** (compute and data layer), a **CLI** (`skills` binary, setup and profile management), and **repo artifacts** (files written into indexed codebases). Skills teach agents the protocol. The MCP server handles execution. The CLI manages the developer experience. Repo artifacts persist the index between sessions.

---

## Repo Structure

```
skills/
  codebase-indexer/       SKILL.md + extractor.md + llmspec-template.yaml
  content-compression/    SKILL.md
  agentic-dev-workflow/   SKILL.md
  doc-drift-detector/     SKILL.md
  context-manager/        SKILL.md
  benchmark-runner/       SKILL.md

mcp/
  repo-index-server/
    src/
      index.ts            MCP server entry point, tool registration
      index-reader.ts     Reads .index/ and .llmspec.yaml
      types.ts            LlmSpec, SymbolMap, ResolutionLevel types
      tools/
        query.ts          get_llmspec, get_file_context, list_exports, find_pattern, get_shortcuts
        reindex.ts        reindex_repo (scanner + writer)
        context.ts        load_context, checkpoint, recommend_next
        drift.ts          check_drift
        generate.ts       generate_llms_txt, generate_changelog
        benchmark.ts      run_benchmark, compare_results, get_metrics_summary
      guidelines.ts     get_guidelines, get_profile
      cache.ts          query_cache
    cli.ts              CLI entry point (shares tools/ with MCP server)
    commands/           CLI command modules
      init.ts           skills init
      add.ts            skills add
      upgrade.ts        skills upgrade
      status.ts         skills status
      profile.ts        skills profile create/edit/list/use
      company.ts        skills company create/edit/register-mcp
      guidelines.ts     skills guidelines [edit|show]
      cache.ts          skills cache add/list/update
      hooks.ts          skills hooks install
      index-cmd.ts      skills index
      drift-cmd.ts      skills drift
    package.json        "bin": { "skills": "./dist/cli.js" }
    tsconfig.json

~/.skills/              Developer profile directory (created by CLI)
  config.yaml
  profiles/
    personal/           profile.yaml, guidelines.md, skills.yaml
    companies/<name>/   profile.yaml, guidelines.md, skills.yaml
  cache/<lib-name>/     Indexed external libraries

tasks/
  sample.yaml             Representative developer task corpus

results/                  Benchmark run summaries (JSON gitignored, .md committed)

docs/
  features/               What and why
  design/                 How (this directory)
  plans/                  Implementation plans

install.sh                Bootstrap only — for first-time setup before CLI is available
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
      "command": "node",
      "args": ["/path/to/mcp/repo-index-server/dist/index.js"],
      "env": { "REPO_PATH": "/path/to/target/repo" }
    }
  }
}
```

For multi-repo support (future): the server can accept `repoPath` as an optional parameter on each tool call, falling back to `REPO_PATH`.

---

## Install Script Behaviour

`install.sh --claude`:
1. Creates `~/.claude/skills/` if it doesn't exist
2. Symlinks each `skills/<name>/` into `~/.claude/skills/<name>`
3. Builds the MCP server (`npm run build`)
4. Merges the MCP server entry into `~/.claude/mcp.json`
5. Sets `REPO_PATH` to `process.cwd()` at install time (can be changed manually)

Symlinking means skills stay current with repo updates without reinstalling.

---

## Technology Choices

| Component | Choice | Reason |
|---|---|---|
| MCP server language | TypeScript | Native MCP SDK support, type safety for tool contracts |
| MCP SDK | `@modelcontextprotocol/sdk` | Standard, maintained by Anthropic |
| CLI prompts | `@clack/prompts` | Beautiful, accessible prompts — select, multiselect, confirm, spinner, note |
| YAML parser | `js-yaml` | Lightweight, well-maintained, handles .llmspec.yaml |
| File globbing | `fast-glob` | Fastest glob implementation for Node.js |
| Test framework | Vitest | Fast, ESM-native, compatible with TypeScript |
| Build | `tsc` | Standard TypeScript compilation to dist/ |

---

## Open Questions

| Question | Status |
|---|---|
| Incremental re-indexing: full rescan vs git-diff-based | Deferred — full rescan for V1, diff-based in V2 |
| Multi-repo support: single MCP instance per repo vs shared | Deferred — per-repo for V1 |
| L2 generation: pre-computed vs on-demand LLM summarisation | Deferred — placeholder for V1, LLM-generated in V2 |
| Benchmark runner: manual A/B vs automated test harness | Manual setup for V1 |
