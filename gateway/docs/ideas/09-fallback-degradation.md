---
name: Fallback & Degradation
description: Noop adapter, budget exhaustion, provider outages — how the gateway degrades gracefully without ever failing hard
date: 2026-04-24
status: idea
related: 01-gateway-engine.md, 10-circuit-breaker.md, 11-budget-management.md
---

# Fallback & Degradation

## Principle

**Never block the user. Always degrade, never fail.**

The gateway serves systems that have their own fallback logic (keyword search when semantic search is unavailable, heuristics when classification is unavailable). A hard error from the gateway forces every caller to handle it. A graceful degradation lets callers continue without knowing inference was skipped.

## Degradation levels

```
Level 0: Full capability
  All providers available, budget sufficient, models responding.

Level 1: Reduced quality
  Primary model unavailable → fallback to secondary model.
  External provider down → local model handles it (lower quality).

Level 2: Local only
  All external providers unavailable or budget exhausted.
  Only local models (Ollama) serving requests.
  Quality reduced but functional.

Level 3: Noop
  Ollama not running, no external providers configured.
  Gateway returns structured "unavailable" responses.
  Callers fall back to their own heuristics.
```

## Noop adapter

The noop adapter is always registered and always has the lowest priority in every chain:

```rust
pub struct NoopAdapter;

impl InferenceAdapter for NoopAdapter {
    fn id(&self) -> &str { "noop" }

    fn supports(&self, _capability: &Capability) -> bool { true }

    async fn execute(&self, _config: &RouterConfig, request: &InferenceRequest)
        -> Result<InferenceResponse, GatewayError>
    {
        Ok(InferenceResponse {
            success: false,
            response: None,
            unavailable_reason: Some(UnavailableReason {
                message: "No inference provider available",
                capability: request.capability.clone(),
                suggestion: self.suggest_action(&request.capability),
            }),
            attempts: vec![],
            cost: Cost::zero(),
        })
    }
}
```

The noop adapter **never errors**. It returns a structured response that tells the caller:
- What capability was requested
- Why it's unavailable
- What the caller should do instead

## Degradation triggers

| Trigger | What happens | Level |
|---------|-------------|-------|
| Single model timeout | Fallback to next in chain | 1 |
| Provider rate limited | Skip provider, try next | 1 |
| Provider auth failure | Skip provider permanently (bad key) | 1 |
| All external providers down | Use local models only | 2 |
| Budget exhausted (daily) | Use local models only | 2 |
| Circuit breaker open on all externals | Use local models only | 2 |
| Ollama not running | Skip local models | 2 or 3 |
| No providers at all | Noop responses | 3 |

## Caller-side fallback examples

| Caller | On degradation | Fallback behavior |
|--------|---------------|-------------------|
| Semantic search | Embeddings unavailable | Fall back to full-text search (tsvector) |
| Pattern detector | Classification unavailable | Fall back to AST-based heuristics |
| Summarizer | Summarization unavailable | Skip L2 summaries, serve L0/L1 only |
| MCP search() | Semantic ranking unavailable | Fall back to BM25 keyword ranking |
| Consolidation | Merge unavailable | Queue items, retry later |

## Status reporting

The gateway exposes its current degradation level:

```rust
pub struct GatewayStatus {
    pub level: DegradationLevel,      // L0, L1, L2, L3
    pub adapters: Vec<AdapterStatus>,
    pub circuit_breakers: Vec<BreakerStatus>,
    pub budget: BudgetStatus,
    pub capabilities: HashMap<Capability, CapabilityStatus>,
}

pub enum CapabilityStatus {
    Available { model: String, quality: Quality },
    Degraded { model: String, reason: String },
    Unavailable { reason: String, fallback_hint: String },
}
```

This status is:
- Returned by `gateway_status` MCP tool
- Included in health endpoint (`GET /api/health`)
- Logged at level changes (Level 0→2 is worth logging; 0→1 is routine)

## Recovery

Degradation is temporary. The gateway recovers automatically:

| Condition | Recovery mechanism |
|-----------|-------------------|
| Provider comes back online | Circuit breaker transitions: Open → HalfOpen → Closed |
| Budget resets (new day/month) | Spend counter resets, external providers re-enabled |
| Ollama starts | Next embedding/inference request succeeds |
| API key fixed | Next request to that provider succeeds |

No manual intervention needed. The gateway self-heals.

## Open questions

| # | Question |
|---|----------|
| 1 | Should Level changes be surfaced to the user? ("Inference degraded: external providers unavailable") |
| 2 | Should degradation affect model selection proactively? e.g. at Level 2, skip external candidates entirely instead of attempting and failing. |
| 3 | Should there be a "maintenance mode" where the gateway intentionally degrades (e.g. during model updates)? |
