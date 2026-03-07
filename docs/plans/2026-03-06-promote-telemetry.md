# Promote + Telemetry Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** After `sensei benchmark doctor` runs, auto-promote the winning strategy to the real docs folder; add `sensei benchmark promote` for user feedback + telemetry; add `sensei serve` as a local report receiver; add `submit_benchmark_report` MCP tool.

**Architecture:** `benchmarkDoctor` picks a winner by combined score (structural + judge), writes its files to `docs/<outputName>/`, and stores a full telemetry report in `results.json`. The optional `promote` command captures user preference and submits the report. `sensei serve` is a Bun HTTP + SQLite server that receives reports.

**Tech Stack:** TypeScript, Bun (built-in `Bun.serve`, `bun:sqlite`), `@clack/prompts`, `@modelcontextprotocol/sdk`, `vitest`

---

### Task 1: Auto-promote winner in `benchmarkDoctor`

**Files:**
- Modify: `packages/sensei/src/commands/benchmark-doctor.ts`
- Modify: `packages/sensei/src/commands/benchmark-doctor.spec.ts`

**Context:**
- `benchmarkDoctor` in `benchmark-doctor.ts` currently scores A/B/C and writes to `results/` but does NOT write to the real target dir
- The target dir is: `join(repoPath, dirname(relative(repoPath, fullInputDir)), outputName)` e.g. `docs/features`
- `results.json` currently has `{ date, input, outputName, strategies, scores, judgeReasoning }`
- We need to add: `autoPromoted`, `userFeedback: null`, `promoted: null`, `report: {...}`

**Step 1: Write the failing test**

In `benchmark-doctor.spec.ts`, add after the existing tests:

```typescript
describe("pickWinner", () => {
  it("returns the strategy with highest combined score", () => {
    // Access via export — add export to benchmark-doctor.ts first
    expect(pickWinner(7, 7, 8, 8, 9, 9)).toBe("c");
  });

  it("breaks ties in favour of a", () => {
    expect(pickWinner(5, 5, 5, 5, 5, 5)).toBe("a");
  });
});
```

Add to imports at top of spec: `import { ..., pickWinner } from "./benchmark-doctor.js";`

**Step 2: Run test to verify it fails**

```bash
cd packages/sensei && bun test src/commands/benchmark-doctor.spec.ts 2>&1 | tail -20
```

Expected: FAIL — `pickWinner is not exported`

**Step 3: Add `pickWinner` export to `benchmark-doctor.ts`**

After the `llmJudge` function and before `// ── Main command ──`, insert:

```typescript
// ── Winner selection ──────────────────────────────────────────────────────────

export function pickWinner(
  structA: number, judgeA: number,
  structB: number, judgeB: number,
  structC: number, judgeC: number,
): "a" | "b" | "c" {
  const scores = [
    { key: "a" as const, score: structA + judgeA },
    { key: "b" as const, score: structB + judgeB },
    { key: "c" as const, score: structC + judgeC },
  ];
  return scores.reduce((best, cur) => cur.score > best.score ? cur : best).key;
}
```

**Step 4: Run test to verify `pickWinner` tests pass**

```bash
cd packages/sensei && bun test src/commands/benchmark-doctor.spec.ts 2>&1 | tail -20
```

Expected: `pickWinner` tests PASS (other tests still pass too)

**Step 5: Add auto-promote + report to `benchmarkDoctor`**

In `benchmark-doctor.ts`, add these imports at the top:

```typescript
import { dirname } from "path";  // add "dirname" to the existing path import
import { cp, rm } from "fs/promises";  // add to existing fs/promises import
```

After the `summary.md` write (after `await writeFile(join(outDir, "summary.md"), summary, "utf-8");`), add:

