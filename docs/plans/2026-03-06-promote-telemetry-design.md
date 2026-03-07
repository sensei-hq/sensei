# Promote + Telemetry Design

## Overview

After `sensei benchmark doctor` completes, the user always gets usable output — the winning strategy is auto-promoted to the real target directory. `sensei benchmark promote` is optional: it captures user preference, overrides the auto-selection if desired, and submits an anonymous report. `sensei serve` runs a local HTTP + SQLite server that receives reports.

## Auto-Promote (change to `benchmarkDoctor`)

After scoring, `benchmarkDoctor`:
1. Picks the winner: `max(structuralScore + judgeScore)` across A/B/C
2. Writes winner's output files to `docs/<outputName>/` (derived from `dirname(inputDir) + outputName`)
3. Adds to `results.json`:
   - `autoPromoted: "a" | "b" | "c"`
   - `userFeedback: null`
   - `promoted: null`
   - `report: { ... }` — full telemetry payload (see Report Schema)
4. Logs: `"Auto-promoted Strategy C → docs/features/"`

The user always gets docs. Promote is gravy.

## Command Interface

```bash
# Auto-promote runs at end of benchmark (no extra command needed)
sensei benchmark doctor docs/requirements features --examples docs/features/

# Optional: capture feedback, override selection, submit telemetry
sensei benchmark promote results/benchmark-doctor-2026-03-06/

# Optional: receive telemetry reports locally
sensei serve [--port 7744] [--db .sensei/reports.db]
```

## `sensei benchmark promote` Flow

1. Read `results.json` from specified dir
2. Display comparison table (structural, judge, tokens, files per strategy)
3. `select` prompt: "Which strategy do you prefer? A / B / C" (highlights auto-promoted)
4. `text` prompt: "Optional note:" (skip with Enter)
5. If chosen ≠ `autoPromoted`: copy chosen files over target, delete previous
6. Update `results.json`: `userFeedback`, `promoted`, `report.userFeedback`, `report.promoted`
7. Submit telemetry via `fetch` to `SENSEI_TELEMETRY_URL` (default: `http://localhost:7744`) — fire-and-forget, silent on failure

## `sensei serve` Design

- `Bun.serve()` HTTP server (no framework)
- `bun:sqlite` for storage (Bun built-in, no extra deps)
- Default port: `7744`, default DB: `.sensei/reports.db`
- Config: `--port`, `--db` flags or `SENSEI_PORT` / `SENSEI_DB` env vars

Routes:
- `GET /health` → `{ ok: true }`
- `POST /reports` → insert row, return `{ ok: true, id }`

SQLite schema:
```sql
CREATE TABLE IF NOT EXISTS reports (
  id TEXT PRIMARY KEY,
  timestamp TEXT NOT NULL,
  payload TEXT NOT NULL
);
```

## MCP Tool: `submit_benchmark_report`

Added to `index.ts` alongside existing tools:

```typescript
server.tool("submit_benchmark_report",
  "Submit an anonymous benchmark report to the sensei telemetry endpoint",
  { report: z.record(z.unknown()) },
  async ({ report }) => {
    const url = process.env.SENSEI_TELEMETRY_URL ?? "http://localhost:7744";
    try {
      const res = await fetch(`${url}/reports`, {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify(report),
      });
      return { content: [{ type: "text", text: `Submitted: ${JSON.stringify(await res.json())}` }] };
    } catch (err) {
      return { content: [{ type: "text", text: `Telemetry unavailable: ${(err as Error).message}` }] };
    }
  }
);
```

## Report Schema

Built by `benchmarkDoctor`, stored under `results.report`, submitted at promote time:

```json
{
  "id": "uuid-v4",
  "timestamp": "2026-03-06T14:00:00Z",
  "scenario": {
    "inputFileCount": 14,
    "inputTotalChars": 24380,
    "inputTotalLines": 3285,
    "outputName": "features",
    "templateName": "feature.md",
    "examplesProvided": true,
    "examplesFileCount": 10
  },
  "strategies": {
    "a": { "name": "Targeted index", "prompt": "...", "promptChars": 1840, "promptLines": 42 },
    "b": { "name": "Raw content",    "prompt": "...", "promptChars": 9200, "promptLines": 310 },
    "c": { "name": "Full repo index","prompt": "...", "promptChars": 5400, "promptLines": 180 }
  },
  "results": {
    "a": { "tokensIn": 480,  "tokensOut": 3200, "filesGenerated": 11, "structuralScore": 7, "judgeScore": 7 },
    "b": { "tokensIn": 2300, "tokensOut": 3500, "filesGenerated": 11, "structuralScore": 8, "judgeScore": 8 },
    "c": { "tokensIn": 1400, "tokensOut": 3400, "filesGenerated": 11, "structuralScore": 9, "judgeScore": 9 }
  },
  "userFeedback": {
    "preferred": "c",
    "systemAgreed": true,
    "note": "C had better Gherkin scenarios"
  },
  "promoted": "c"
}
```

**Path cleaning:** strip everything up to and including the repoPath prefix from all prompt strings before storing in the report.

## Winner Picking

```typescript
function pickWinner(structA, judgeA, structB, judgeB, structC, judgeC): "a" | "b" | "c" {
  const scores = [
    { key: "a", score: structA + judgeA },
    { key: "b", score: structB + judgeB },
    { key: "c", score: structC + judgeC },
  ];
  return scores.reduce((best, cur) => cur.score > best.score ? cur : best).key;
}
```

Ties broken by array order (A wins ties).

## Target Dir Derivation

```typescript
// inputDir = "docs/requirements", outputName = "features"
// → target = "docs/features"
const relInput = relative(repoPath, fullInputDir);      // "docs/requirements"
const targetDir = join(repoPath, dirname(relInput), outputName); // "docs/features"
```

## Implementation

- Modify: `packages/sensei/src/commands/benchmark-doctor.ts`
- Create: `packages/sensei/src/commands/benchmark-promote.ts`
- Create: `packages/sensei/src/commands/serve.ts`
- Modify: `packages/sensei/src/index.ts` (add MCP tool)
- Modify: `packages/sensei/src/cli.ts` (add `benchmark promote` + `serve` commands)
