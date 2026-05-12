---
name: Adding External Providers
description: Configure Anthropic/OpenAI → chains upgrade with external fallback → budget activated
date: 2026-04-24
status: idea
---

# Journey: Adding External Providers

## Scenario

The developer has been using local-only inference (Journey 01). Chat quality with local models is good but not great for complex code reasoning. They decide to add Anthropic (Claude) as an external provider.

## What happens

### 1. Set API key

```bash
export ANTHROPIC_API_KEY=sk-ant-...
```

Or configure via sensei settings:

```
sensei config set gateway.providers.anthropic.api_key_env ANTHROPIC_API_KEY
```

### 2. Daemon detects new provider

On next config reload (or restart):

```
  → Probing Anthropic... API key found, testing... OK
  → Registering adapter: anthropic
  → Updating chains with external fallback
  → Budget management activated (daily: $5.00, monthly: $50.00)
  → Gateway ready. Degradation level: 0 (full capability)
```

### 3. Chains auto-upgrade

The gateway extends existing chains with external models:

```
Before:
  chat_chain: gemma3:27b → qwen3:14b → noop

After:
  chat_chain: claude-sonnet-4-6 (anthropic) → gemma3:27b (ollama) → noop
  inference_chain: gemma3:27b (ollama) → claude-haiku-4-5 (anthropic) → noop
  embed_chain: all-minilm:l6-v2 (ollama) → noop  ← unchanged (local-only by design)
```

Chat chain now defaults to Claude for quality. Inference chain keeps local-first but has external fallback.

### 4. Budget kicks in

First external call:

```
chat("explain this auth flow")
  → claude-sonnet-4-6 (anthropic)
  → 1,200 input tokens + 800 output tokens
  → cost: $0.016
  → daily_spent: $0.016 / $5.00 limit
```

Budget tracking starts. All external calls accumulate toward daily/monthly limits.

### 5. User experience improves

Chat responses are noticeably better for complex code reasoning. The gateway transparently routes:
- Background tasks (indexing, classification) → local models (free)
- User-facing chat → Claude (quality, metered)

The user doesn't configure this routing — it's the default chain behavior.

### 6. Adding OpenAI too

```bash
export OPENAI_API_KEY=sk-...
```

Chains extend further:

```
chat_chain: claude-sonnet-4-6 → gpt-4o → gemma3:27b → noop
inference_chain: gemma3:27b → claude-haiku-4-5 → gpt-4o-mini → noop
```

More fallback options. If Anthropic is rate-limited, OpenAI catches it. If both are down, local models handle it.

## What's different from Journey 01

| Aspect | Journey 01 | Journey 02 |
|--------|-----------|-----------|
| Degradation level | 2 (local only) | 0 (full) |
| Chat quality | Good (local) | Excellent (Claude) |
| Budget tracking | Off | Active |
| Fallback depth | 1-2 models | 3-4 models |
| Cost | Free | $0-5/day |
