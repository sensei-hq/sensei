# Benchmarking Design

## Overview

The benchmark system runs a task corpus against two configurations (with-skills and without-skills) and records five metrics per task. Results are stored as JSON. A comparison report shows percentage improvement across metrics and highlights weak categories.

---

## Task Corpus Schema

`tasks/sample.yaml`:

```yaml
version: "1.0"

tasks:
  - id: string            # Unique identifier, e.g. "orient-01"
    category: string      # orientation | discovery | understanding | feature-add | bug-fix | refactor | doc-update | drift-check
    prompt: string        # The task given to the agent
    success_criteria: string  # What a correct response looks like
```

**Categories and what they test:**

| Category | Tests | Example task |
|---|---|---|
| orientation | LLMSpec + llms.txt efficiency | "Explain the overall architecture" |
| discovery | list_exports, find_pattern efficiency | "List all functions in the auth module" |
| understanding | L1/L2 vs L3 usage | "What does processOrder return?" |
| feature-add | Pattern + context loading | "Add a discount field to Order" |
| bug-fix | L2/L3 + targeted loading | "Fix the rounding bug in calculateTax" |
| refactor | L3 + pattern awareness | "Extract validation from processOrder" |
| doc-update | Doc layer awareness | "Update README to reflect new CLI flags" |
| drift-check | check_drift usage | "Check if docs are in sync with code" |

---

## Metrics Schema

Each task result:

```json
{
  "taskId": "orient-01",
  "category": "orientation",
  "config": "with-skills",
  "success": true,
  "tokensIn": 850,
  "tokensOut": 420,
  "interactions": 2,
  "toolCalls": {
    "mcp": 1,
    "fileRead": 0,
    "glob": 0,
    "grep": 0
  },
  "driftScore": null,
  "notes": ""
}
```

**Metric definitions:**

| Metric | How measured |
|---|---|
| `tokensIn` | Total prompt tokens across all interactions for this task |
| `tokensOut` | Total completion tokens across all interactions |
| `interactions` | Number of agent turns (user message → agent response) |
| `toolCalls.mcp` | Calls to MCP tools (get_llmspec, list_exports, etc.) |
| `toolCalls.fileRead` | Calls to Read tool (direct file access) |
| `toolCalls.glob` | Calls to Glob tool |
| `toolCalls.grep` | Calls to Grep tool |
| `driftScore` | null unless category is doc-update or drift-check |

---

## Benchmark Run Output

`results/YYYY-MM-DD-benchmark.json`:

```json
{
  "date": "2026-03-06",
  "repoPath": "/path/to/test-repo",
  "configs": {
    "a": "with-skills",
    "b": "without-skills"
  },
  "tasks": [
    {
      "taskId": "orient-01",
      "configA": { /* metrics */ },
      "configB": { /* metrics */ }
    }
  ]
}
```

---

## Comparison Report Format

`compare_results(fileA, fileB)` output:

```
Benchmark Comparison: with-skills vs without-skills
Date: 2026-03-06 | Repo: billing-service

SUMMARY
  Tokens in:      -62%   (avg 920 → 350)
  Tokens out:     -18%   (avg 510 → 420)
  Interactions:   -45%   (avg 4.2 → 2.3)
  File reads:     -78%   (avg 9.1 → 2.0)

BY CATEGORY
  orientation:    -74% tokens | -60% interactions  ✅ Strong
  discovery:      -81% tokens | -70% file reads    ✅ Strong
  understanding:  -55% tokens                       ✅ Good
  feature-add:    -40% tokens                       ✅ Good
  bug-fix:        -22% tokens                       ⚠️  Weak
  refactor:       -18% tokens                       ⚠️  Weak
  doc-update:     -35% tokens                       ✅ Good
  drift-check:    -90% tokens                       ✅ Strong

REGRESSIONS
  None detected.

RECOMMENDED IMPROVEMENTS
  bug-fix and refactor categories show < 25% improvement.
  Review: agentic-dev-workflow and content-compression skills for these task types.
```

---

## A/B Setup

Manual setup in V1. No automated test harness — the developer runs the tasks manually in each configuration and records results.

**with-skills configuration:**
- Repo is indexed (`.llmspec.yaml`, `.index/` present)
- `~/.claude/skills/<name>` symlinks active
- MCP server running with `REPO_PATH` set

**without-skills configuration:**
- No `.index/` directory
- No `.llmspec.yaml`
- `~/.claude/skills/` emptied or skills removed
- MCP server not registered

**Test repo:** A representative codebase of moderate size (10–30 files, 2–5 modules, existing docs). Use the same repo for both configurations. Branch for isolation.

---

## Results Directory Convention

```
results/
  2026-03-06-benchmark.json       Raw results (gitignored)
  2026-03-06-comparison.md        Human-readable summary (committed)
  .gitignore                      *.json
```

Summaries are committed so improvement trends are visible in git history. Raw JSON is gitignored to avoid repo bloat.
