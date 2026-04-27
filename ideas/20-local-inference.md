---
name: Local Inference
description: Use lightweight local models (Gemma4 via Ollama) to augment cloud models — reduce cost, enable offline operations, and handle high-frequency analysis tasks
date: 2026-04-17
status: idea
related: 08-codebase-intelligence.md, 14-context-delivery.md, 15-pattern-store.md
---

# Local Inference

## Problem

Not every AI task needs Claude. Summarizing a function body, classifying a prompt as correction/continuation, generating L2 logic flow descriptions, detecting structural patterns, computing embeddings — these are high-frequency, lower-complexity tasks that a local model can handle at zero marginal cost. Using Claude for everything is expensive and unnecessary.

## Current state

- Ollama support: designed in `docs/design/01-daemon/local-model-indexer.md`, not implemented
- Gemma4 preference: established in project memory
- No local inference pipeline in the daemon
- All intelligence tasks currently require either the cloud model or heuristics

## What this idea covers

### Tasks suitable for local inference

| Task | Current approach | Local model approach | Benefit |
|------|-----------------|---------------------|---------|
| **L2 logic flow summaries** | Not implemented | Gemma4 summarizes function bodies during indexing | Richer context delivery without cloud API calls |
| **Prompt classification** | Regex heuristics in hook | Local model classifies correction/continuation/clarification | More accurate FTR detection |
| **Pattern detection** | Naming heuristics (Phase A) | Local model classifies code structures into pattern types | Better pattern accuracy than heuristics alone |
| **Embeddings for semantic search** | Not implemented | Local embedding model during indexing | Semantic search without external API |
| **Docstring generation** | Not implemented | Local model generates docstrings for undocumented functions | Fill documentation gaps during indexing |
| **Code similarity** | Not implemented | Local model compares normalized AST structures | Duplicate detection without external tools |
| **Commit message drafting** | Cloud model | Local model for routine commits | Cost reduction |
| **Test case suggestions** | Cloud model | Local model for simple unit test skeletons | Cost reduction for routine tests |

### Architecture

```
senseid (daemon)
├── indexer
│   ├── AST parsing (existing)
│   ├── symbol extraction (existing)
│   └── local inference pass (NEW)
│       ├── L2 summaries
│       ├── pattern classification
│       ├── embedding generation
│       └── docstring generation
├── inference adapter
│   ├── OllamaAdapter (HTTP to localhost:11434)
│   ├── (future) LlamafileAdapter
│   └── NoOpAdapter (graceful degradation when no local model)
└── config
    └── inference.model: "gemma4" (or "none" to disable)
```

### Key design principle: graceful degradation

Local inference is always optional. If Ollama isn't running or the model isn't available:
- Indexing still works (just without L2 summaries, embeddings, etc.)
- Pattern detection falls back to naming heuristics
- Prompt classification falls back to regex
- No features break — they just have less data to work with

### Cost comparison

| Operation | Cloud (Claude) | Local (Gemma4) | Savings |
|-----------|---------------|----------------|---------|
| Summarize 1000 functions | ~$2-5 per index run | $0 (electricity only) | 100% |
| Generate embeddings for 5000 symbols | ~$1-3 per index run | $0 | 100% |
| Classify 500 prompts per day | ~$0.50/day | $0 | 100% |
| Pattern detection on 100 files | ~$1-2 per run | $0 | 100% |

For projects indexed frequently or with many sessions per day, local inference significantly reduces cost.

## Open questions

| # | Question |
|---|----------|
| 1 | What's the minimum hardware to run Gemma4 effectively? Need to document requirements. |
| 2 | Should the inference pass run during indexing (blocking) or as a background post-processing step? |
| 3 | How do we handle model version changes? Re-run summaries when model upgrades? |
| 4 | Should the desktop app manage Ollama lifecycle (start/stop) or expect it to be running? |
| 5 | Can we use quantized models (GGUF) for faster inference on laptops without GPU? |
