---
name: Request Lifecycle
description: Complete path of an inference request — from MCP call through selection, execution, tracing, to response
date: 2026-04-24
status: idea
---

# Journey: Request Lifecycle

## Scenario

A developer asks sensei: "what functions call `refresh_token`?" The MCP `search()` tool needs to embed the query for semantic search, then also needs inference to rank and explain results.

## The embedding call

### Step 1: MCP receives the query

```
search("what functions call refresh_token?")
  → needs embedding for semantic search
  → gateway.execute(InferenceRequest {
      capability: Embed,
      payload: EmbedPayload { texts: ["what functions call refresh_token?"] },
    })
```

### Step 2: Model selection

```
SelectionCriteria { capability: Embed }
  → Tier 3: capability lookup → finds embed_chain
  → Chain models: [all-minilm:l6-v2 (ollama, p:1)]
  → Validate candidate:
    ✓ Router "ollama" exists and enabled
    ✓ Model supports Embed capability
    ✓ Circuit breaker: Closed (healthy)
    ✓ Budget: N/A (local model, free)
  → Result: SelectedModel { model: "all-minilm:l6-v2", router: "ollama" }
```

### Step 3: Execution

```
Engine.walk_candidates():
  → fire_hook(on_task_start, { task_id: "emb-a1b2", candidates: 1 })
  → Adapter: ollama
  → POST http://localhost:11434/v1/embeddings
    { model: "all-minilm:l6-v2", input: ["what functions call refresh_token?"] }
  → Response: { embedding: [0.023, -0.115, ...], usage: { tokens: 8 } }
  → circuit_breaker.record_success("ollama:all-minilm:l6-v2")
  → fire_hook(on_attempt, { status: success, duration_ms: 12 })
  → fire_hook(on_task_end, { status: success, cost: $0.00 })
```

### Step 4: Trace recorded

```
inference_calls: {
  capability: "embed",
  adapter: "ollama",
  model: "all-minilm:l6-v2",
  input_tokens: 8,
  cost_usd: 0.00,
  duration_ms: 12,
  status: "success",
  fallback_sequence: 0
}
```

### Step 5: Embedding returned to caller

```
EmbedResponse {
  embeddings: [[0.023, -0.115, ...]],
  model: "all-minilm:l6-v2",
  dimensions: 384,
  duration_ms: 12
}
```

Search layer uses this to find `get_callers("refresh_token")` results + semantically similar code.

## The chat call (if user wants an explanation)

### Step 1: User follows up

```
"explain the refresh_token flow"
  → gateway.execute(InferenceRequest {
      capability: Chat,
      payload: ChatPayload {
        messages: [{ role: user, content: "explain the refresh_token flow" }],
        system: "You are a code analysis assistant...",
      },
    })
```

### Step 2: Model selection (chat chain)

```
SelectionCriteria { capability: Chat }
  → embed_chain? No. inference_chain? No. chat_chain? Yes.
  → Chain models: [
      claude-sonnet-4-6 (anthropic, p:1),
      gpt-4o (openai, p:2),
      gemma3:27b (ollama, p:3),
    ]
  → Validate candidate 1:
    ✓ Router "anthropic" exists and enabled
    ✓ Model supports Chat capability
    ✓ Circuit breaker: Closed
    ✓ Budget: estimate $0.06, daily remaining $4.84 → affordable
  → Result: SelectedModel { model: "claude-sonnet-4-6", router: "anthropic" }
```

### Step 3: Execution (streaming)

```
Engine.stream():
  → fire_hook(on_task_start)
  → Adapter: anthropic
  → POST https://api.anthropic.com/v1/messages (stream: true)
  → SSE events:
    content_block_delta: "The refresh_token flow..."
    content_block_delta: "starts in auth/handler.rs..."
    message_delta: { usage: { input: 450, output: 280 } }
  → Yield: StreamEvent::Chunk("The refresh_token flow...")
  → Yield: StreamEvent::Chunk("starts in auth/handler.rs...")
  → Yield: StreamEvent::Done { model: "claude-sonnet-4-6", tokens: 730, cost: 0.006 }
```

### Step 4: Trace + cost recorded

```
inference_calls: {
  capability: "chat",
  adapter: "anthropic",
  model: "claude-sonnet-4-6",
  input_tokens: 450,
  output_tokens: 280,
  cost_usd: 0.006,
  duration_ms: 1840,
  status: "success"
}

Budget: daily_spent += 0.006 → $0.022 / $5.00
```

## Key takeaway

Two gateway calls happened transparently:
1. Embedding (local, free, 12ms)
2. Chat (external, $0.006, 1840ms)

The user asked one question. The system routed each sub-task through the appropriate chain. No configuration needed.
