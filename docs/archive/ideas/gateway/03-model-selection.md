---
name: Model Selection
description: 3-tier model resolution — direct, chain, capability — with candidate validation pipeline
date: 2026-04-24
status: idea
related: 01-gateway-engine.md, 10-circuit-breaker.md, 11-budget-management.md
reference: /Users/Jerry/Developer/strategos/packages/gateway/src/services/model-selection.ts
---

# Model Selection

## Problem

A caller says "I need embeddings" or "use this specific model" or "run this chain." The gateway needs a single resolution path that handles all three cases, validates each candidate, and returns them in priority order for the engine to walk.

## Resolution tiers

### Tier 1: Direct selection
Caller specifies both router and model: `router: "ollama", model: "gemma3:27b"`
- Resolves to a single candidate
- Validates: router exists, model exists, circuit breaker allows, within budget
- Use case: caller knows exactly what it wants

### Tier 2: Chain selection
Caller specifies a named chain: `chain: "reasoning_chain"`
- Resolves all models in the chain, sorted by priority
- Each candidate validated independently
- Invalid candidates skipped with reason
- Use case: caller wants a capability with fallback

### Tier 3: Capability selection
Caller specifies only a capability: `capability: Capability::Embed`
- Finds the chain configured for that capability
- Falls back to Tier 2 logic
- Use case: caller doesn't care about routing, just needs the capability

## Validation pipeline

Each candidate passes through four checks. Any failure → skip, continue to next:

```
1. Router validation
   ├── Router exists in config?
   ├── Router enabled?
   └── API key available (if required)?

2. Model validation
   ├── Model exists in config?
   └── Model supports requested capability?

3. Availability check
   └── Circuit breaker allows execution? (not Open)

4. Budget check
   ├── Estimate cost for this model
   └── Cost ≤ caller's budget?
```

Skipped candidates carry a reason:

```rust
pub struct SkippedCandidate {
    pub model: String,
    pub router: String,
    pub reason: SkipReason,
}

pub enum SkipReason {
    RouterNotFound,
    RouterDisabled,
    ApiKeyMissing,
    ModelNotFound,
    CapabilityNotSupported,
    CircuitBreakerOpen,
    OverBudget { estimated: f64, budget: f64 },
}
```

These reasons are included in execution traces for debugging.

## API model ID resolution

Some providers use different model IDs than the canonical name. A chain entry can override:

```rust
pub struct ChainEntry {
    pub model: String,              // canonical: "claude-haiku-4-5"
    pub router: Option<String>,     // "anthropic" or inferred from model.provider
    pub api_model_id: Option<String>, // override: "claude-haiku-4-5-20251001"
    pub priority: u8,
}
```

When no explicit router is specified, the model's `provider` field is used as implicit router.

## Public API

```rust
pub struct ModelSelectionService {
    config: Arc<RwLock<GatewayConfig>>,
    circuit_breaker: Arc<CircuitBreakerManager>,
}

impl ModelSelectionService {
    pub fn select(&self, criteria: &SelectionCriteria) -> SelectionResult;
    pub fn select_all_from_chain(&self, criteria: &SelectionCriteria) -> SelectionResult;
}

pub struct SelectionResult {
    pub selected: Option<SelectedModel>,
    pub skipped: Vec<SkippedCandidate>,
    pub chain: Option<FallbackChainConfig>,
}
```

`select()` returns the first valid candidate. `select_all_from_chain()` returns all valid candidates for the engine to walk.

## Open questions

| # | Question |
|---|----------|
| 1 | Should selection support model preferences? e.g. "prefer local" vs "prefer quality" as a soft hint. |
| 2 | Should capability matching be strict (exact) or allow promotion (Classify request → Chat model can handle it)? |
| 3 | Should we cache selection results to avoid re-resolving for the same criteria within a short window? |
