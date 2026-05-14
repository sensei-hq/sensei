# Gateway -- Inference Routing

## Overview

The gateway is a standalone Rust crate (`crates/gateway/`) that routes LLM inference requests across providers with fallback, cost tracking, and circuit breaking. It is a port of the Strategos gateway concepts into idiomatic Rust. See [ideas/05-gateway](../ideas/05-gateway.md) for the motivation and scope.

The gateway has no sensei-specific dependencies. Sensei consumes it as a library; other applications can too.

## Architecture

```
crates/gateway/src/
  lib.rs              -- public API: Gateway, GatewayBuilder
  engine.rs           -- execution engine, fallback chain walking
  selection.rs        -- model selection service (3-tier resolution)
  circuit_breaker.rs  -- per-endpoint failure management
  budget.rs           -- cost estimation, budget filtering
  config.rs           -- GatewayBuilder, validation, defaults
  consensus.rs        -- MOE consensus protocol (wrapper over engine)
  types/
    mod.rs
    config.rs         -- RouterConfig, ModelConfig, ChainConfig
    request.rs        -- InferenceRequest, InferenceResponse, StreamEvent
    cost.rs           -- Cost, CostEstimate, ModelPricing, Budget
    error.rs          -- GatewayError enum (thiserror)
    trace.rs          -- ExecutionTrace, Attempt, hook events
  adapters/
    mod.rs            -- InferenceAdapter trait, AdapterRegistry
    base.rs           -- HTTP utilities, SSE parsing, error extraction
    ollama.rs         -- Ollama adapter (OpenAI-compatible)
    anthropic.rs      -- Anthropic adapter
    openai.rs         -- OpenAI-compatible adapter (also covers OpenRouter)
    noop.rs           -- graceful degradation when no providers available
```

Dependency flow: `lib.rs` (public API) depends on `engine`, `config`, and `consensus`. The engine depends on `selection`, `circuit_breaker`, `budget`, and `adapters`. All modules depend on `types/`.

### Concurrency model

- **Gateway** is `Arc`-wrapped and shared across tokio tasks.
- **Config** is `Arc<RwLock<GatewayConfig>>` -- reads are non-blocking, writes are rare (hot-reload).
- **Circuit breaker** uses `Arc<Mutex<HashMap>>` -- short critical sections.
- **Adapters** are `Arc<dyn InferenceAdapter>` -- stateless, shared freely.
- **Hooks** fire via `tokio::spawn` -- never block the engine.
- **Streaming** returns `Pin<Box<dyn Stream + Send>>` -- composable with any async runtime.

## Type system

### Capability enum

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Capability {
    Chat,
    Embed,
    Classify,
    Summarize,
    Consolidate,
    VoiceStt,
    VoiceTts,
}
```

Capabilities are matched at compile time -- no stringly-typed checks.

### Error enum

All errors use `thiserror` for ergonomic handling:

```rust
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    Authentication { adapter, message },
    RateLimit { adapter, retry_after_ms },
    BudgetExceeded { estimated, remaining },
    Timeout { adapter, model, duration_ms },
    ProviderError { adapter, message, status },
    ModelUnavailable { adapter, model },
    NoCandidates { capability },
    AllAttemptsFailed { attempts },
    Network(reqwest::Error),
    Serialization(serde_json::Error),
}
```

Each error variant knows whether it is retryable (`is_retryable()`) and whether it should trigger a fallback given the chain's configured triggers (`should_trigger_fallback()`).

### Request / response types

- `InferenceRequest` -- capability, messages, model hints, budget, max tokens.
- `InferenceResponse` -- success flag, content, token usage, cost, execution trace.
- `StreamEvent` -- `Chunk`, `ProviderSwitch`, `Done`, `Error` variants for streaming.
- `StreamChunk` -- incremental content with optional finish reason and usage.
- `TokenUsage` -- input, output, and total token counts.
- `FallbackTrigger` -- `RateLimit`, `Timeout`, `ProviderError`, `ModelUnavailable`, `BudgetExceeded`.

All types derive `Serialize, Deserialize` with `#[serde(rename_all = "snake_case")]` for JSON storage, API bodies, and execution trace persistence.

## Chains

A chain is a named sequence of model candidates for a given capability. The engine walks the chain in priority order until one succeeds.

### Inference chain

General-purpose text generation. Walks from preferred model (e.g. local Ollama) through external providers. Used by the indexing pipeline, session lifecycle, and insights engine.

### Embedding chain

Vector embedding generation. Typically local-only (Ollama embedding models) with external fallback for quality-sensitive operations.

### Chat chain

User-facing conversational inference. May prefer higher-quality external models, with local fallback when budget is exhausted.

### Consolidation chain

