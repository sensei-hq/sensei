# Doctor Benchmark Design

## Overview

`sensei benchmark doctor` runs a 3-way comparison between doc-generation strategies for folder-to-folder conversion. Input: `docs/requirements/`. Output target: `docs/features/` (multiple files + root README). Scores each strategy on structural quality, content fidelity, and token cost.

## Command Interface

```bash
sensei benchmark doctor <input-dir> <output-name> \
  [--template <path>]   # feature template (default: docs/templates/feature.md)
  [--examples <dir>]    # example output folder for Strategy B reference
  [--sample <N>]        # cap input files (default: all)
  [--out <dir>]         # results destination (default: results/)
```

Example:
```bash
sensei benchmark doctor docs/requirements features \
  --template docs/templates/feature.md \
  --examples docs/features/
```

## Strategies

| Strategy | Name | Context sent to Claude | Expected tokens | Expected quality |
|---|---|---|---|---|
| A | Targeted index | L1 summaries of `<input-dir>` files only + template | Lowest | Good |
| B | Raw content | Full content of all input files + example output folder | Highest | Good |
| C | Full repo index | Full `.index/symbol-map.json` + template | Medium-high | Best (C > A) |

All three strategies make **one Claude call** to generate the entire output folder (all feature docs + README) in a single shot.

### Strategy A — Targeted index
Extract L1 summaries (signatures + IO patterns) from `.index/symbol-map.json` for files under `<input-dir>` only. Send with feature template. Lowest context, tests whether compressed index is sufficient.

### Strategy B — Raw content
Read full content of all input files + full content of `--examples` folder. Send as-is with instruction to match example structure. Largest context, establishes quality ceiling.

### Strategy C — Full repo index
Provide entire `.index/symbol-map.json` alongside feature template. More context than A (Claude sees how requirements relate to rest of codebase), tests whether whole-repo context improves output quality at the cost of more tokens.

## Execution Flow

1. Validate inputs: index exists, template exists, examples dir exists
2. Run Strategy A → save output files to `results/.../a/<output-name>/`
3. Run Strategy B → save output files to `results/.../b/<output-name>/`
4. Run Strategy C → save output files to `results/.../c/<output-name>/`
5. Structural scoring pass (automated, no API call)
6. LLM judge pass (one Claude call — all three outputs compared in single prompt)
7. Write `results.json` and `summary.md`

Each strategy output is a folder containing multiple `.md` files + `README.md`.

## Scoring

### Structural Score (0–10, automated, per strategy)

- **+3** README.md present in output
- **+3** all expected template section headers present across output files
- **+2** content coverage ratio ≥ 0.8 (key terms from input appear in output)
- **+2** no TODO inflation (TODO count ≤ input TODO count + N files)

### LLM Judge Score (0–10, one call, all strategies compared)

Single Claude call receives:
- Original input files (requirements)
- Strategy A output folder
- Strategy B output folder
- Strategy C output folder

Rubric: structure conformance, content completeness, no invented content. Returns `{ scoreA, scoreB, scoreC, reasoning }`.

### Token Usage (per strategy)

Captured from API response `usage` field: `tokensIn`, `tokensOut`. Enables direct cost comparison.

## Output Structure

```
results/benchmark-doctor-YYYY-MM-DD/
├── a/features/           # Strategy A output
│   ├── README.md
│   └── *.md
├── b/features/           # Strategy B output
│   ├── README.md
│   └── *.md
├── c/features/           # Strategy C output
│   ├── README.md
│   └── *.md
├── results.json
└── summary.md
```

### results.json shape

```json
{
  "date": "2026-03-06",
  "input": "docs/requirements",
  "outputName": "features",
  "strategies": {
    "a": { "name": "Targeted index", "description": "L1 summaries of input dir only + template", "templatePath": "docs/templates/feature.md" },
    "b": { "name": "Raw content", "description": "Full input content + example output folder", "examplesPath": "docs/features/" },
    "c": { "name": "Full repo index", "description": "Full symbol-map.json + template", "indexPath": ".index/symbol-map.json" }
  },
  "docs": {
    "a": { "tokensIn": 2100, "tokensOut": 3200, "filesGenerated": 11, "structuralScore": 7, "judgeScore": 7 },
    "b": { "tokensIn": 8400, "tokensOut": 3500, "filesGenerated": 11, "structuralScore": 8, "judgeScore": 8 },
    "c": { "tokensIn": 5200, "tokensOut": 3400, "filesGenerated": 11, "structuralScore": 9, "judgeScore": 9 }
  },
  "judgeReasoning": "..."
}
```

### summary.md shape

```markdown
# Doctor Benchmark — YYYY-MM-DD

Input: docs/requirements → docs/features

| Strategy | Approach | Structural | Judge | Tokens In | Tokens Out | Files |
|---|---|---|---|---|---|---|
| A | Targeted index (lowest context) | 7 | 7 | 2,100 | 3,200 | 11 |
| B | Raw content (highest context) | 8 | 8 | 8,400 | 3,500 | 11 |
| C | Full repo index | 9 | 9 | 5,200 | 3,400 | 11 |

## Judge Reasoning
...
```

## Implementation

- New file: `packages/sensei/src/commands/benchmark-doctor.ts`
- New CLI subcommand: `sensei benchmark doctor` (add `benchmark` parent command to `cli.ts`)
- Extract `callClaude()` from `doctor.ts` → `packages/sensei/src/claude.ts` (shared)
- Strategy A: filter `symbol-map.json` to input dir entries, format as L1 summary text
- Strategy B: `readFile` all input + examples files, concatenate
- Strategy C: read full `symbol-map.json`, serialize as structured context
- Output parsing: Claude returns a structured folder — parse into individual files by splitting on `## File: <name>` markers or JSON envelope
- Results written to `results/` (raw JSON gitignored, `summary.md` committed)
