---
name: First Run
description: Zero config to working local inference — Ollama detected, chains configured, first embedding generated
date: 2026-04-24
status: idea
---

# Journey: First Run

## Scenario

A developer installs sensei for the first time. They have Ollama running locally with `gemma3:27b` and `all-minilm:l6-v2` pulled. They haven't configured any external API keys.

## What happens

### 1. Daemon starts

```
senseid starting...
  → Loading gateway config from DB... (none found, using defaults)
  → Probing Ollama at http://localhost:11434... OK
    Models available: gemma3:27b, all-minilm:l6-v2, qwen3:14b
  → Probing Anthropic... ANTHROPIC_API_KEY not set, skipping
  → Probing OpenAI... OPENAI_API_KEY not set, skipping
  → Registering adapters: ollama, noop
  → Gateway ready. Degradation level: 2 (local only)
```

### 2. Default chains auto-configured

With only Ollama available, the gateway creates chains from available models:

```
embed_chain:    all-minilm:l6-v2 (ollama) → noop
inference_chain: gemma3:27b (ollama) → qwen3:14b (ollama) → noop
chat_chain:     gemma3:27b (ollama) → qwen3:14b (ollama) → noop
```

No consolidation or voice chains (no suitable models detected).

### 3. First indexing run

The indexing pipeline starts processing the project:

```
Indexing src/main.rs...
  → embed("fn main() { ... }") → all-minilm:l6-v2 → [0.023, -0.115, ...] (384-dim)
  → classify("main.rs") → gemma3:27b → "application_entry_point"
  → summarize("fn main") → gemma3:27b → "Application entry point that initializes..."
```

All calls are local, free, and produce `inference_calls` records.

### 4. User asks a question

```
> search("where is the playground ideation?")

Gateway routes:
  → embed("where is the playground ideation?") → all-minilm:l6-v2
  → cosine similarity against stored embeddings
  → top result: docs/ideas/25-playground-and-insights.md (0.87 similarity)
```

The semantic search layer works — powered entirely by local inference.

### 5. What the user sees

Nothing about the gateway. It's invisible. They ask questions, get answers. The gateway handled model selection, embedding, and routing without any configuration.

## Key principle

**Zero config should work.** If Ollama is running, everything works. If it's not, the gateway degrades to noop and the system falls back to keyword search. The user never sees an error about inference — they just get slightly less intelligent results.

## What's missing at this level

- No external provider fallback (no API keys)
- No budget tracking needed (everything is free)
- No voice chain (no Whisper/TTS models detected)
- Chat quality limited to local models (gemma3:27b is capable but not Claude-level)

These unlock in [Journey 02: Adding External Providers](./02-adding-providers.md).