Knowledge merging -- a new capability type not in Strategos. Combines multiple context windows into a unified summary. Used by the consolidation pipeline.

### Voice chains (STT / TTS)

Speech-to-text and text-to-speech. Separate chains because provider support varies significantly.

### Fallback behavior

Each chain configures which error types trigger a fallback to the next candidate via `FallbackTrigger`. Non-triggerable errors (bad API key, malformed request) stop the chain immediately -- retrying will not help. On adapter failure mid-stream, the engine switches to the next candidate and emits a `ProviderSwitch` event. Each restart is from the beginning (no resumption).

## Model selection

Resolution follows a 3-tier strategy:

### Tier 1: Exact match

Caller specifies both adapter and model directly (e.g. `adapter: "ollama", model: "gemma3:27b"`). Resolves to a single candidate. Validates that the adapter exists, the model exists, the circuit breaker allows it, and it is within budget.

### Tier 2: Capability match

Caller specifies a named chain (e.g. `chain: "reasoning_chain"`). Resolves all models in the chain sorted by priority. Each candidate is validated independently. Invalid candidates are skipped with a reason.

### Tier 3: Fallback chain walk

Caller specifies only a capability (e.g. `capability: Capability::Embed`). The selection service finds the chain configured for that capability and falls back to Tier 2 logic.

### Candidate validation

Each candidate passes through four checks before it is eligible:

1. **Router validation** -- adapter exists, is enabled, API key available if required.
2. **Model validation** -- model exists in config, supports the requested capability.
3. **Availability check** -- circuit breaker is not in Open state.
4. **Budget check** -- estimated cost is within the remaining budget.

Skipped candidates carry a `SkipReason` (e.g. `RouterDisabled`, `CircuitBreakerOpen`, `OverBudget`) that is included in execution traces for debugging.

## Adapters

### Trait

```rust
#[async_trait]
pub trait InferenceAdapter: Send + Sync {
    fn id(&self) -> &str;
    fn supports(&self, capability: &Capability) -> bool;
    async fn execute(&self, config: &RouterConfig, request: &InferenceRequest)
        -> Result<InferenceResponse, GatewayError>;
    async fn stream(&self, config: &RouterConfig, request: &InferenceRequest)
        -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, GatewayError>> + Send>>, GatewayError>;
}
```

`Send + Sync` is required because adapters live in `Arc` and are accessed from multiple tokio tasks.

### Implementations

| Adapter | Provider | Notes |
|---------|----------|-------|
| `OllamaAdapter` | Ollama | OpenAI-compatible endpoints, zero cost |
| `AnthropicAdapter` | Anthropic | Request/response translation to Anthropic format |
| `OpenAIAdapter` | OpenAI | Also covers OpenRouter and compatible APIs |
| `NoopAdapter` | None | Graceful degradation -- returns empty response when no providers are available |

### Registry

`AdapterRegistry` maps adapter IDs to `Arc<dyn InferenceAdapter>` instances. The engine looks up adapters by ID during candidate walking.

## Circuit breaker

Per-endpoint failure tracking with a state machine that prevents hammering a down provider.

### State machine

```
CLOSED (normal)  -- failures >= threshold -->  OPEN (blocked)
                                                  |
                                            timeout expires
                                                  |
                                                  v
CLOSED  <-- successes >= halfOpenMax --  HALF_OPEN (test)
                                            |
                                       any failure --> OPEN
```

### Configuration

- `threshold` -- failures before opening (default: 5).
- `timeout` -- how long to stay open (default: 5 minutes).
- `half_open_max_requests` -- successful test requests needed to close (default: 3).

### Endpoint tracking

Each breaker tracks `{adapter}:{model}` independently (e.g. `ollama:gemma3:27b` and `ollama:qwen3:14b` are separate). State is in-memory only -- restart resets all breakers to Closed. Lazy initialization creates a Closed state on first access.

### Auto-recovery

When the Open timeout expires, the breaker transitions to HalfOpen and allows test requests. If enough succeed, the breaker closes. A single failure in HalfOpen reopens immediately.

## Budget

### Cost estimation

Before executing, the gateway estimates cost to decide if a model is affordable:

```rust
CostEstimate {
    estimated: (input_tokens * pricing.input_per_1k / 1000)
             + (max_output_tokens * pricing.output_per_1k / 1000),
    minimum: input_tokens * pricing.input_per_1k / 1000,
    maximum: estimated,   // upper bound uses max_output_tokens
    currency: "USD",
    model: "...",
}
```

Local models (Ollama) always have zero cost and always pass the budget filter.

### Daily and monthly limits

```rust
pub struct Budget {
    pub daily_limit: f64,       // e.g. $5.00/day
    pub monthly_limit: f64,     // e.g. $50/month
    pub alert_threshold: f32,   // e.g. 0.8 = alert at 80% usage
}
```

