---
name: Circuit Breaker
description: Per-endpoint failure tracking with state machine — closed, open, half-open — auto-recovery
date: 2026-04-24
status: idea
related: 01-gateway-engine.md, 03-model-selection.md, 09-fallback-degradation.md
reference: /Users/Jerry/Developer/strategos/packages/gateway/src/services/circuit-breaker.ts
---

# Circuit Breaker

## Problem

When a provider is down, hammering it with requests wastes time and may trigger rate limits. The circuit breaker detects repeated failures and temporarily blocks requests to that endpoint, automatically recovering when the provider comes back.

## State machine

```
                 ┌──────────────────────────────────────┐
                 │                                      │
                 ▼                                      │
            ┌─────────┐    failures ≥ threshold    ┌─────────┐
            │ CLOSED   │ ─────────────────────────> │  OPEN   │
            │ (normal) │                            │(blocked)│
            └─────────┘                            └─────────┘
                 ▲                                      │
                 │                                      │ timeout expires
                 │  successes ≥ halfOpenMax             ▼
                 │                                ┌───────────┐
                 └─────────────────────────────── │ HALF_OPEN │
                            any failure ──────────│  (test)   │──> OPEN
                                                  └───────────┘
```

## Configuration

```rust
pub struct CircuitBreakerConfig {
    pub threshold: usize,              // failures before open (default: 5)
    pub timeout: Duration,             // how long to stay open (default: 5 min)
    pub half_open_max_requests: usize, // test requests in half-open (default: 3)
}
```

## State representation

```rust
pub enum BreakerState {
    Closed {
        failure_count: usize,
    },
    Open {
        next_retry: Instant,
    },
    HalfOpen {
        success_count: usize,
    },
}
```

## Endpoint ID

Each circuit breaker tracks a single endpoint: `{adapter}:{model}`

Examples:
- `ollama:gemma3:27b`
- `anthropic:claude-haiku-4-5`
- `openai:gpt-4o-mini`

## API

```rust
pub struct CircuitBreakerManager {
    states: Arc<Mutex<HashMap<String, BreakerState>>>,
    config: CircuitBreakerConfig,
}

impl CircuitBreakerManager {
    pub fn can_execute(&self, endpoint: &str) -> bool;
    pub fn record_success(&self, endpoint: &str);
    pub fn record_failure(&self, endpoint: &str);
    pub fn get_state(&self, endpoint: &str) -> BreakerState;
    pub fn reset(&self, endpoint: &str);
    pub fn reset_all(&self);
}
```

## Transition rules

### `can_execute(endpoint)`
- **Closed:** always true
- **Open:** check `Instant::now() >= next_retry`. If yes → transition to HalfOpen, return true. If no → return false.
- **HalfOpen:** true (allow test requests)

### `record_failure(endpoint)`
- **Closed:** increment `failure_count`. If `≥ threshold` → transition to Open with `next_retry = now + timeout`
- **HalfOpen:** → transition to Open immediately (single failure resets)
- **Open:** no-op (already blocked)

### `record_success(endpoint)`
- **HalfOpen:** increment `success_count`. If `≥ half_open_max_requests` → transition to Closed
- **Closed:** reset `failure_count` to 0
- **Open:** no-op

## Design notes

- **In-memory only** — state is ephemeral. Restart = all breakers reset to Closed. This is fine — we want to re-test endpoints on restart.
- **Lazy initialization** — first `can_execute()` call creates a Closed state for unknown endpoints.
- **Thread-safe** — `Arc<Mutex<HashMap>>`. Transitions happen atomically within the lock.
- **Per-endpoint, not per-adapter** — `ollama:gemma3:27b` and `ollama:qwen3:14b` are tracked independently.

## Open questions

| # | Question |
|---|----------|
| 1 | Should the timeout be adaptive? e.g. double on each re-open (exponential backoff). Strategos uses fixed timeout. |
| 2 | Should breaker state be exposed in the MCP status tool? Useful for debugging but adds surface area. |
| 3 | Should there be a "manual open" mode where the user can force-open a breaker? e.g. "don't use OpenAI for the next hour." |
