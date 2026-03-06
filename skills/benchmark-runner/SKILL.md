---
name: benchmark-runner
description: Use when you want to measure the impact of skills on token usage and interaction counts, or when comparing agent performance with and without skills on a representative task corpus.
---

# Benchmark Runner

## Overview

Quantify skill value with A/B comparisons. Run the same task corpus on two branches — one with skills active, one without — and compare: tokens in, tokens out, interactions, tool calls, and task success rate.

## A/B Setup

```
branch A (with-skills):    skills active, MCP server running
branch B (without-skills): no skills, no MCP, raw agent
```

Both branches run the same task corpus (`tasks/sample.yaml`). Results go to `results/`.

## Task Corpus Format (`tasks/sample.yaml`)

```yaml
tasks:
  - id: orient-new-repo
    description: "Orient yourself on this codebase and identify entry points"
    success_criteria: "Correctly identifies 3+ entry points within 5 interactions"
    category: orientation

  - id: add-endpoint
    description: "Add a POST /users endpoint following existing patterns"
    success_criteria: "Endpoint added, tests pass, follows repo conventions"
    category: feature

  - id: fix-bug
    description: "Fix the validation bug in src/auth/login.ts"
    success_criteria: "Bug fixed, existing tests pass, no regressions"
    category: debugging
```

## Running a Benchmark

**Step 1: Switch to with-skills branch and run**

```bash
git checkout with-skills
sensei status          # verify skills active, MCP running
bun run benchmark      # runs task corpus, writes results/with-skills-<date>.json
```

**Step 2: Switch to without-skills branch and run**

```bash
git checkout without-skills
bun run benchmark      # writes results/without-skills-<date>.json
```

**Step 3: Compare results**

```
call: compare_results({
  a: "results/with-skills-2026-03-01.json",
  b: "results/without-skills-2026-03-01.json"
})
```

## Metrics Captured

| Metric | Description |
|---|---|
| `tokensIn` | Tokens sent to the model per task |
| `tokensOut` | Tokens generated per task |
| `interactions` | Number of back-and-forth turns |
| `toolCalls` | Number of tool invocations |
| `success` | Boolean: did the agent complete the task correctly? |
| `duration` | Wall-clock time in seconds |

## Results Format

```json
{
  "run": "with-skills",
  "date": "2026-03-01",
  "tasks": [
    {
      "id": "orient-new-repo",
      "tokensIn": 1200,
      "tokensOut": 340,
      "interactions": 3,
      "toolCalls": 4,
      "success": true,
      "duration": 18
    }
  ],
  "totals": {
    "tokensIn": 24000,
    "tokensOut": 8200,
    "avgInteractions": 4.2,
    "successRate": 0.87
  }
}
```

## Interpreting Results

- **Token reduction** — primary metric; target >40% reduction with skills
- **Interaction reduction** — fewer turns = faster, cheaper, less error-prone
- **Success rate** — skills should not reduce success rate
- **Tool calls** — higher with skills is expected (offloading to MCP is the point)

Commit result summaries (`.md`) to `results/`. Keep raw JSON local (gitignored).
