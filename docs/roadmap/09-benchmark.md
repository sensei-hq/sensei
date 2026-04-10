# Sensei Benchmark — Measuring Real-World Value

> **Status:** Implemented (`sensei benchmark run`) — see `examples/sample/` and
> `packages/cli/src/commands/benchmark-run.ts`.

---

## Why this exists

Sensei's core claim is that skills and indexed context make coding agents cheaper and more
accurate. The benchmark exists to **prove that claim with hard numbers**, not marketing copy.

The metrics that matter:

| Signal | What it reveals |
|--------|----------------|
| **Input tokens** | How much context the ACP had to consume per task |
| **Tool calls** | How much file exploration the ACP had to do |
| **Test pass rate** | Whether the ACP produced correct implementations |
| **Turns** | How many round-trips the ACP needed |

Without sensei, the ACP explores the codebase by reading files one by one. Each file read
costs tokens. With sensei skills loaded, the ACP already knows the patterns, types, and
conventions — fewer reads, lower cost, better first attempts.

---

## Architecture

```
sensei benchmark run --acp claude

  1. Copy examples/sample/ → temp dir, git init, commit "initial state"

  2. Branch: claude-without-sensei
     ├─ install Stop hook → .claude/settings.json (real-world session capture)
     ├─ for each task file (feature1.md, feature2.md, feature3.md):
     │   ├─ claude --print --output-format stream-json < feature.md
     │   ├─ parse stream-json result event for input_tokens, output_tokens,
     │   │  num_turns, tool_call count
     │   ├─ git commit the ACP's changes
     │   └─ bun test → parse pass/fail counts
     └─ collect all session metrics

  3. git checkout main → Branch: claude-with-sensei
     ├─ copy examples/sample/skills/*.md → .claude/skills/
     ├─ same Stop hook
     ├─ same three task files, same order
     └─ collect session metrics

  4. Compare branches → print table → write benchmark-results.json
```

### Token capture

Claude Code's `--output-format stream-json` emits newline-delimited JSON events.
The final `result` event contains exact token counts from the Anthropic API:

```json
{
  "type": "result",
  "num_turns": 4,
  "usage": {
    "input_tokens": 8421,
    "output_tokens": 1203,
    "cache_creation_input_tokens": 0,
    "cache_read_input_tokens": 0
  }
}
```

Tool calls are counted from `assistant` message events with `type: "tool_use"` content blocks.
This gives per-session: reads, writes, bash calls — a proxy for codebase exploration effort.

### Quality capture

After each task, `bun test` runs the full test suite. The output is parsed for
`N pass` / `N fail` lines. The delta between branches (baseline vs. sensei) is the
quality signal: did sensei help the ACP produce more correct code on the first attempt?

---

## The sample app: `examples/sample/`

A TypeScript project board API (Hono + bun:sqlite) chosen for these properties:

- **Pattern-rich** — consistent validation, pagination, error handling, and DB access patterns
  across all routes. Skills can meaningfully summarise these patterns.
- **Deliberately incomplete** — three features left as 501 stubs, each pointing to a task file.
  The test suite has failing tests for each stub.
- **Objectively testable** — `bun test` gives a definitive pass/fail count.
- **Realistic scope** — each feature takes a single ACP session (not trivial, not enormous).

### Structure

```
examples/sample/
  src/
    db.ts               — bun:sqlite singleton, schema + FTS5 triggers
    types.ts            — Zod schemas for all entities
    utils/
      errors.ts         — AppError, notFound(), badRequest(), errorHandler()
      pagination.ts     — cursor pagination: decodeCursor, encodeCursor, toPage<T>
      validate.ts       — validate(schema) middleware, parseQuery() helper
    routes/
      projects.ts       — /projects CRUD + /projects/:id/tasks  [complete]
      tasks.ts          — /tasks CRUD + stubs for search, bulk, comments  [partial]
      comments.ts       — DELETE /comments/:id stub  [stub]
    index.ts            — app entry, route mounting, error handler
  tests/
    projects.test.ts    — all passing
    tasks.test.ts       — CRUD passing; search + bulk tests failing (stubs)
    comments.test.ts    — all failing (stub)
  tasks/                — feature implementation tasks (given to the ACP)
    feature1.md
    feature2.md
    feature3.md
  skills/               — pre-built sensei skills for this repo
    architecture.md
    patterns.md
```

---

## Scenarios

### Scenario 1 — Task Comments (`feature1.md`)

**What the ACP must implement:**
- `GET /tasks/:id/comments` — paginated comment list (404 if task missing)
- `POST /tasks/:id/comments` — create comment with validation (404 if task missing, 201 on success)
- `DELETE /comments/:id` — delete comment (404 if missing, 204 on success)

**Why skills help:**
The task references `CreateCommentSchema`, `validate()`, `notFound()`, `toPage()`,
and `getDb()`. Without skills, the ACP must find these by reading 4–6 source files.
With skills, the patterns are available immediately in `.claude/skills/`.

**Failing tests before implementation:** 7 (all of `tests/comments.test.ts`)

**Expected token delta:** ~70–80% input token reduction (no file exploration needed).

---

### Scenario 2 — Full-Text Search (`feature2.md`)

