# Benchmark Git-Branch Design

## Overview

`sensei benchmark doctor` runs 3 strategies, creates a git branch per strategy, commits the generated docs to each branch, picks a winner, and stays on the winner branch. `sensei benchmark inspect` switches branches for review. `sensei benchmark promote` merges the chosen branch back into the base branch and offers to delete the others.

`sensei serve` and the `submit_benchmark_report` MCP tool are unchanged.

## Run Naming

Each benchmark run gets a random memorable adjective+noun name (e.g. `wild-cat`). Branches:

```
benchmark/wild-cat-a   ← strategy A (Targeted index)
benchmark/wild-cat-b   ← strategy B (Raw content)
benchmark/wild-cat-c   ← strategy C (Full repo index)
```

## Commit Message Format

```
chore: sensei benchmark doctor using "targeted index": docs/features (11 files)
```

Single-line. Includes the command (`sensei benchmark doctor`), strategy name in quotes, and `<outputDir> (<N> files)`.

## Command Interface

```bash
# Run benchmark — creates 3 branches, stays on winner
sensei benchmark doctor docs/requirements features --examples docs/features/

# Switch to a branch to inspect its output
sensei benchmark inspect wild-cat-b

# Merge chosen branch, offer to delete others, submit telemetry
sensei benchmark promote wild-cat
```

## Git Permission Model

Before any git operations, sensei displays exactly what it will do and prompts:

```
sensei will perform these git operations:
  git checkout -b benchmark/wild-cat-a
  git add docs/features/ benchmark-wild-cat.json
  git commit -m "chore: sensei benchmark doctor using \"targeted index\": docs/features (11 files)"
  git checkout <baseBranch>
  git checkout -b benchmark/wild-cat-b
  ...
  git checkout benchmark/wild-cat-c  ← winner

Proceed? [Y/n]
```

**Preconditions checked before asking:**
- Working tree is clean — hard stop if not (print error, exit)
- On a valid branch (not detached HEAD)
- `baseBranch` recorded before any checkouts

## `benchmarkDoctor` Changes

Remove: all folder-based output (`results/a/`, `results/b/`, `results/c/`, auto-promote to `docs/<outputName>/`).

Add:

1. Check preconditions (clean tree, valid branch)
2. Generate run name (e.g. `wild-cat`) — see Name Generation below
3. Display git ops plan, ask permission
4. For each strategy A/B/C:
   - `git checkout -b benchmark/wild-cat-{a|b|c} <baseBranch>`
   - Run strategy, write output to `docs/<outputName>/`
   - Write `benchmark-wild-cat.json` (see Schema below)
   - `git add docs/<outputName>/ benchmark-wild-cat.json`
   - `git commit -m "chore: sensei benchmark doctor using \"<strategyName>\": docs/<outputName> (<N> files)"`
5. `git checkout <baseBranch>` then score all strategies
6. Pick winner: `max(structuralScore + judgeScore)` — ties broken by A first
7. `git checkout benchmark/wild-cat-<winner>`
8. Print summary table + announce winner

**Output:** stays on winner branch. `benchmark-wild-cat.json` exists on each of the 3 branches.

## `sensei benchmark inspect`

```bash
sensei benchmark inspect wild-cat-b
```

1. Verify `benchmark/wild-cat-b` exists
2. Ask permission: `"git checkout benchmark/wild-cat-b"`
3. Checkout branch

## `sensei benchmark promote`

```bash
sensei benchmark promote wild-cat
```

1. Find the 3 branches for this run (`benchmark/wild-cat-a/b/c`) — error if not all present
2. Read `benchmark-wild-cat.json` from each branch (via `git show`)
3. Display comparison table (structural, judge, tokens, files per strategy)
4. `select` prompt: "Which strategy to promote? A / B / C" (highlights auto-winner)
5. `text` prompt: "Optional note:" (skip with Enter)
6. Ask permission:
   ```
   git checkout <baseBranch>
   git merge benchmark/wild-cat-<choice>
   ```
7. Execute merge
8. Read `benchmark-wild-cat.json`, update `userFeedback` + `promoted` fields, write back, commit:
   ```
   chore: record benchmark promotion wild-cat → <choice>
   ```
9. Submit telemetry via `fetch` to `SENSEI_TELEMETRY_URL` (fire-and-forget, silent on failure)
10. Ask: "Delete the other 2 benchmark branches? (benchmark/wild-cat-{x,y})"
    - If yes: `git branch -d benchmark/wild-cat-x && git branch -d benchmark/wild-cat-y`

## `benchmark-<runname>.json` Schema

Stored in the repo root of each branch. Same content on all 3 branches (scores filled in for all, not just this branch's strategy).

```json
{
  "run": "wild-cat",
  "baseBranch": "main",
  "branches": {
    "a": "benchmark/wild-cat-a",
    "b": "benchmark/wild-cat-b",
    "c": "benchmark/wild-cat-c"
  },
  "autoPromoted": "c",
  "userFeedback": null,
  "promoted": null,
  "report": {
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
      "a": { "name": "Targeted index", "promptChars": 1840, "promptLines": 42 },
      "b": { "name": "Raw content",    "promptChars": 9200, "promptLines": 310 },
      "c": { "name": "Full repo index","promptChars": 5400, "promptLines": 180 }
    },
    "results": {
      "a": { "tokensIn": 480,  "tokensOut": 3200, "filesGenerated": 11, "structuralScore": 7, "judgeScore": 7 },
      "b": { "tokensIn": 2300, "tokensOut": 3500, "filesGenerated": 11, "structuralScore": 8, "judgeScore": 8 },
      "c": { "tokensIn": 1400, "tokensOut": 3400, "filesGenerated": 11, "structuralScore": 9, "judgeScore": 9 }
    },
    "userFeedback": null,
    "promoted": null
  }
}
```

After promote: `userFeedback` and `promoted` are filled in, committed to the base branch.

## Name Generation

Random adjective+noun pairs from embedded wordlists (no external dependency):

```typescript
const ADJECTIVES = ["wild", "blue", "red", "new", "old", "swift", "dark", "bright", ...];
const NOUNS      = ["cat", "moon", "bird", "river", "ridge", "oak", "pine", "fox", ...];

function generateRunName(): string {
  const adj  = ADJECTIVES[Math.floor(Math.random() * ADJECTIVES.length)];
  const noun = NOUNS[Math.floor(Math.random() * NOUNS.length)];
  return `${adj}-${noun}`;
}
```

## Winner Picking

```typescript
function pickWinner(results): "a" | "b" | "c" {
  const scores = (["a", "b", "c"] as const).map(k => ({
    key: k,
    score: results[k].structuralScore + results[k].judgeScore,
  }));
  return scores.reduce((best, cur) => cur.score > best.score ? cur : best).key;
}
```

Ties broken by array order (A wins ties).

## Implementation

- Modify: `packages/sensei/src/commands/benchmark-doctor.ts`
  - Remove folder-based output + old auto-promote logic
  - Add git ops: branch creation, commit, winner checkout
  - Add name generation
- Replace: `packages/sensei/src/commands/benchmark-promote.ts`
  - Remove folder-copy logic
  - Add git merge + branch deletion
- Create: `packages/sensei/src/commands/benchmark-inspect.ts`
- Modify: `packages/sensei/src/cli.ts` (add `benchmark inspect` subcommand)
- `packages/sensei/src/commands/serve.ts` — unchanged
- `packages/sensei/src/index.ts` — unchanged
