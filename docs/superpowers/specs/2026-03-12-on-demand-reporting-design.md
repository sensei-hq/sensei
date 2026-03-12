# On-Demand Drift Reporting — Design

**Feature:** `on-demand-reporting` in `docs/traceability.yaml`

## Goal

Make drift results actionable for agents (structured JSON from MCP) and surface drift automatically during watch sessions (`--drift` flag).

## Changes

### 1. Structured MCP output

**File:** `packages/mcp/src/index.ts`

The `check_drift` tool currently returns only `result.summary` (prose string). Change it to return the full `DriftResult` serialized as JSON:

```json
{
  "drifted": [
    {
      "docPath": "docs/design/03-mcp-server.md",
      "reason": "code-changed",
      "changedFiles": ["packages/mcp/src/index.ts"]
    }
  ],
  "summary": "1 doc(s) drifted since a3f8c21:\n...",
  "lastIndexedCommit": "a3f8c21"
}
```

Agents iterate `drifted[]` directly — each entry has `docPath` (what to fix), `reason` (`code-changed` | `doc-changed` | `file-deleted` | `raw-modified`), and `changedFiles` (which code triggered it). The `summary` string remains for human-readable display.

One-line change: `result.summary` → `JSON.stringify(result, null, 2)`.

### 2. Watch `--drift` flag

**Files:** `packages/cli/src/cli.ts`, `packages/cli/src/commands/watch.ts`

Add a `--drift` boolean flag to `sensei watch`. When set, after each successful reindex, `checkDrift` runs and prints the summary only when drift is detected. Silent on clean.

```
sensei watch --drift
```

Behaviour:
- Reindex completes → call `checkDrift(repoPath)`
- Drift found → print `result.summary`
- No drift → print nothing (keep output clean)
- `checkDrift` error → print warning, do not interrupt watch loop

## Files Touched

- Modify: `packages/mcp/src/index.ts` — return JSON from `check_drift` tool
- Modify: `packages/cli/src/cli.ts` — add `--drift` flag to watch args + help text
- Modify: `packages/cli/src/commands/watch.ts` — accept `drift` option, call `checkDrift` after reindex

## Testing

- `watch.ts`: unit test that `--drift` calls `checkDrift` after reindex and prints summary when drift found, and is silent when no drift
- MCP change is a one-liner with no branching — covered by existing integration

## Non-Goals

- No structured output for the `sensei drift` CLI command (prose summary is fine for humans)
- No automatic drift blocking in watch (informational only)
