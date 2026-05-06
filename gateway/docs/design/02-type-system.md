---
name: Type System
description: Core types, error handling, serde strategy, Rust trait design for the gateway
date: 2026-04-24
status: design
reference: /Users/Jerry/Developer/strategos/packages/core/src/types/
---

# Type System

## TypeScript → Rust translation

| TypeScript | Rust |
|------------|------|
| `interface` | `struct` (data) or `trait` (behavior) |
| `type Union = A \| B` | `enum { A, B }` |
| `Promise<T>` | `async fn -> Result<T, E>` |
| `AsyncGenerator<T>` | `impl Stream<Item = Result<T, E>>` |
| `Record<K, V>` | `HashMap<K, V>` |
| `Optional / undefined` | `Option<T>` |
| Class with state | `struct` + `impl` |
| Mutable shared state | `Arc<Mutex<T>>` or `Arc<RwLock<T>>` |

## Capability enum

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

Match at compile time — no stringly-typed capability checks.

## Error enum

```rust
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("authentication failed for {adapter}: {message}")]
    Authentication { adapter: String, message: String },

    #[error("rate limited by {adapter}: retry after {retry_after_ms}ms")]
    RateLimit { adapter: String, retry_after_ms: Option<u64> },

    #[error("budget exceeded: estimated ${estimated:.4}, remaining ${remaining:.4}")]
    BudgetExceeded { estimated: f64, remaining: f64 },

    #[error("request timeout after {duration_ms}ms to {adapter}:{model}")]
    Timeout { adapter: String, model: String, duration_ms: u64 },

    #[error("provider error from {adapter}: {message}")]
    ProviderError { adapter: String, message: String, status: Option<u16> },

    #[error("model {model} not available on {adapter}")]
    ModelUnavailable { adapter: String, model: String },

    #[error("no candidates available for capability {capability:?}")]
    NoCandidates { capability: Capability },

    #[error("all {attempts} attempts failed")]
    AllAttemptsFailed { attempts: usize },

    #[error("network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

impl GatewayError {
    pub fn is_retryable(&self) -> bool {
        matches!(self,
            Self::RateLimit { .. }
            | Self::Timeout { .. }
            | Self::ProviderError { .. }
            | Self::ModelUnavailable { .. }
            | Self::Network(_)
        )
    }

    pub fn should_trigger_fallback(&self, triggers: &[FallbackTrigger]) -> bool {
        match self {
            Self::RateLimit { .. } => triggers.contains(&FallbackTrigger::RateLimit),
            Self::Timeout { .. } => triggers.contains(&FallbackTrigger::Timeout),
            Self::ProviderError { .. } => triggers.contains(&FallbackTrigger::ProviderError),
            Self::ModelUnavailable { .. } => triggers.contains(&FallbackTrigger::ModelUnavailable),
            Self::BudgetExceeded { .. } => triggers.contains(&FallbackTrigger::BudgetExceeded),
            _ => false,
        }
    }
}
```

## Serde strategy

All types derive `Serialize, Deserialize` for:
- JSON storage in PostgreSQL (JSONB columns)
- API request/response bodies
- Config file loading (future)
- Execution trace persistence

Naming convention: `#[serde(rename_all = "snake_case")]` everywhere for consistency.

## Adapter trait

```rust
#[async_trait]
pub trait InferenceAdapter: Send + Sync {
    fn id(&self) -> &str;
    fn supports(&self, capability: &Capability) -> bool;

    async fn execute(
        &self,
        config: &RouterConfig,
        request: &InferenceRequest,
    ) -> Result<InferenceResponse, GatewayError>;

    async fn stream(
        &self,
        config: &RouterConfig,
        request: &InferenceRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<StreamChunk, GatewayError>> + Send>>, GatewayError>;
}
```

`Send + Sync` required because adapters live in `Arc` and are accessed from multiple tokio tasks.

## Stream types

```rust
pub enum StreamEvent {
    Chunk {
        content: String,
    },
    ProviderSwitch {
        from_adapter: String,
        from_model: String,
        to_adapter: String,
        to_model: String,
        reason: String,
    },
    Done {
        model: String,
        tokens: TokenUsage,
        cost: f64,
    },
    Error {
        code: String,
        message: String,
    },
}

pub struct StreamChunk {
    pub content: String,
    pub finish_reason: Option<String>,
    pub usage: Option<TokenUsage>,
}
```

## Token usage

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}
```

## Fallback triggers

```rust
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackTrigger {
    RateLimit,
    Timeout,
    ProviderError,
    ModelUnavailable,
    BudgetExceeded,
}
```
