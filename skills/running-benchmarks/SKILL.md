---
name: running-benchmarks
description: Use when evaluating whether a new skill or workflow change actually
reduces token usage and interaction counts — runs A/B comparisons on a task corpus
(with-skills vs without-skills branches) and produces a metrics report to confirm
or reject the improvement.
Also use before shipping significant skill changes to verify the token reduction
target (>40%) is still being met.
---

# Benchmark Runner

## Overview

Quantify skill value with A/B comparisons. Run the same task corpus on two branches — one with skills active, one without — and compare: tokens in, tokens out, interactions, tool calls, and task success rate.

## Benchmark Pair (with vs. without sensei)

Use this for a direct cost comparison on a single task using OTel telemetry. Results appear automatically in the repo's Analytics page under "Benchmark Comparison".

### Prerequisites

1. `CLAUDE_CODE_ENABLE_TELEMETRY=1` must be set in the shell environment
2. The sensei MCP server must be running (it hosts the OTLP endpoint on port 4318)
3. `SENSEI_OTEL_DRY_RUN` must be unset or `false` so costs are written to Supabase

### Procedure

**Step 1: Create a benchmark worktree**

```bash
SLUG=<short-task-name>
STAMP=$(date +%Y%m%d-%H%M)
git worktree add .worktrees/benchmark-${SLUG}-${STAMP} -b benchmark/${SLUG}-${STAMP}
cd .worktrees/benchmark-${SLUG}-${STAMP}
```

**Step 2: Run WITHOUT sensei**

1. Back up and strip sensei from the worktree's `.mcp.json`:
   ```bash
   cp .mcp.json .mcp.json.sensei-backup
   # Remove the sensei server entry from .mcp.json
   ```
2. Export OTel env vars:
   ```bash
   export CLAUDE_CODE_ENABLE_TELEMETRY=1
   export OTEL_METRICS_EXPORTER=otlp
   export OTEL_EXPORTER_OTLP_ENDPOINT=http://localhost:4318
   export SENSEI_OTEL_DRY_RUN=false
   ```
3. Insert a `benchmark_runs` row (sensei_enabled: false) — note the returned `id` as `$NO_SENSEI_RUN_ID`
4. Run the task with Claude Code using the exact task prompt
5. After completion: update the `benchmark_runs` row with `ended_at` and aggregate totals from `api_requests` rows where `recorded_at` is between `started_at` and `ended_at`

**Step 3: Run WITH sensei**

1. Restore `.mcp.json`:
   ```bash
   cp .mcp.json.sensei-backup .mcp.json && rm .mcp.json.sensei-backup
   ```
2. Same OTel env vars (already exported)
3. Insert a `benchmark_runs` row (sensei_enabled: true) — note the `id` as `$WITH_SENSEI_RUN_ID`
4. Run the exact same task prompt with Claude Code
5. Update the `benchmark_runs` row with `ended_at` and aggregate totals

**Step 4: Clean up**

```bash
cd ../..
git worktree remove .worktrees/benchmark-${SLUG}-${STAMP}
git branch -d benchmark/${SLUG}-${STAMP}
```

**Step 5: View results**

Open the repo's Analytics page in the sensei dashboard. The "Benchmark Comparison" table shows the paired runs side-by-side with cost delta and savings percentage. Pairs are matched by `(task_description, branch)`.

### What makes a good benchmark task

- Identical prompt for both runs — copy it verbatim, do not paraphrase
- Same branch/codebase state (hence the worktree approach)
- Task genuinely exercises sensei context tools (`context_pack`, `search`, `get_session_context`) — otherwise savings will be minimal
- Tasks in the 5–20 tool call range show the clearest variance; single-tool tasks are too noisy

### Environment variables reference

| Variable | Value | Purpose |
|---|---|---|
| `CLAUDE_CODE_ENABLE_TELEMETRY` | `1` | Enables OTel emission from Claude Code |
| `OTEL_METRICS_EXPORTER` | `otlp` | Routes metrics to OTLP HTTP |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://localhost:4318` | Points to sensei's OTLP endpoint |
| `SENSEI_OTEL_DRY_RUN` | `false` (or unset) | Enables DB writes; set to `true` to verify event parsing only |
| `SENSEI_OTEL_PORT` | `4318` (default) | Override OTLP port if 4318 is taken |

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
