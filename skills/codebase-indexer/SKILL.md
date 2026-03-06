---
name: codebase-indexer
description: Use when starting work on an unfamiliar codebase, when a codebase has changed significantly, when an agent is doing broad file searches to orient itself, or when setting up a repo for efficient AI agent use.
---

# Codebase Indexer

## Overview

Scan a repo once, produce structured artifacts so future agents orient in ~500 tokens instead of hundreds of file reads. Outputs: `.llmspec.yaml`, `CLAUDE.md`, `llms.txt`, `.index/` directory.

**REQUIRED:** Use `content-compression` skill to understand resolution levels before indexing.

## When to Run

- First time working on a repo
- After a major refactor or feature addition
- When an agent reports spending many turns on orientation
- Before running a benchmark

## Steps

**Step 1: Check for existing index**

```bash
ls .index/ 2>/dev/null && cat .llmspec.yaml 2>/dev/null
```

If index exists and is recent (< 7 days or no major commits since), call `get_llmspec()` MCP tool and stop — no need to re-index.

**Step 2: Run the indexer MCP tool**

```
call: reindex_repo({ path: ".", output: ".index/" })
```

This scans the repo using the extractor guide (`extractor.md`) and writes all output artifacts.

**Step 3: Review and fill gaps in `.llmspec.yaml`**

Auto-generated fields will be populated. Manually review and complete:
- `concepts` — domain terms that aren't obvious from code
- `patterns` — conventions that require judgment to identify
- `description` — one-sentence project summary

**Step 4: Verify outputs**

```bash
ls .index/          # symbol-map.json, patterns.md, shortcuts.md, stack.md, doc-index.json
cat llms.txt        # LLM-friendly summary
cat CLAUDE.md       # Project context for Claude Code
```

**Step 5: Commit index artifacts**

```bash
git add .llmspec.yaml llms.txt CLAUDE.md
git add .index/patterns.md .index/shortcuts.md .index/stack.md
# symbol-map.json and doc-index.json: gitignore or commit based on team preference
git commit -m "chore: add/update codebase index"
```

## Output Artifacts

| Artifact | Purpose | Where |
|---|---|---|
| `.llmspec.yaml` | Primary LLM orientation spec | Repo root |
| `llms.txt` | llmstxt.org standard summary | Repo root |
| `CLAUDE.md` | Claude Code project context | Repo root |
| `.index/symbol-map.json` | All exports at L0–L2 | Repo root |
| `.index/patterns.md` | Detected conventions | Repo root |
| `.index/shortcuts.md` | Dev commands | Repo root |
| `.index/stack.md` | Tech stack summary | Repo root |
| `.index/doc-index.json` | Doc layer fingerprints | Repo root |

## Re-indexing

Call `reindex_repo()` again any time. It diffs against the previous index and only re-processes changed files.
