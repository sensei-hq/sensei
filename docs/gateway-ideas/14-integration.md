---
name: Integration
description: Daemon lifecycle wiring, health endpoints, MCP tool exposure — how the gateway connects to the rest of the system
date: 2026-04-24
status: idea
related: 01-gateway-engine.md, 13-configuration.md
---

# Integration

## Problem

The gateway is a library — it needs to be wired into the daemon process, exposed via MCP, and monitored via health endpoints. This doc covers how the gateway connects to the surrounding system.

## Daemon lifecycle

### Startup sequence

```
1. Load config from DB (settings + services tables)
2. Build GatewayConfig via GatewayBuilder
3. Probe available providers:
   - Ollama: GET http://localhost:11434/api/tags (timeout: 2s)
   - Anthropic: check ANTHROPIC_API_KEY env var exists
   - OpenAI: check OPENAI_API_KEY env var exists
4. Register adapters (available providers + noop fallback)
5. Create Gateway instance → Arc<Gateway>
6. Store in AppState (shared across API handlers + MCP)
7. Log: "Gateway initialized: {adapters}, degradation level: {level}"
```

**Non-blocking:** Provider probes run with a short timeout. If Ollama is slow to respond, startup continues without it (noop fallback handles requests until Ollama becomes available).

### Shutdown

No special cleanup needed. The gateway doesn't hold persistent connections (reqwest creates connections per-request). Dropping `Arc<Gateway>` is sufficient.

### Config hot-reload

When `settings` or `services` change:
1. Rebuild `GatewayConfig` from DB
2. Call `gateway.update_config(new_config)`
3. Re-probe providers if services changed
4. Register/unregister adapters as needed

Trigger: DB notify channel, API endpoint, or periodic poll (every 60s).

## MCP tools

The gateway exposes these MCP tools for ACP consumption:

### `infer`
General-purpose inference call.
```
Input:  { capability, messages, model?, chain?, max_tokens?, temperature? }
Output: { text, model_used, tokens, cost, duration_ms }
```

### `embed`
Generate embeddings.
```
Input:  { texts: string[] }
Output: { embeddings: float[][], model, dimensions }
```

### `gateway_status`
Health and status check.
```
Output: {
  degradation_level,
  adapters: [{ id, available, models }],
  circuit_breakers: [{ endpoint, state }],
  budget: { daily_limit, daily_spent, monthly_limit, monthly_spent },
  capabilities: { embed: "available", chat: "degraded", ... }
}
```

All MCP calls go through the full engine path — selection, circuit breaker, budget, tracing. They produce `inference_calls` records for analytics.

## Health endpoint

`GET /api/health` includes gateway status:

```json
{
  "status": "healthy",
  "gateway": {
    "degradation_level": 0,
    "adapters": {
      "ollama": { "available": true, "models": ["gemma3:27b", "all-minilm:l6-v2"] },
      "anthropic": { "available": true },
      "noop": { "available": true }
    },
    "budget": { "daily_remaining": 4.23, "monthly_remaining": 47.50 }
  }
}
```

## Callers within the daemon

| Caller | Uses | Frequency |
|--------|------|-----------|
| Indexing pipeline | embed chain | High (every file change) |
| Pattern detector | inference chain (classify) | Medium (during index) |
| Summarizer | inference chain (summarize) | Medium (during index) |
| Session lifecycle | inference chain (classify) | Per user message |
| Insights engine | consolidation chain | Background, periodic |
| MCP tools (user) | chat chain, embed chain | On-demand |

All callers use the same `Arc<Gateway>` — shared config, shared circuit breakers, shared budget tracking.

## Embedding in consumer applications

The gateway is designed as a **standalone crate** that can be embedded by any Rust application:

```rust
// In Cargo.toml
[dependencies]
gateway = { path = "../gateway" }

// In application code
let config = GatewayBuilder::new()
    .add_router("ollama", ollama_config)
    .add_model(gemma_config)
    .add_chain(embed_chain)
    .build()?;

let gateway = Gateway::new(config, adapters);
let response = gateway.execute(request).await?;
```

Sensei is the primary consumer, but the gateway has no sensei-specific dependencies.

## Open questions

| # | Question |
|---|----------|
| 1 | Should the gateway crate be published to crates.io? It's useful as a general-purpose LLM router. |
| 2 | Should MCP tools be in the gateway crate or in sensei-mcp? Gateway crate = portable, sensei-mcp = integrated. |
| 3 | Should there be a gateway CLI for standalone use? `gateway serve --config config.toml` |
