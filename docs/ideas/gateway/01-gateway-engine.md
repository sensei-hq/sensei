---
name: Gateway Engine
description: Core execution engine — resolves candidates, walks fallback chains, fires hooks, produces traces
date: 2026-04-24
status: idea
related: 02-adapter-system.md, 03-model-selection.md, 10-circuit-breaker.md
reference: /Users/Jerry/Developer/strategos/packages/gateway/src/engine.ts
---

# Gateway Engine

## Problem

Multiple callers (indexing pipeline, session lifecycle, insights engine, MCP tools) need LLM inference with different requirements. Each call needs: model resolution, fallback on failure, cost tracking, and an execution trace. Without a central engine, every caller reimplements this logic.

## Role

The engine is the **orchestrator** — it doesn't know about specific providers or models. It delegates:

- **Model selection** → which models to try, in what order
- **Adapters** → how to talk to a specific provider
- **Circuit breaker** → whether an endpoint is healthy
- **Budget** → whether a model is affordable

The engine's job is to walk candidates and handle the success/failure lifecycle.

## Execution flow

```
InferenceRequest
    │
    ▼
build_criteria() → SelectionCriteria
    │
    ▼
selection.select_all_from_chain(criteria) → candidates[]
    │
    ▼
fire_hook(on_task_start)
    │
    ▼
walk_candidates() {
    for candidate in candidates {
        adapter = registry.get(candidate.adapter)
        result = adapter.execute(candidate, request)

        fire_hook(on_attempt, result)

        if result.success:
            circuit_breaker.record_success(endpoint)
            fire_hook(on_task_end, success)
            return success

        circuit_breaker.record_failure(endpoint)

        if !should_trigger_fallback(result.error, chain.triggers):
            break   // non-triggerable error → stop
    }

    fire_hook(on_task_end, failed)
    return all_failed
}
    │
    ▼
InferenceResponse {
    success, response, error,
    attempts[], estimated_cost, actual_cost, trace
}
```

## Fallback logic

Whether to try the next candidate depends on the error type and the chain's configured triggers:

```rust
fn should_trigger_fallback(error: &GatewayError, triggers: &[FallbackTrigger]) -> bool {
    match error {
        GatewayError::RateLimit { .. }      => triggers.contains(&FallbackTrigger::RateLimit),
        GatewayError::Timeout { .. }        => triggers.contains(&FallbackTrigger::Timeout),
        GatewayError::ProviderError { .. }  => triggers.contains(&FallbackTrigger::ProviderError),
        GatewayError::ModelUnavailable { .. } => triggers.contains(&FallbackTrigger::ModelUnavailable),
        GatewayError::BudgetExceeded { .. } => triggers.contains(&FallbackTrigger::BudgetExceeded),
        _ => false,  // authentication errors, unknown errors → don't retry
    }
}
```

Non-triggerable errors (bad API key, malformed request) stop the chain immediately — retrying won't help.

## Streaming

Streaming follows the same fallback logic but yields events mid-stream:

```rust
pub async fn stream(&self, request: InferenceRequest)
    -> Pin<Box<dyn Stream<Item = Result<StreamEvent, GatewayError>> + Send>>
```

Event types:
- `Chunk { content }` — incremental text from the model
- `ProviderSwitch { from, to, reason }` — fallback happened mid-stream
- `Done { model, tokens, cost }` — stream completed
- `Error { code, message }` — all candidates failed

On adapter failure mid-stream, the engine switches to the next candidate and emits a `ProviderSwitch` event. Each restart is from the beginning (no resumption).

## Hooks

Lifecycle hooks are fire-and-forget — they never block execution or propagate errors:

```rust
fn fire_hook<T>(hook: &Option<Box<dyn Fn(T) + Send + Sync>>, event: T) {
    if let Some(f) = hook {
        tokio::spawn(async move { let _ = f(event); });
    }
}
```

Hook events:
- `TaskStartEvent` — request received, candidates resolved
- `AttemptEvent` — one adapter call completed (success or failure)
- `TaskEndEvent` — final result, total duration, all attempts

## Config hot-reload

```rust
pub fn update_config(&self, config: GatewayConfig) {
    *self.config.write() = config;
}
```

Config is behind `Arc<RwLock<GatewayConfig>>`. Reads are non-blocking. Writes acquire the lock briefly. No restart needed.

## Public API

```rust
pub struct Gateway {
    config: Arc<RwLock<GatewayConfig>>,
    adapters: AdapterRegistry,
    selection: ModelSelectionService,
    circuit_breaker: CircuitBreakerManager,
}

impl Gateway {
    pub fn new(config: GatewayConfig, adapters: AdapterRegistry) -> Self;
    pub async fn execute(&self, request: InferenceRequest) -> InferenceResponse;
    pub async fn stream(&self, request: InferenceRequest)
        -> Pin<Box<dyn Stream<Item = Result<StreamEvent, GatewayError>> + Send>>;
    pub fn update_config(&self, config: GatewayConfig);
}
```

## Open questions

| # | Question |
|---|----------|
| 1 | Should the engine retry within a single adapter (e.g. transient network error) or only fallback across adapters? Strategos does adapter-level only. |
| 2 | Should streaming support resumption? If adapter A streams 50% then fails, should adapter B start from scratch or attempt to continue? |
| 3 | Should hooks be configurable per-chain or global only? |