### Spend tracking

Every inference call records its cost. An in-memory `SpendCache` with atomic counters avoids querying the database on every call. The cache increments after each call and periodically syncs with the database.

### Degradation

| Condition | Action |
|-----------|--------|
| `daily_spend < daily_limit` | Normal routing |
| `daily_spend >= daily_limit * alert_threshold` | Log warning, continue |
| `daily_spend >= daily_limit` | Skip external providers, local only |
| `monthly_spend >= monthly_limit` | Skip external providers, local only |
| No local models available + budget exceeded | Noop response |

**Never block. Always degrade.** Budget status is exposed via the `gateway_status` MCP tool and the daemon health endpoint.

## Consensus

The MOE (Mixture of Experts) consensus protocol is a **caller of the gateway**, not part of the engine. This keeps the engine clean -- it routes single requests. Consensus composes multiple requests into a protocol.

```rust
impl ConsensusEngine {
    pub async fn run(&self, gateway: &Gateway, config: &ConsensusConfig, request: ConsensusRequest)
        -> Result<ConsensusResult, GatewayError>
    {
        // Step 1: Propose (calls gateway.execute with proposer model)
        // Step 2: Challenge (calls gateway.execute with challenger model)
        // Step 3: Synthesize (calls gateway.execute with synthesizer model)
    }
}
```

The synthesizer produces a weighted synthesis of the propose and challenge phases, yielding a final result with confidence metadata.

## Data model

The gateway crate defines a `GatewayStore` trait for persistence; consumers implement it. Sensei implements this on its PostgreSQL store. Two tables:

### gateway.inference_calls

Every gateway call is recorded for budget tracking and analytics.

| Column | Type | Purpose |
|--------|------|---------|
| `id` | uuid PK | |
| `session_id` | uuid | FK managed by consumer |
| `project_id` | uuid | FK managed by consumer |
| `capability` | text | Chat, Embed, etc. |
| `chain_id` | text | Named chain used |
| `adapter` | text | Provider adapter ID |
| `model` | text | Model name |
| `api_model_id` | text | Actual API model ID if different |
| `input_tokens` | integer | |
| `output_tokens` | integer | |
| `cost_usd` | numeric(10,6) | |
| `duration_ms` | integer | |
| `status` | text | success / failed |
| `error_type` | text | Error variant if failed |
| `fallback_sequence` | smallint | Position in fallback chain |
| `request_metadata` | jsonb | |
| `response_metadata` | jsonb | |
| `recorded_at` | timestamptz | |

**Indexes**: `project_id`, `session_id`, `recorded_at`, `model`, `capability`.

### gateway.execution_traces

Full execution trace for debugging. One per gateway call.

| Column | Type | Purpose |
|--------|------|---------|
| `id` | uuid PK | |
| `inference_call_id` | uuid FK | References inference_calls |
| `request_id` | text | |
| `capability` | text | |
| `status` | text | |
| `duration_ms` | integer | |
| `candidates` | jsonb | Resolved candidate list |
| `skipped` | jsonb | Skipped candidates with reasons |
| `attempts` | jsonb | Each attempt with timing and result |
| `estimated_cost` | jsonb | |
| `actual_cost` | jsonb | |
| `created_at` | timestamptz | |

**Indexes**: `inference_call_id`, `status`, `created_at`.

### Storage trait

```rust
#[async_trait]
pub trait GatewayStore: Send + Sync {
    async fn insert_inference_call(&self, call: &InferenceCall) -> Result<Uuid>;
    async fn insert_execution_trace(&self, trace: &ExecutionTrace) -> Result<Uuid>;
    async fn get_spend_since(&self, since: DateTime<Utc>) -> Result<f64>;
    async fn get_spend_by_model_since(&self, since: DateTime<Utc>) -> Result<Vec<(String, f64)>>;
}
```

## Integration

### Daemon lifecycle

The daemon (`crates/senseid`) owns the gateway instance. On startup, it builds the gateway from database-backed config, registers adapters, and shares the `Arc<Gateway>` with API handlers. Config changes are applied via `gateway.update_config()` with no restart required.

### Health endpoints

The daemon exposes `GET /api/health` which includes gateway status: adapter availability, circuit breaker states, and budget summary (daily/monthly spend and limits).

### MCP exposure

The MCP server (`crates/mcp`) exposes gateway capabilities as tools:
- `infer` -- single inference call through the gateway.
- `embed` -- embedding generation.
- `consensus` -- MOE consensus protocol.
- `gateway_status` -- budget, adapter health, circuit breaker states.

These tools call the daemon HTTP API, which delegates to the gateway instance.
