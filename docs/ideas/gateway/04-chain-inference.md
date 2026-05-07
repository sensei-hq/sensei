---
name: Inference Chain
description: General-purpose reasoning chain — classification, summarization, analysis: local models first, external providers as fallback
date: 2026-04-24
status: idea
related: 01-gateway-engine.md, 05-chain-embedding.md, 06-chain-chat.md, 09-fallback-degradation.md
---

# Inference Chain

## Purpose

The inference chain handles general reasoning tasks that don't require conversational context or embedding generation. These are the workhorses of background processing:

- **Classification** — prompt classification, pattern detection, file type categorization
- **Summarization** — L2 logic flow summaries, docstring generation, commit message drafting
- **Analysis** — code review signals, complexity assessment, dependency analysis
- **Extraction** — structured data from unstructured text, entity extraction

## Why a separate chain

These tasks share characteristics:
- **High frequency** — indexing pipeline runs them on every file change
- **Latency-tolerant** — background tasks, not blocking user interaction
- **Quality-flexible** — a smaller local model is often good enough
- **Cost-sensitive** — running these externally at scale gets expensive fast

This makes them ideal for local-first routing: use the best available local model, only fall back to external when local quality is insufficient or unavailable.

## Default chain

```yaml
inference_chain:
  capability: inference
  fallback_triggers: [timeout, model_unavailable, provider_error]
  models:
    - model: gemma3:12b
      router: ollama
      priority: 1          # fast, good enough for classification
    - model: gemma3:27b
      router: ollama
      priority: 2          # better quality for summarization
    - model: claude-haiku-4-5
      router: anthropic
      priority: 3          # external fallback, high quality
    - model: gpt-4o-mini
      router: openai
      priority: 4          # second external fallback
```

## Task routing within the chain

Not all inference tasks need the same quality. The engine selects the starting model based on task complexity:

| Task | Preferred start | Rationale |
|------|----------------|-----------|
| File type classification | gemma3:12b | Binary decision, small context |
| Prompt classification | gemma3:12b | Short input, few categories |
| Pattern detection | gemma3:27b | Needs code understanding |
| L2 summarization | gemma3:27b | Needs to reason about logic flow |
| Code review signals | claude-haiku-4-5 | Needs deep code understanding |

**Mechanism:** The caller can hint with `min_quality: "low" | "medium" | "high"` to skip lower-priority models. The chain still walks from the hinted model downward on failure.

## Fallback behavior

```
gemma3:12b → timeout → try gemma3:27b
gemma3:27b → model_unavailable → try claude-haiku
claude-haiku → rate_limit → try gpt-4o-mini
gpt-4o-mini → all failed → noop (skip inference, use heuristic)
```

Authentication errors (bad API key) stop the chain — retrying won't help.

## Budget interaction

External models (priority 3+) are subject to budget checks:
- If daily budget exceeded → skip external, return best local result or noop
- Local models (priority 1-2) are always free → never budget-filtered

## Metrics to track

| Metric | Purpose |
|--------|---------|
| Calls per model per day | Usage distribution |
| Fallback rate | How often local models fail |
| Avg latency per model | Performance comparison |
| Cost per day (external) | Budget utilization |
| Quality score (if available) | Downstream task success rate |

## Open questions

| # | Question |
|---|----------|
| 1 | Should task complexity be inferred from the request (input length, capability type) or always explicit from the caller? |
| 2 | Should we batch classification requests? e.g. classify 10 files in one call instead of 10 separate calls. |
| 3 | Should the chain cache results? If the same file is classified twice in 5 minutes, skip the second call. |