```typescript
  // ── Auto-promote winner ─────────────────────────────────────────────────────
  const winner = pickWinner(structA, judge.scoreA, structB, judge.scoreB, structC, judge.scoreC);
  const winnerOutput = winner === "a" ? outputA : winner === "b" ? outputB : outputC;
  const relInput = relative(repoPath, fullInputDir);
  const targetDir = join(repoPath, dirname(relInput), outputName);

  await mkdir(targetDir, { recursive: true });
  for (const [name, content] of Object.entries(winnerOutput)) {
    await writeFile(join(targetDir, name), content, "utf-8");
  }

  // ── Build telemetry report ──────────────────────────────────────────────────
  const promptA = buildTargetedIndexPrompt({ inputDir: fullInputDir, repoPath, templateContent, outputName });
  const promptB = buildRawContentPrompt({ inputDir: fullInputDir, examplesDir, outputName });
  const promptC = buildFullRepoIndexPrompt({ repoPath, templateContent, outputName });

  function cleanPaths(s: string): string {
    return s.replaceAll(repoPath + "/", "").replaceAll(repoPath, "");
  }

  const exFiles = examplesDir && existsSync(examplesDir)
    ? readdirSync(examplesDir).filter(f => extname(f) === ".md")
    : [];
  const inputLines = Object.values(original).join("\n").split("\n").length;
  const inputChars = Object.values(original).join("").length;

  const report = {
    id: crypto.randomUUID(),
    timestamp: new Date().toISOString(),
    scenario: {
      inputFileCount: Object.keys(original).length,
      inputTotalChars: inputChars,
      inputTotalLines: inputLines,
      outputName,
      templateName: relative(repoPath, templatePath),
      examplesProvided: examplesDir !== null,
      examplesFileCount: exFiles.length,
    },
    strategies: {
      a: { name: "Targeted index",  prompt: cleanPaths(promptA), promptChars: promptA.length, promptLines: promptA.split("\n").length },
      b: { name: "Raw content",     prompt: cleanPaths(promptB), promptChars: promptB.length, promptLines: promptB.split("\n").length },
      c: { name: "Full repo index", prompt: cleanPaths(promptC), promptChars: promptC.length, promptLines: promptC.split("\n").length },
    },
    results: {
      a: { tokensIn: resultA.usage.tokensIn, tokensOut: resultA.usage.tokensOut, filesGenerated: Object.keys(outputA).length, structuralScore: structA, judgeScore: judge.scoreA },
      b: { tokensIn: resultB.usage.tokensIn, tokensOut: resultB.usage.tokensOut, filesGenerated: Object.keys(outputB).length, structuralScore: structB, judgeScore: judge.scoreB },
      c: { tokensIn: resultC.usage.tokensIn, tokensOut: resultC.usage.tokensOut, filesGenerated: Object.keys(outputC).length, structuralScore: structC, judgeScore: judge.scoreC },
    },
    userFeedback: null as null | { preferred: string; systemAgreed: boolean; note?: string },
    promoted: null as null | string,
  };

  const fullResults = {
    ...results,
    autoPromoted: winner,
    userFeedback: null,
    promoted: null,
    report,
  };

  await writeFile(join(outDir, "results.json"), JSON.stringify(fullResults, null, 2), "utf-8");
```

Also remove the earlier `await writeFile(join(outDir, "results.json"), ...)` (the one that wrote `results` without `autoPromoted`).

Update the `note()` call at the bottom to add:

```typescript
  note(
    `Results: ${relative(repoPath, outDir)}/\n` +
    `A: struct=${structA} judge=${judge.scoreA} tokens=${resultA.usage.tokensIn}→${resultA.usage.tokensOut}\n` +
    `B: struct=${structB} judge=${judge.scoreB} tokens=${resultB.usage.tokensIn}→${resultB.usage.tokensOut}\n` +
    `C: struct=${structC} judge=${judge.scoreC} tokens=${resultC.usage.tokensIn}→${resultC.usage.tokensOut}\n` +
    `Auto-promoted: Strategy ${winner.toUpperCase()} → ${relative(repoPath, targetDir)}/`,
    "Benchmark complete"
  );
```

**Step 6: Run all benchmark-doctor tests**

```bash
cd packages/sensei && bun test src/commands/benchmark-doctor.spec.ts 2>&1 | tail -20
```

Expected: All tests PASS

**Step 7: Commit**

```bash
cd packages/sensei && git add src/commands/benchmark-doctor.ts src/commands/benchmark-doctor.spec.ts
git commit -m "feat: auto-promote winner + telemetry report in benchmarkDoctor"
```

---

### Task 2: `sensei benchmark promote` command

**Files:**
- Create: `packages/sensei/src/commands/benchmark-promote.ts`
- Create: `packages/sensei/src/commands/benchmark-promote.spec.ts`