**What the ACP must implement:**
Extend `GET /tasks` to support `?q=` via SQLite FTS5. The `tasks_fts` virtual table
and its sync triggers already exist in `src/db.ts`. The ACP must:
1. Remove the 501 guard in the `GET /tasks` handler
2. Query `tasks_fts` via `MATCH` when `?q=` is present
3. Join results back to `tasks` for the full row, preserving relevance order
4. Combine with existing status/priority/assignee filters

**Why skills help:**
The FTS5 query pattern and the DB access convention (parameterised queries, `getDb()`,
never `new Database()`) are non-obvious from the route file alone. Skills document both.

**Failing tests before implementation:** 2 (the `GET /tasks?q=` tests in `tasks.test.ts`)

**Expected token delta:** ~70% (ACP needs only the skills context, not a full file scan).

---

### Scenario 3 — Bulk Status Update (`feature3.md`)

**What the ACP must implement:**
`PATCH /tasks/bulk` — body `{ ids: string[], status: TaskStatus }` — updates all matching
tasks in a single SQLite transaction. Missing IDs are collected (not thrown) and returned
in the response as `{ updated: number, notFound: string[] }`.

**Why skills help:**
Two non-obvious patterns are required: the SQLite transaction idiom (`db.transaction(fn)`)
and the `COALESCE` update pattern. Without skills, the ACP typically implements one of:
- individual queries without a transaction (wrong — not atomic)
- throwing on the first missing ID (wrong — spec says collect them)

Skills document both patterns explicitly.

**Failing tests before implementation:** 2 (the `PATCH /tasks/bulk` tests in `tasks.test.ts`)

**Expected token delta:** ~70%; quality delta likely larger (ACP gets the idiom right first try).

---

## Expected benchmark output

```
── Branch: claude-without-sensei ──────────────────────────
  [feature1] 8421in/1203out 14 tools 4 turns → 0/7 tests pass
  [feature2] 8380in/987out 12 tools 3 turns → 14/16 tests pass
  [feature3] 8395in/1102out 13 tools 4 turns → 14/16 tests pass

── Branch: claude-with-sensei ──────────────────────────────
  [feature1] 2103in/1180out 3 tools 2 turns → 7/7 tests pass
  [feature2] 2091in/961out 2 tools 2 turns → 16/16 tests pass
  [feature3] 2098in/1089out 2 tools 2 turns → 16/16 tests pass

── Results ─────────────────────────────────────────────────────────
Task                   Base tokens  Sensei tokens   Saved  Tests +/-
────────────────────────────────────────────────────────────────────
feature1                      8421           2103   75.0%         +7
feature2                      8380           2091   75.1%         +2
feature3                      8395           2098   75.0%         +2
────────────────────────────────────────────────────────────────────
TOTAL                        25196           6292   75.0%        +11

  input tokens saved : 18,904 (75.0%)
  tool calls saved   : 32
  extra tests passing: +11
```

Token savings come from the skills replacing file reads. Test pass rate improvements come
from the ACP applying established patterns correctly on the first attempt.

---

## Adding more ACPs

The runner is pluggable. To add Cursor or opencode:

```typescript
// packages/cli/src/lib/acp-runner.ts

export class OpenCodeRunner implements AcpRunner {
  readonly id = "opencode";
  readonly name = "OpenCode";

  async detect(): Promise<boolean> {
    return whichExists("opencode");
  }

  async runTask(taskPath: string, cwd: string): Promise<AcpSession> {
    const prompt = await readFile(taskPath, "utf-8");
    // opencode uses a different CLI interface — adjust flags as needed
    const { stdout, exitCode } = await spawnCapture(
      ["opencode", "run", "--non-interactive"],
      { cwd, stdin: prompt },
    );
    return parseOpenCodeOutput(stdout, exitCode);
  }
}

// Register it:
const RUNNERS: AcpRunner[] = [
  new ClaudeRunner(),
  new OpenCodeRunner(),
];
```

Then run:
```sh
sensei benchmark run --acp opencode
```

Each ACP's output format differs — implement a parser specific to its output.
Claude's `--output-format stream-json` is the most structured.

---

## Future scenarios

| Scenario | What it tests |
|----------|--------------|
| **Multi-session task** | A feature that requires 3+ back-and-forth turns — measures turn reduction |
| **Bug fix with test** | ACP must find and fix a bug that has a failing test — measures diagnosis accuracy |
| **Cross-file refactor** | Rename a pattern across 5+ files — measures global context benefit |
| **Library integration** | Add a new dependency following existing integration patterns — measures lib skill value |
| **Custom repo** | `sensei benchmark run --repo ./my-project` — run against your own codebase |

---

## Usage

```sh
# Run with default ACP (claude) on bundled sample
sensei benchmark run

# Specify ACP
sensei benchmark run --acp claude

# Custom repo with its own tasks and skills
sensei benchmark run --repo /path/to/repo --tasks ./tasks/ --skills ./.claude/skills/

# Verbose: print raw ACP output preview after each task
sensei benchmark run --verbose

# Write results to a specific file
sensei benchmark run --output results/$(date +%Y-%m-%d).json
```

Results are written as `benchmark-results.json` with full per-task detail:
session metrics, test counts, and comparison deltas across both branches.
