# AI Skills Repo — Design Document
**Date:** 2026-03-06
**Status:** Approved

---

## Overview

A universal, model-agnostic skills library that helps AI agents (Claude, and others) work more efficiently on codebases. The core idea: scan a repo once, produce structured artifacts (skills, an LLM spec, a context index), and expose an MCP server so agents load only what they need — reducing tokens, reducing interactions, and producing consistent results.

---

## Repository Structure

```
skills/
  skills/
    codebase-indexer/       # Scan repo → build index + artifacts
      SKILL.md
      extractor.md
    content-compression/    # Token-efficient code/content representations
      SKILL.md
    agentic-dev-workflow/   # How AI developer agents should work efficiently
      SKILL.md
    doc-drift-detector/     # Detect sync drift across doc layers
      SKILL.md
    context-manager/        # Protocol for loading/offloading context
      SKILL.md
    benchmark-runner/       # How to set up and run skill evaluations
      SKILL.md
  mcp/
    repo-index-server/      # MCP server: index queries + compute offload
      index.ts
      README.md
  tasks/
    sample.yaml             # Representative task corpus for benchmarking
  results/                  # Benchmark outputs (summaries committed)
  docs/
    plans/                  # Design documents
  install.sh                # Symlinks skills to ~/.claude/skills, registers MCP
```

---

## Skills

### `codebase-indexer`
Guides an agent through scanning a repo and producing all output artifacts. Triggered when starting work on an unfamiliar codebase or after significant changes.

**Extracts:**
- File map: directory tree, key entry points (main, router, config, index files)
- Tech stack: languages, frameworks, package managers, major dependencies
- Code patterns: naming conventions, file organisation, common idioms
- Exports/symbols: public APIs, key functions/classes per module (stored at all resolution levels)
- Dev shortcuts: npm/make/bash scripts, aliases, common commands
- Documentation: design docs, ADRs, feature specs, READMEs, public docs, changelogs
- Existing skills/context: CLAUDE.md, .cursor rules, existing skills, MCP configs

**Outputs per repo:**
```
<repo-root>/
  CLAUDE.md                    # Auto-generated or merged if exists
  llms.txt                     # llmstxt.org standard, LLM-friendly project summary
  .llmspec.yaml                # Structured LLM spec (see below)
  .index/
    symbol-map.json            # Compact exports/entry-points at all resolution levels
    patterns.md                # Detected conventions
    shortcuts.md               # Scripts and commands
    stack.md                   # Tech stack summary
    doc-index.json             # Doc layer fingerprints for drift detection

~/.claude/skills/<project>/    # Auto-generated project-scoped skills
  patterns.md
  shortcuts.md
  entry-points.md
```

---

### `content-compression`
Teaches agents how to represent code and content at the minimal resolution needed for the current task. Reduces token usage without losing the information needed to reason correctly.

**Code resolution levels:**

| Level | Format | Approx tokens | When to use |
|---|---|---|---|
| L0 — Signature | `processOrder(orderId: string): Promise<Order>` | ~10 | Discovery, listing |
| L1 — IO Pattern | `order = processOrder(orderId)` + types | ~30 | Understanding what |
| L2 — Logic Flow | Bullet steps / pseudocode | ~80 | Understanding how |
| L3 — Full Source | Actual code | 200–2000 | Editing, debugging |

**Key principles:**
- Docstrings and doc-comments are stripped — they repeat what signatures already convey to LLMs
- Documentation belongs in the design/feature doc layer, not inline code
- Logic flows use plain English bullets and IO chain notation (`raw → parse() → validate() → output`)
- State machines use shorthand (`idle → loading → success/error`)
- Imports, type boilerplate, decorators stripped at L0/L1/L2

**Task-to-level mapping:**
```
"list available functions"       → L0
"what does login() return?"      → L1
"trace the auth flow"            → L2
"fix a bug in validateToken()"   → L3
```

---

### `agentic-dev-workflow`
Protocol for how AI developer agents should work efficiently: orient first, load narrow, work targeted, checkpoint before switching, offload deterministic tasks to MCP.

**Core pattern:**
1. Start → load `.llmspec.yaml` (~500 tokens for full orientation)
2. Get task → call `recommend_next(task)` → load targeted slice only
3. Work → call `query_index()` for specifics, never load full files unless L3 needed
4. Switch task → call `checkpoint()`, unload previous slice, load new one

---

### `doc-drift-detector`
Detects when the three documentation layers fall out of sync with each other or with code.