**Context:**
- Reads `results.json` from a results dir (e.g. `results/benchmark-doctor-2026-03-06/`)
- Shows scores, prompts for preferred strategy + optional note
- Copies files if choice differs from `autoPromoted`
- Updates `results.json` with `userFeedback` + `promoted` + fills `report.userFeedback` + `report.promoted`
- Submits to `SENSEI_TELEMETRY_URL` (default `http://localhost:7744`) — fire-and-forget
- Uses `@clack/prompts`: `select`, `text`, `isCancel`, `intro`, `outro`, `note`

**Step 1: Write the failing test**

Create `packages/sensei/src/commands/benchmark-promote.spec.ts`:

```typescript
import { describe, it, expect, vi, beforeEach } from "vitest";
import { pickPreferred, buildFeedback, submitReport } from "./benchmark-promote.js";

describe("pickPreferred", () => {
  it("identifies winner from scores", () => {
    const scores = {
      a: { structuralScore: 7, judgeScore: 7 },
      b: { structuralScore: 8, judgeScore: 8 },
      c: { structuralScore: 9, judgeScore: 9 },
    };
    expect(pickPreferred(scores)).toBe("c");
  });
});

describe("buildFeedback", () => {
  it("sets systemAgreed true when user matches auto", () => {
    const fb = buildFeedback("c", "c", "great output");
    expect(fb).toEqual({ preferred: "c", systemAgreed: true, note: "great output" });
  });

  it("sets systemAgreed false when user overrides", () => {
    const fb = buildFeedback("b", "c");
    expect(fb).toEqual({ preferred: "b", systemAgreed: false });
  });
});

describe("submitReport", () => {
  beforeEach(() => {
    global.fetch = vi.fn().mockResolvedValue({ json: async () => ({ ok: true }) });
  });

  it("POSTs to telemetry URL", async () => {
    await submitReport({ id: "test-id" }, "http://localhost:7744");
    expect(global.fetch).toHaveBeenCalledWith(
      "http://localhost:7744/reports",
      expect.objectContaining({ method: "POST" }),
    );
  });

  it("does not throw when server is down", async () => {
    global.fetch = vi.fn().mockRejectedValue(new Error("ECONNREFUSED"));
    await expect(submitReport({ id: "test-id" }, "http://localhost:7744")).resolves.not.toThrow();
  });
});
```

**Step 2: Run test to verify it fails**

```bash
cd packages/sensei && bun test src/commands/benchmark-promote.spec.ts 2>&1 | tail -20
```

Expected: FAIL — module not found

**Step 3: Create `benchmark-promote.ts`**

Create `packages/sensei/src/commands/benchmark-promote.ts`:

