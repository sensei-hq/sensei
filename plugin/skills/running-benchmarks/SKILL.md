---
name: running-benchmarks
description: Use when evaluating whether a new skill or workflow change actually reduces token usage and test pass rates — guides you through running `sensei benchmark run` to compare a <acp>-without-sensei branch against a <acp>-with-sensei branch on the sample project board app.
Also use before shipping significant skill changes to verify the token reduction target (>40%) is still being met.
---

# Benchmark Runner

## Overview

Quantify skill value using `sensei benchmark run`. The CLI runs identical feature implementation tasks through an ACP (e.g. Claude Code) on two git branches — one with skills installed, one without — and compares token usage and test pass rates.

## Prerequisites

- ACP CLI installed (e.g. `npm install -g @anthropic-ai/claude-code`)
- Run from inside the sensei repo

## Running a Benchmark

```bash
# Run with default ACP (claude) on bundled sample app
sensei benchmark run

# Specify ACP explicitly
sensei benchmark run --acp claude

# Custom repo with its own tasks and skills
sensei benchmark run --repo /path/to/repo --tasks ./tasks/ --skills ./.claude/skills/

# Verbose: print raw ACP output preview after each task
sensei benchmark run --verbose

# Write results to a specific file
sensei benchmark run --output results/$(date +%Y-%m-%d).json
```

## What It Does

The CLI:

1. Copies `examples/sample/` to a temp dir, git init, initial commit
2. Creates branch `claude-without-sensei`:
   - Runs each task file through `claude --print --output-format stream-json`
   - Parses token usage from the `result` event in the stream-json output
   - Counts `tool_use` blocks in `assistant` events (proxy for file exploration effort)
   - Commits ACP changes, runs `bun test`, records pass/fail counts
3. Returns to main, creates branch `claude-with-sensei`:
   - Copies skills from `examples/sample/skills/` to `.claude/skills/`
   - Runs same tasks, captures metrics
4. Prints comparison table and writes `benchmark-results.json`

## The Sample App (`examples/sample/`)

A TypeScript project board API (Hono + bun:sqlite) with three feature stubs as benchmark tasks:

| Task | Feature | Failing tests |
|------|---------|--------------|
| `feature1.md` | GET/POST /tasks/:id/comments + DELETE /comments/:id | 7 |
| `feature2.md` | Full-text search via SQLite FTS5 (GET /tasks?q=) | 2 |
| `feature3.md` | Bulk status update PATCH /tasks/bulk with transaction | 2 |

## Interpreting Output

```
── Branch: claude-without-sensei ──────────────────────────
  [feature1] 8421in/1203out 14 tools 4 turns → 0/7 tests pass
  [feature2] 8380in/987out 12 tools 3 turns → 14/16 tests pass

── Branch: claude-with-sensei ─────────────────────────────
  [feature1] 2103in/1180out 3 tools 2 turns → 7/7 tests pass
  [feature2] 2091in/961out 2 tools 2 turns → 16/16 tests pass

── Results ─────────────────────────────────────────────────
Task          Base tokens  Sensei tokens   Saved  Tests +/-
────────────────────────────────────────────────────────────
feature1             8421           2103   75.0%         +7
feature2             8380           2091   75.1%         +2
```

Key metrics:
- **Input tokens saved** — primary signal; target >40% with skills loaded
- **Tool calls saved** — fewer reads = ACP relied on skills instead of exploring files
- **Test delta** — positive = ACP produced more correct code on first attempt

## What Makes a Good Benchmark Task

- Uses patterns documented in skills (validation, pagination, DB access, FTS5, transactions)
- Has a failing test suite before implementation — gives an objective pass/fail signal
- Realistic scope: completable in a single ACP session
- The same task file used verbatim in both branches

## Extending the Benchmark

To add a new ACP, implement `AcpRunner` in `packages/cli/src/lib/acp-runner.ts`:

```typescript
export class OpenCodeRunner implements AcpRunner {
  readonly id = "opencode";
  readonly name = "OpenCode";
  async detect(): Promise<boolean> { /* check if opencode is installed */ }
  async runTask(taskPath: string, cwd: string): Promise<AcpSession> { /* spawn opencode */ }
}
```

Then run: `sensei benchmark run --acp opencode`