**Three doc layers:**
1. Design/feature docs — ADRs, specs, plans, feature writeups
2. Code docs — inline comments, type signatures, exported API surface
3. Public docs — README, guides, tutorials, changelogs

**Drift detection:** The indexer records file hashes + last-modified timestamps. The MCP `check_drift` tool compares current state against the stored fingerprints and reports:
- Docs referencing code that has since changed
- Code with no corresponding design or public documentation
- Public docs lagging behind design/feature docs

Can be run on demand or as a pre-commit/CI hook.

---

### `context-manager`
Teaches agents the protocol for managing what's in context — what to load, when to offload, how to summarise before switching tasks.

**Principles:**
- Start narrow: orientation only (llmspec), never load full repo
- Expand on demand: one targeted slice at a time via MCP
- Summarise before switching: compress current context to checkpoint before loading next task
- Offload deterministic work: generation, validation, formatting → MCP tools, not LLM context

---

### `benchmark-runner`
How to set up and run comparative evaluations of skills vs no-skills on a representative task corpus.

---

## LLM Spec (`.llmspec.yaml`)

An OpenAPI-equivalent structured format for LLM consumption. The primary orientation artifact.

```yaml
project: my-app
version: 1.0.0
description: One-sentence project summary
stack: [typescript, react, postgres]
entry_points:
  - path: src/index.ts
    role: server entry
  - path: src/router.ts
    role: route definitions
concepts:
  - name: Order
    definition: A confirmed purchase with line items and a status lifecycle
patterns:
  - name: Repository pattern
    files: src/repositories/
    convention: All DB access goes through repository classes, never direct queries in handlers
api_surface:
  - name: processOrder
    path: src/services/orders.ts
    io: "order = processOrder(orderId: string)"
    flow: validate → fetch items → charge → emit event
doc_layers:
  design: docs/plans/
  code: src/
  public: [README.md, docs/guides/]
shortcuts:
  dev: npm run dev
  test: npm test
  index: npm run index   # re-runs codebase-indexer
```

---

## MCP Server (`repo-index-server`)

A local server that reads from `.index/` and `.llmspec.yaml` and serves targeted slices on demand. Registered once via `install.sh`.

**Tool categories:**

| Category | Tools |
|---|---|
| Querying | `query_index`, `get_file_context(path, level)`, `find_pattern`, `list_exports`, `get_shortcuts` |
| LLM Spec | `get_llmspec`, `get_llmspec_section(section)` |
| Generation | `generate_llms_txt`, `generate_changelog`, `generate_api_docs` |
| Validation | `check_drift`, `validate_llmspec`, `lint_docs` |
| Flows | `reindex_repo`, `sync_docs`, `onboard_agent` |
| Context | `load_context(scope)`, `get_context_summary`, `recommend_next(task)`, `checkpoint` |
| Benchmark | `run_benchmark(task, config)`, `compare_results(a, b)`, `get_metrics_summary` |

---

## Benchmarking + Evaluation

**A/B evaluation setup:**
- `branch/with-skills` — index + skills + MCP loaded
- `branch/without-skills` — raw repo, no index, no MCP

**Metrics per task:**
- Token count (in/out)
- Interaction count (turns to complete)
- Tool call count (searches, reads)
- Task completion (success/failure, output correctness)
- Drift score (doc sync state before/after)

**Task corpus** (`tasks/sample.yaml`) covers representative developer tasks:
- Orientation: "explain the auth flow"
- Bug fix: "fix the failing test in orders.ts"
- Feature add: "add a discount field to the Order type"
- Doc update: "update README to reflect new CLI flags"
- Refactor: "extract the validation logic from processOrder"

**CLI:**
```bash
benchmark run --task-corpus tasks/sample.yaml \
              --branch-a with-skills \
              --branch-b without-skills \
              --output results/2026-03-06.json
```

Results stored as JSON, summaries committed to `results/`.

---

## Install

```bash
./install.sh [--claude] [--codex] [--all]
```

- Symlinks `skills/` to `~/.claude/skills/<skill-name>` (or equivalent)
- Registers MCP server in Claude's MCP config
- Drops `.llmspec.yaml` template in current repo if not present

---

## Open Questions

- MCP server language: TypeScript (Node) vs Python — decision deferred to implementation planning
- Incremental reindexing: full rescan vs git-diff-based update — deferred
- Multi-repo support: single MCP instance serving multiple indexed repos — deferred