```typescript
import { readFile, writeFile, mkdir } from "fs/promises";
import { existsSync } from "fs";
import { join, dirname, relative } from "path";
import { intro, outro, select, text, isCancel, note, log } from "@clack/prompts";

// ── Pure helpers (exported for testing) ────────────────────────────────────────

export function pickPreferred(scores: Record<string, { structuralScore: number; judgeScore: number }>): string {
  return Object.entries(scores)
    .map(([k, v]) => ({ k, score: v.structuralScore + v.judgeScore }))
    .reduce((best, cur) => cur.score > best.score ? cur : best).k;
}

export function buildFeedback(
  preferred: string,
  autoPromoted: string,
  noteText?: string,
): { preferred: string; systemAgreed: boolean; note?: string } {
  const fb: { preferred: string; systemAgreed: boolean; note?: string } = {
    preferred,
    systemAgreed: preferred === autoPromoted,
  };
  if (noteText) fb.note = noteText;
  return fb;
}

export async function submitReport(report: unknown, baseUrl: string): Promise<void> {
  try {
    await fetch(`${baseUrl}/reports`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(report),
    });
  } catch {
    // Telemetry is best-effort — never throw
  }
}

// ── Main command ────────────────────────────────────────────────────────────────

export async function benchmarkPromote(resultsDir: string, repoPath: string): Promise<void> {
  intro("sensei benchmark promote");

  const resultsPath = join(repoPath, resultsDir, "results.json");
  if (!existsSync(resultsPath)) {
    log.error(`results.json not found: ${resultsPath}`);
    outro("Aborted.");
    return;
  }

  const data = JSON.parse(await readFile(resultsPath, "utf-8"));
  const { scores, autoPromoted, outputName, input } = data;

  // Display comparison table
  note(
    Object.entries(scores as Record<string, { structuralScore: number; judgeScore: number; tokensIn: number; filesGenerated: number }>)
      .map(([k, v]) => {
        const marker = k === autoPromoted ? " ← auto" : "";
        return `Strategy ${k.toUpperCase()}: struct=${v.structuralScore} judge=${(data.scores[k] as { judgeScore: number }).judgeScore} tokens=${v.tokensIn} files=${v.filesGenerated}${marker}`;
      })
      .join("\n"),
    "Benchmark results"
  );

  const preferred = await select({
    message: "Which strategy do you prefer?",
    options: [
      { value: "a", label: `A — Targeted index (struct=${scores.a.structuralScore} judge=${scores.a.judgeScore})` },
      { value: "b", label: `B — Raw content    (struct=${scores.b.structuralScore} judge=${scores.b.judgeScore})` },
      { value: "c", label: `C — Full repo index(struct=${scores.c.structuralScore} judge=${scores.c.judgeScore})` },
    ],
  });
  if (isCancel(preferred)) { outro("Cancelled."); return; }

  const noteText = await text({ message: "Optional note (press Enter to skip):", placeholder: "" });
  if (isCancel(noteText)) { outro("Cancelled."); return; }

  // Copy files if user's choice differs from auto-promoted
  if (preferred !== autoPromoted) {
    const srcDir = join(repoPath, resultsDir, preferred as string, outputName);
    const relInput = relative(repoPath, join(repoPath, input));
    const targetDir = join(repoPath, dirname(relInput), outputName);
    await mkdir(targetDir, { recursive: true });

    const { readdirSync, readFileSync } = await import("fs");
    for (const f of readdirSync(srcDir)) {
      const content = readFileSync(join(srcDir, f), "utf-8");
      await writeFile(join(targetDir, f), content, "utf-8");
    }
    log.success(`Copied Strategy ${(preferred as string).toUpperCase()} → ${relative(repoPath, targetDir)}/`);
  }

  // Update results.json
  const feedback = buildFeedback(preferred as string, autoPromoted, noteText as string || undefined);
  data.userFeedback = feedback;
  data.promoted = preferred;
  if (data.report) {
    data.report.userFeedback = feedback;
    data.report.promoted = preferred;
  }
  await writeFile(resultsPath, JSON.stringify(data, null, 2), "utf-8");

  // Submit telemetry (fire-and-forget)
  const telemetryUrl = process.env.SENSEI_TELEMETRY_URL ?? "http://localhost:7744";
  submitReport(data.report ?? data, telemetryUrl).catch(() => {});

  note(`Promoted: Strategy ${(preferred as string).toUpperCase()}`, "Done");
  outro("Done.");
}
```

**Step 4: Run tests to verify they pass**

```bash
cd packages/sensei && bun test src/commands/benchmark-promote.spec.ts 2>&1 | tail -20
```

Expected: All 4 tests PASS

**Step 5: Commit**

```bash
cd packages/sensei && git add src/commands/benchmark-promote.ts src/commands/benchmark-promote.spec.ts
git commit -m "feat: add sensei benchmark promote command"
```

---

### Task 3: `sensei serve` — local HTTP + SQLite server

**Files:**
- Create: `packages/sensei/src/commands/serve.ts`
- Create: `packages/sensei/src/commands/serve.spec.ts`

**Context:**
- Uses `Bun.serve()` and `bun:sqlite` (Bun built-ins, zero new deps)
- Default port `7744`, default DB `.sensei/reports.db`
- `POST /reports` → inserts row, returns `{ ok: true, id }`
- `GET /health` → returns `{ ok: true }`
- Creates the DB directory if it doesn't exist

**Step 1: Write the failing test**

Create `packages/sensei/src/commands/serve.spec.ts`:

```typescript
import { describe, it, expect, afterAll } from "vitest";
import { createReportServer } from "./serve.js";
import { tmpdir } from "os";
import { join } from "path";

const PORT = 17744; // non-default to avoid conflicts
const DB_PATH = join(tmpdir(), `sensei-test-${Date.now()}.db`);

describe("createReportServer", () => {
  let server: { stop: () => void };

  afterAll(() => server?.stop());

  it("returns health ok", async () => {
    server = await createReportServer({ port: PORT, dbPath: DB_PATH });
    const res = await fetch(`http://localhost:${PORT}/health`);
    const body = await res.json();
    expect(body).toEqual({ ok: true });
  });

  it("accepts POST /reports and returns id", async () => {
    const report = { id: "test-123", timestamp: "2026-03-06T00:00:00Z", scenario: {} };
    const res = await fetch(`http://localhost:${PORT}/reports`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(report),
    });
    const body = await res.json();
    expect(body.ok).toBe(true);
    expect(body.id).toBe("test-123");
  });

  it("returns 404 for unknown routes", async () => {
    const res = await fetch(`http://localhost:${PORT}/unknown`);
    expect(res.status).toBe(404);
  });
});
```

**Step 2: Run test to verify it fails**

```bash
cd packages/sensei && bun test src/commands/serve.spec.ts 2>&1 | tail -20
```

Expected: FAIL — `createReportServer is not exported`

**Step 3: Create `serve.ts`**

Create `packages/sensei/src/commands/serve.ts`:

```typescript
import { Database } from "bun:sqlite";
import { mkdir } from "fs/promises";
import { dirname } from "path";

export interface ServeOptions {
  port?: number;
  dbPath?: string;
}

export async function createReportServer(opts: ServeOptions = {}): Promise<{ stop: () => void }> {
  const port = opts.port ?? 7744;
  const dbPath = opts.dbPath ?? ".sensei/reports.db";

  await mkdir(dirname(dbPath), { recursive: true });

  const db = new Database(dbPath);
  db.run(`
    CREATE TABLE IF NOT EXISTS reports (
      id TEXT PRIMARY KEY,
      timestamp TEXT NOT NULL,
      payload TEXT NOT NULL
    )
  `);

  const server = Bun.serve({
    port,
    async fetch(req) {
      const url = new URL(req.url);

      if (req.method === "GET" && url.pathname === "/health") {
        return Response.json({ ok: true });
      }

      if (req.method === "POST" && url.pathname === "/reports") {
        try {
          const body = await req.json() as Record<string, unknown>;
          const id = (body.id as string) ?? crypto.randomUUID();
          const timestamp = (body.timestamp as string) ?? new Date().toISOString();
          db.run(
            "INSERT OR REPLACE INTO reports (id, timestamp, payload) VALUES (?, ?, ?)",
            [id, timestamp, JSON.stringify(body)],
          );
          return Response.json({ ok: true, id });
        } catch {
          return Response.json({ ok: false, error: "Invalid JSON" }, { status: 400 });
        }
      }

      return Response.json({ ok: false, error: "Not found" }, { status: 404 });
    },
  });

  return { stop: () => server.stop() };
}

export async function serve(repoPath: string, opts: { port?: number; db?: string }): Promise<void> {
  const port = opts.port ?? parseInt(process.env.SENSEI_PORT ?? "7744", 10);
  const dbPath = opts.db ?? process.env.SENSEI_DB ?? `${repoPath}/.sensei/reports.db`;

  console.log(`sensei serve starting on :${port}`);
  console.log(`Database: ${dbPath}`);

  await createReportServer({ port, dbPath });
  // Keep process alive
  await new Promise(() => {});
}
```

**Step 4: Run tests to verify they pass**

```bash
cd packages/sensei && bun test src/commands/serve.spec.ts 2>&1 | tail -20
```

Expected: All 3 tests PASS

**Step 5: Commit**

```bash
cd packages/sensei && git add src/commands/serve.ts src/commands/serve.spec.ts
git commit -m "feat: add sensei serve (Bun HTTP + SQLite report receiver)"
```

---

### Task 4: Wire CLI + add MCP tool

**Files:**
- Modify: `packages/sensei/src/cli.ts`
- Modify: `packages/sensei/src/index.ts`

**Context:**
- `cli.ts` needs `benchmark promote` subcommand and `serve` command
- `cli.ts` parseArgs needs `port` (string) and `db` (string) options
- `index.ts` needs `submit_benchmark_report` MCP tool

**Step 1: Add `port` and `db` to `parseArgs` in `cli.ts`**

In the `parseArgs` options block (lines 7–15), add:
```typescript
    port: { type: "string" },
    db: { type: "string" },
