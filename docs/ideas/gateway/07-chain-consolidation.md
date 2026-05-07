---
name: Consolidation Chain
description: Merge, deduplicate, and synthesize knowledge from multiple sources into coherent outputs
date: 2026-04-24
status: idea
related: 04-chain-inference.md, 06-chain-chat.md
---

# Consolidation Chain

## Problem

Multiple system components produce overlapping, redundant, or contradictory knowledge:

- **Memory system** — accumulated observations, some stale, some redundant
- **Pattern detector** — similar patterns detected across files/sessions
- **Session replays** — recurring correction themes across sessions
- **Code graph** — duplicate structures, overlapping communities
- **Documentation** — docs that describe the same thing differently

Without consolidation, this knowledge accumulates but never synthesizes. The user sees 47 memories instead of 5 insights. The system stores 12 variations of the same pattern instead of 1 canonical version.

## Purpose

The consolidation chain takes **N pieces of related knowledge** and produces:

1. **Merged representation** — a single, canonical version that captures all unique information
2. **Deduplication signals** — which inputs are redundant and can be archived
3. **Contradiction detection** — where inputs disagree, with suggested resolution
4. **Confidence assessment** — how confident the merge is (high = clear overlap, low = ambiguous)

## Use cases

### Memory consolidation
```
Input:  [memory_1: "auth module uses JWT", memory_2: "auth uses JWT tokens with 1h expiry",
         memory_3: "authentication handles JWT refresh"]
Output: {
    merged: "Auth module uses JWT with 1h expiry and handles token refresh",
    archive: [memory_1, memory_3],  // subsumed by merged + memory_2
    keep: [memory_2],               // most specific, update with refresh info
    confidence: 0.85
}
```

### Pattern deduplication
```
Input:  [pattern_A: "error handling with Result<T, AppError>",
         pattern_B: "functions return Result with custom error type",
         pattern_C: "error propagation via ? operator with AppError"]
Output: {
    canonical: "Error handling: all functions return Result<T, AppError>, propagated via ?",
    archive: [pattern_A, pattern_B],  // subsumed
    promote: pattern_C,               // closest to canonical, update it
    confidence: 0.92
}
```

### Correction theme clustering
```
Input:  [correction_1: "don't use unwrap() in production code",
         correction_2: "handle the error instead of unwrapping",
         correction_3: "avoid panic paths in handlers"]
Output: {
    theme: "Error handling discipline: no unwrap/panic in production paths",
    frequency: 3,
    recommendation: "Add rule: 'no-unwrap-in-handlers'",
    confidence: 0.88
}
```

## Default chain

```yaml
consolidation_chain:
  capability: consolidate
  fallback_triggers: [timeout, model_unavailable]
  models:
    - model: gemma3:27b
      router: ollama
      priority: 1          # good enough for merge/dedup
    - model: qwen3:14b
      router: ollama
      priority: 2          # alternative local
    - model: claude-haiku-4-5
      router: anthropic
      priority: 3          # external fallback for complex merges
```

Local-first because consolidation is a background task (high frequency, latency-tolerant).

## Protocol

```
1. Group related items (caller provides the grouping)
2. For each group:
   a. Format items as structured input
   b. Send to consolidation chain with merge prompt
   c. Parse structured output (merged text, archive list, confidence)
   d. If confidence < threshold → flag for human review
   e. If confidence >= threshold → apply automatically (archive duplicates, update canonical)
```

## Structured prompts

The consolidation chain uses structured prompts with JSON output:

```
System: You are a knowledge consolidation engine. Given N related items,
produce a single merged representation. Identify which inputs are redundant.
Flag contradictions. Output JSON.

User: Consolidate these {type} items:
{items as JSON array}

Expected output:
{
  "merged": "...",
  "archive_ids": [...],
  "keep_ids": [...],
  "contradictions": [...],
  "confidence": 0.0-1.0
}
```

## Triggers

| Trigger | Input | Threshold |
|---------|-------|-----------|
| Memory count exceeds N per topic | Group by topic tag | N = 5 |
| Pattern similarity score > 0.8 | Embedding cosine similarity | 0.8 |
| Same correction 3+ times | Session correction clustering | 3 |
| Scheduled daily | All recent unconsolidated items | — |

## Relationship to MOE

Consolidation is a **single-model task** — it doesn't need debate. MOE consensus is for higher-stakes decisions (root cause analysis, action recommendations). Consolidation produces input that MOE may later reason about.

```
Consolidation (this chain)     →  "Auth module has 3 recurring corrections"
MOE consensus (sensei wrapper) →  "Root cause: missing persona. Action: create auth persona"
```

## Open questions

| # | Question |
|---|----------|
| 1 | Should consolidation run eagerly (on every new item) or lazily (on threshold/schedule)? |
| 2 | How do we handle consolidation quality? If the model merges incorrectly, we lose information. Should we keep originals as soft-deleted? |
| 3 | Should contradictions be auto-resolved (model picks one) or always flagged for human? |
| 4 | What's the max batch size for one consolidation call? Too many items → model loses context. |
