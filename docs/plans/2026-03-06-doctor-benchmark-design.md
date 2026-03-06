# Doctor Benchmark Design

## Overview

`sensei benchmark doctor` runs an A/B comparison between two doc-reformatting strategies on a sampled set of docs, scoring each on structural quality, content fidelity, and token cost.

## Command Interface

```bash
sensei benchmark doctor <target-dir> \
  [--examples <dir>]   # reference folder for Strategy B (default: same template as A)
  [--sample <N>]       # docs to test (default: 5)
  [--out <dir>]        # results destination (default: results/)
```

## Strategies

| Strategy | Name | Approach |
|---|---|---|
| A | sensei doctor | Explicit template + 6-rule reformat prompt (existing doctor command) |
| B | raw Claude | Few-shot example reference folder — "reformat to match these examples" |

Both strategies call `claude-opus-4-6` with adaptive thinking and capture token usage from the API response.

## Execution Flow

1. Sample up to N `.md` files from `<target-dir>`
2. For each doc: run Strategy A, save output to `results/.../a/<file>`
3. For each doc: run Strategy B, save output to `results/.../b/<file>`
4. Score all docs (structural pass — no API call)
5. Score all docs (LLM judge — one Claude call per doc, both A+B in same prompt)
6. Write `results.json` and `summary.md`

## Scoring

### Structural Score (0–10, automated)

- **+4** all expected template section headers present in output
- **+3** content preservation ratio ≥ 0.8 (key terms from original appear in output)
- **+2** TODO count ≤ original TODO count + 2 (no inflation)
- **+1** no invented proper nouns (no proper nouns in output absent from input)

### LLM Judge Score (0–10, one call per doc)

Both A and B outputs sent in a single Claude call for direct comparison. Rubric: structure conformance, content completeness, no invented content. Returns JSON with `scoreA`, `scoreB`, `reasoningA`, `reasoningB`.

## Output Structure

```
results/benchmark-doctor-YYYY-MM-DD/
├── a/                    # Strategy A outputs
│   └── <filename>.md
├── b/                    # Strategy B outputs
│   └── <filename>.md
├── results.json          # full scores + token counts + approach metadata
└── summary.md            # human-readable comparison table
```

### results.json shape

```json
{
  "date": "2026-03-06",
  "strategies": {
    "a": { "name": "sensei doctor", "approach": "Explicit template + 6-rule reformat prompt", "templatePath": "..." },
    "b": { "name": "raw Claude", "approach": "Few-shot example reference folder", "examplesPath": "..." }
  },
  "docs": [
    {
      "file": "01-core.md",
      "a": { "tokensIn": 1820, "tokensOut": 940, "structuralScore": 8, "judgeScore": 7, "judgeReasoning": "..." },
      "b": { "tokensIn": 2400, "tokensOut": 1100, "structuralScore": 6, "judgeScore": 8, "judgeReasoning": "..." }
    }
  ],
  "totals": {
    "a": { "tokensIn": 9100, "tokensOut": 4700, "avgStructural": 7.8, "avgJudge": 7.4 },
    "b": { "tokensIn": 12000, "tokensOut": 5500, "avgStructural": 6.2, "avgJudge": 7.9 }
  }
}
```

### summary.md shape

```markdown
# Doctor Benchmark — YYYY-MM-DD

| Strategy | Approach | Avg Structural | Avg Judge | Tokens In | Tokens Out |
|---|---|---|---|---|---|
| A | sensei doctor (template + rules) | 7.8 | 7.4 | 9,100 | 4,700 |
| B | raw Claude (example reference) | 6.2 | 7.9 | 12,000 | 5,500 |

## Per-Doc Results
...
```

## Implementation

- New file: `packages/sensei/src/commands/benchmark-doctor.ts`
- New CLI entry: `sensei benchmark doctor` (subcommand under `benchmark`)
- Reuses `callClaude()` from `doctor.ts` (extract to shared `packages/sensei/src/claude.ts`)
- Results written to `results/` (gitignored for JSON, committed for `summary.md`)
