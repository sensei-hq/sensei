# Indexer Comparison: cocoindex vs. sensei

**Date:** 2026-03-10
**Status:** Approved

## Problem

Sensei's current indexer is regex-based and unreliable. cocoindex-code is a mature AST-based indexer with a local model and MCP server, already running against this repo. Before deciding whether to replace sensei's indexer with cocoindex, we need to know if cocoindex is actually better across three dimensions: symbol coverage, description quality, and queryability.

## Goal

A `sensei benchmark indexer` CLI command that runs both indexers against this repo, scores them automatically, and prints a side-by-side report with a spot-check table for manual quality review. Output is sufficient to make an architectural decision.

## Metrics

| Metric | How measured |
|---|---|
| Symbol coverage | `(symbols found) / (known TS exports)` — ground truth from parsing `export` statements |
| Query relevance | 5 hardcoded test queries; check if expected symbols appear in results |
| Description quality | 15-symbol spot-check table printed for manual rating |

## Architecture

Three phases: Collect → Score → Report.

### Components

All under `packages/tools/src/benchmark/indexer-comparison/`:

- **`cocoindex-adapter.ts`** — reads `target_sqlite.db` directly (no MCP server required). Returns `Symbol[]` with `{ name, path, description }`.
- **`sensei-adapter.ts`** — reads `.index/symbol-map.json`; auto-regenerates via `reindex_repo()` if missing. Returns same `Symbol[]` shape.
- **`ground-truth.ts`** — walks `packages/*/src/**/*.ts`, extracts exported identifiers via `export (function|class|const|type)` patterns. Returns `string[]`.
- **`scorer.ts`** — computes coverage %, runs query matching against both indexes, selects random 15-symbol spot-check sample.
- **`reporter.ts`** — formats terminal table (via existing CLI table utilities) and writes `results/indexer-comparison-YYYY-MM-DD.md`.

CLI wired at: `apps/cli/src/commands/benchmark/indexer.ts`

### Data Flow

```
ground-truth.ts ──────────────────────────────┐
cocoindex-adapter.ts → Symbol[] ──► scorer.ts ──► reporter.ts
sensei-adapter.ts    → Symbol[] ──►
```

### Shared Symbol Shape

```ts
type IndexedSymbol = {
  name: string
  path: string
  description: string | null
}
```

## Error Handling

| Condition | Behavior |
|---|---|
| cocoindex db missing | Fail fast: "Run `cocoindex update` first" |
| sensei index missing/stale | Auto-regenerate before comparing |
| `tsc` unavailable | Skip coverage metric, note in report |

## Test Queries (hardcoded)

1. "reindex repo"
2. "check doc drift"
3. "load context"
4. "checkpoint session"
5. "list exports"

Expected symbols per query defined in `scorer.ts` as a fixture.

## Output

Terminal:
```
┌─────────────────────┬──────────────┬──────────────┐
│ Metric              │ cocoindex    │ sensei       │
├─────────────────────┼──────────────┼──────────────┤
│ Symbols found       │ ?            │ ?            │
│ Coverage            │ ?%           │ ?%           │
│ Query hits (5)      │ ?            │ ?            │
└─────────────────────┴──────────────┴──────────────┘

Spot-check (15 symbols — rate manually 1–5):
[table]
```

Markdown written to `results/indexer-comparison-YYYY-MM-DD.md`.

## Testing

Scorer logic (`coverage %`, query matching) gets unit tests with fixture data in `packages/tools/src/benchmark/indexer-comparison/scorer.test.ts`. No tests for adapters or reporter (diagnostic tool).

## Decision Criteria

After running the comparison:
- If cocoindex coverage > 2× sensei AND descriptions are clearly more useful → replace sensei's indexer with cocoindex
- If roughly equivalent → keep sensei's indexer, invest in improving it
- If cocoindex wins but setup is complex → wrap cocoindex as an optional backend