```

**Step 2: Add `benchmark promote` handling**

In the `case "benchmark":` block, after the `if (subCmd === "doctor")` block, add:

```typescript
      } else if (subCmd === "promote") {
        const { benchmarkPromote } = await import("./commands/benchmark-promote.js");
        const resultsDir = rest[1];
        if (!resultsDir) {
          console.error("Usage: sensei benchmark promote <results-dir>");
          process.exit(1);
        }
        await benchmarkPromote(resultsDir, process.cwd());
```

**Step 3: Add `serve` command**

After the `case "migrate":` block and before `case "benchmark":`, add:

```typescript
    case "serve": {
      const { serve } = await import("./commands/serve.js");
      await serve(process.cwd(), {
        port: values.port ? parseInt(values.port, 10) : undefined,
        db: values.db,
      });
      break;
    }
```

**Step 4: Update help text**

In the default case help text, add:
```
  sensei benchmark promote <results-dir>
                                Review benchmark, capture feedback, submit telemetry
  sensei serve [--port 7744] [--db <path>]
                                Start local telemetry report receiver
```

**Step 5: Add MCP tool to `index.ts`**

At the end of `index.ts` (before `const transport = new StdioServerTransport();`), add:

```typescript
// Telemetry tool
server.tool("submit_benchmark_report",
  "Submit an anonymous benchmark report to the sensei telemetry endpoint. Reads SENSEI_TELEMETRY_URL env var (default: http://localhost:7744).",
  { report: z.record(z.unknown()) },
  async ({ report }) => {
    const url = process.env.SENSEI_TELEMETRY_URL ?? "http://localhost:7744";
    try {
      const res = await fetch(`${url}/reports`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(report),
      });
      const data = await res.json();
      return { content: [{ type: "text", text: `Report submitted: id=${(data as { id?: string }).id ?? "?"}` }] };
    } catch (err) {
      return { content: [{ type: "text", text: `Telemetry unavailable (is sensei serve running?): ${(err as Error).message}` }] };
    }
  }
);
```

**Step 6: Build and run all tests**

```bash
cd packages/sensei && bun run build 2>&1 | tail -5
bun test 2>&1 | tail -20
```

Expected: Build succeeds, all tests PASS

**Step 7: Smoke test CLI**

```bash
sensei 2>&1 | grep -E "promote|serve"
```

Expected: Both commands appear in help output

**Step 8: Commit**

```bash
cd packages/sensei && git add src/cli.ts src/index.ts
git commit -m "feat: wire benchmark promote + serve commands; add submit_benchmark_report MCP tool"
```

---

### Task 5: End-to-end smoke test on strategos

**Context:**
This task verifies the full flow works on a real repo. No code changes — just running the commands.

**Step 1: Build sensei**

```bash
cd /Users/Jerry/Developer/skills/packages/sensei && bun run build
```

**Step 2: Run benchmark on strategos (3-file sample)**

```bash
cd /Users/Jerry/Developer/strategos
sensei benchmark doctor docs/requirements features \
  --template docs/templates/feature.md \
  --examples docs/features/ \
  --sample 3
```

Expected:
- 3 strategy runs complete
- `docs/features/` is populated with winner's output
- `results/benchmark-doctor-<date>/results.json` exists with `autoPromoted` set
- `results/benchmark-doctor-<date>/summary.md` mentions auto-promoted strategy

**Step 3: Verify target files exist**

```bash
ls /Users/Jerry/Developer/strategos/docs/features/
```

Expected: README.md + feature files from winning strategy

**Step 4: Run promote**

```bash
cd /Users/Jerry/Developer/strategos
sensei benchmark promote results/benchmark-doctor-$(date +%Y-%m-%d)/
```

Expected: Interactive prompts show, user can select preferred strategy, `results.json` updated with `userFeedback`

**Step 5: Start serve and test telemetry**

```bash
# Terminal 1
sensei serve &

# Terminal 2
curl -s http://localhost:7744/health
```

Expected: `{"ok":true}`

**Step 6: No commit needed** — this is a verification-only task
