# Gateway Phase 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the gateway crate — a standalone Rust LLM routing engine with fallback chains, circuit breaker, budget management, and adapter abstraction. Phase 1 delivers types, circuit breaker, noop adapter, model selection, budget filtering, engine, and config builder — all fully tested without requiring external services.

**Architecture:** Standalone `crates/gateway/` Rust library crate. The gateway routes inference requests through fallback chains of model candidates, handling provider failures via circuit breaker, cost filtering via budget, and graceful degradation via noop adapter. No sensei-specific dependencies. Ported from TypeScript Strategos gateway at `/Users/Jerry/Developer/strategos/packages/gateway`.

**Tech Stack:** Rust, tokio, reqwest, serde, async-trait, thiserror, futures, uuid, chrono, tracing

---

## File Structure

```
crates/gateway/
├── Cargo.toml
└── src/
    ├── lib.rs                  — re-exports, public API surface
    ├── types/
    │   ├── mod.rs              — re-exports all type modules
    │   ├── capability.rs       — Capability enum
    │   ├── config.rs           — RouterConfig, ModelConfig, FallbackChainConfig, GatewayConfig
    │   ├── request.rs          — InferenceRequest, InferenceResponse, Message, Payload types
    │   ├── cost.rs             — Cost, CostEstimate, ModelPricing, TokenUsage, Budget
    │   ├── error.rs            — GatewayError enum with thiserror
    │   └── trace.rs            — ExecutionTrace, Attempt, TaskStartEvent, AttemptEvent, TaskEndEvent
    ├── circuit_breaker.rs      — CircuitBreakerManager, BreakerState, state machine
    ├── adapters/
    │   ├── mod.rs              — InferenceAdapter trait, AdapterRegistry
    │   └── noop.rs             — NoopAdapter (graceful degradation)
    ├── selection.rs            — ModelSelectionService, 3-tier resolution, validation pipeline
    ├── budget.rs               — estimate_cost, filter_by_budget, BudgetFilterResult
    ├── engine.rs               — Gateway struct, execute(), fallback chain walking
    └── config.rs               — GatewayBuilder, validation, defaults
```

Each file has one clear responsibility. Tests live in `#[cfg(test)] mod tests` within each file.

---

### Task 1: Project scaffolding

**Files:**
- Create: `crates/gateway/Cargo.toml`
- Create: `crates/gateway/src/lib.rs`
- Create: `.gitignore`

- [ ] **Step 1: Create Cargo.toml**

```toml
[package]
name = "gateway"
version = "0.1.0"
edition = "2024"
description = "LLM inference routing engine — fallback chains, circuit breaker, budget management"
license = "MIT"

[dependencies]
async-trait = "0.1"
chrono = { version = "0.4", features = ["serde"] }
futures = "0.3"
pin-project-lite = "0.2"
reqwest = { version = "0.12", features = ["json", "stream"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
uuid = { version = "1", features = ["v4", "serde"] }

[dev-dependencies]
tokio = { version = "1", features = ["full", "test-util"] }
```

- [ ] **Step 2: Create lib.rs stub**

```rust
pub mod types;
```

- [ ] **Step 3: Create types/mod.rs stub**

```rust
pub mod capability;
```

- [ ] **Step 4: Create minimal capability.rs to verify compilation**

```rust
use serde::{Deserialize, Serialize};

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

- [ ] **Step 5: Create .gitignore**

```
/target
Cargo.lock
```

- [ ] **Step 6: Verify it compiles**

Run: `cargo check --manifest-path crates/gateway/Cargo.toml`
Expected: compiles with no errors

- [ ] **Step 7: Commit**

```bash
git add .gitignore crates/gateway/Cargo.toml crates/gateway/src/lib.rs crates/gateway/src/types/mod.rs crates/gateway/src/types/capability.rs
git commit -m "feat: scaffold gateway crate with Capability enum"
```

---

### Task 2: Core types — config

**Files:**
- Create: `crates/gateway/src/types/config.rs`
- Modify: `crates/gateway/src/types/mod.rs`

- [ ] **Step 1: Write failing tests for config types**

In `crates/gateway/src/types/config.rs`:

```rust
use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use super::capability::Capability;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_model_id: Option<String>,
    pub provider: String,
    pub capabilities: Vec<Capability>,
    pub context_window: u32,
    pub max_output_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pricing: Option<ModelPricing>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelPricing {
    pub input_per_1k: f64,
    pub output_per_1k: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub per_request: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FallbackTrigger {
    RateLimit,
    Timeout,
    ProviderError,
    ModelUnavailable,
    BudgetExceeded,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainEntry {
    pub model: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub router: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_model_id: Option<String>,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FallbackChainConfig {
    pub id: String,
    pub capability: Capability,
    pub models: Vec<ChainEntry>,
    pub fallback_triggers: Vec<FallbackTrigger>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GatewayConfig {
    pub routers: HashMap<String, RouterConfig>,
    pub models: HashMap<String, ModelConfig>,
    pub chains: HashMap<String, FallbackChainConfig>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn router_config_serde_roundtrip() {
        let config = RouterConfig {
            url: "http://localhost:11434".into(),
            api_key_env: None,
            enabled: true,
            timeout_ms: Some(30_000),
            headers: HashMap::new(),
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: RouterConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.url, "http://localhost:11434");
        assert!(parsed.enabled);
        assert_eq!(parsed.timeout_ms, Some(30_000));
    }

    #[test]
    fn router_config_defaults() {
        let json = r#"{"url": "http://localhost:11434"}"#;
        let config: RouterConfig = serde_json::from_str(json).unwrap();
        assert!(config.enabled);
        assert!(config.headers.is_empty());
    }

    #[test]
    fn model_config_serde_roundtrip() {
        let config = ModelConfig {
            id: "gemma3:27b".into(),
            api_model_id: None,
            provider: "ollama".into(),
            capabilities: vec![Capability::Chat, Capability::Embed],
            context_window: 8192,
            max_output_tokens: 2048,
            pricing: None,
        };
        let json = serde_json::to_string(&config).unwrap();
        let parsed: ModelConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "gemma3:27b");
        assert_eq!(parsed.capabilities.len(), 2);
    }

    #[test]
    fn fallback_chain_serde_roundtrip() {
        let chain = FallbackChainConfig {
            id: "embed_chain".into(),
            capability: Capability::Embed,
            models: vec![
                ChainEntry {
                    model: "all-minilm".into(),
                    router: Some("ollama".into()),
                    api_model_id: None,
                    priority: 1,
                },
            ],
            fallback_triggers: vec![FallbackTrigger::Timeout, FallbackTrigger::ModelUnavailable],
        };
        let json = serde_json::to_string(&chain).unwrap();
        let parsed: FallbackChainConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "embed_chain");
        assert_eq!(parsed.models.len(), 1);
        assert_eq!(parsed.fallback_triggers.len(), 2);
    }

    #[test]
    fn fallback_trigger_snake_case_serde() {
        let trigger = FallbackTrigger::RateLimit;
        let json = serde_json::to_string(&trigger).unwrap();
        assert_eq!(json, r#""rate_limit""#);
    }

    #[test]
    fn gateway_config_default_is_empty() {
        let config = GatewayConfig::default();
        assert!(config.routers.is_empty());
        assert!(config.models.is_empty());
        assert!(config.chains.is_empty());
    }
}
```

- [ ] **Step 2: Add config to types/mod.rs**

```rust
pub mod capability;
pub mod config;
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path crates/gateway/Cargo.toml -- types::config`
Expected: all 5 tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/gateway/src/types/config.rs crates/gateway/src/types/mod.rs
git commit -m "feat: add config types — RouterConfig, ModelConfig, FallbackChainConfig"
```

---

### Task 3: Core types — cost

**Files:**
- Create: `crates/gateway/src/types/cost.rs`
- Modify: `crates/gateway/src/types/mod.rs`

- [ ] **Step 1: Write cost types with tests**

In `crates/gateway/src/types/cost.rs`:

```rust
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TokenUsage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cost {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
    pub input_cost: f64,
    pub output_cost: f64,
    pub total_cost: f64,
    pub currency: String,
}

impl Cost {
    pub fn zero() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            total_tokens: 0,
            input_cost: 0.0,
            output_cost: 0.0,
            total_cost: 0.0,
            currency: "USD".into(),
        }
    }

    pub fn from_usage(usage: &TokenUsage, input_per_1k: f64, output_per_1k: f64) -> Self {
        let input_cost = usage.input_tokens as f64 * input_per_1k / 1000.0;
        let output_cost = usage.output_tokens as f64 * output_per_1k / 1000.0;
        Self {
            input_tokens: usage.input_tokens,
            output_tokens: usage.output_tokens,
            total_tokens: usage.total_tokens,
            input_cost,
            output_cost,
            total_cost: input_cost + output_cost,
            currency: "USD".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CostEstimate {
    pub estimated: f64,
    pub minimum: f64,
    pub maximum: f64,
    pub currency: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Budget {
    pub daily_limit: f64,
    pub monthly_limit: f64,
    #[serde(default = "default_alert_threshold")]
    pub alert_threshold: f32,
}

fn default_alert_threshold() -> f32 {
    0.8
}

impl Default for Budget {
    fn default() -> Self {
        Self {
            daily_limit: 5.0,
            monthly_limit: 50.0,
            alert_threshold: 0.8,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cost_zero() {
        let cost = Cost::zero();
        assert_eq!(cost.total_cost, 0.0);
        assert_eq!(cost.currency, "USD");
    }

    #[test]
    fn cost_from_usage() {
        let usage = TokenUsage {
            input_tokens: 1000,
            output_tokens: 500,
            total_tokens: 1500,
        };
        // claude-haiku pricing: $0.0008/1K input, $0.004/1K output
        let cost = Cost::from_usage(&usage, 0.0008, 0.004);
        assert!((cost.input_cost - 0.0008).abs() < 1e-10);
        assert!((cost.output_cost - 0.002).abs() < 1e-10);
        assert!((cost.total_cost - 0.0028).abs() < 1e-10);
    }

    #[test]
    fn cost_from_usage_zero_tokens() {
        let usage = TokenUsage::default();
        let cost = Cost::from_usage(&usage, 0.003, 0.015);
        assert_eq!(cost.total_cost, 0.0);
    }

    #[test]
    fn budget_default() {
        let budget = Budget::default();
        assert_eq!(budget.daily_limit, 5.0);
        assert_eq!(budget.monthly_limit, 50.0);
        assert_eq!(budget.alert_threshold, 0.8);
    }

    #[test]
    fn cost_estimate_serde_roundtrip() {
        let estimate = CostEstimate {
            estimated: 0.015,
            minimum: 0.008,
            maximum: 0.025,
            currency: "USD".into(),
            model: "claude-sonnet-4-6".into(),
        };
        let json = serde_json::to_string(&estimate).unwrap();
        let parsed: CostEstimate = serde_json::from_str(&json).unwrap();
        assert!((parsed.estimated - 0.015).abs() < 1e-10);
        assert_eq!(parsed.model, "claude-sonnet-4-6");
    }
}
```

- [ ] **Step 2: Add cost to types/mod.rs**

```rust
pub mod capability;
pub mod config;
pub mod cost;
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path crates/gateway/Cargo.toml -- types::cost`
Expected: all 5 tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/gateway/src/types/cost.rs crates/gateway/src/types/mod.rs
git commit -m "feat: add cost types — Cost, CostEstimate, TokenUsage, Budget"
```

---

### Task 4: Core types — error

**Files:**
- Create: `crates/gateway/src/types/error.rs`
- Modify: `crates/gateway/src/types/mod.rs`

- [ ] **Step 1: Write error enum with tests**

In `crates/gateway/src/types/error.rs`:

```rust
use super::capability::Capability;
use super::config::FallbackTrigger;

#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    #[error("authentication failed for {adapter}: {message}")]
    Authentication { adapter: String, message: String },

    #[error("rate limited by {adapter}: retry after {retry_after_ms:?}ms")]
    RateLimit {
        adapter: String,
        retry_after_ms: Option<u64>,
    },

    #[error("budget exceeded: estimated ${estimated:.4}, remaining ${remaining:.4}")]
    BudgetExceeded { estimated: f64, remaining: f64 },

    #[error("request timeout after {duration_ms}ms to {adapter}:{model}")]
    Timeout {
        adapter: String,
        model: String,
        duration_ms: u64,
    },

    #[error("provider error from {adapter}: {message}")]
    ProviderError {
        adapter: String,
        message: String,
        status: Option<u16>,
    },

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
        matches!(
            self,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display_messages() {
        let err = GatewayError::Authentication {
            adapter: "anthropic".into(),
            message: "invalid key".into(),
        };
        assert_eq!(
            err.to_string(),
            "authentication failed for anthropic: invalid key"
        );

        let err = GatewayError::Timeout {
            adapter: "ollama".into(),
            model: "gemma3:27b".into(),
            duration_ms: 30000,
        };
        assert_eq!(
            err.to_string(),
            "request timeout after 30000ms to ollama:gemma3:27b"
        );
    }

    #[test]
    fn is_retryable() {
        assert!(GatewayError::RateLimit {
            adapter: "x".into(),
            retry_after_ms: None
        }
        .is_retryable());

        assert!(GatewayError::Timeout {
            adapter: "x".into(),
            model: "y".into(),
            duration_ms: 0
        }
        .is_retryable());

        assert!(!GatewayError::Authentication {
            adapter: "x".into(),
            message: "bad key".into()
        }
        .is_retryable());

        assert!(!GatewayError::BudgetExceeded {
            estimated: 1.0,
            remaining: 0.5
        }
        .is_retryable());
    }

    #[test]
    fn should_trigger_fallback_matches_triggers() {
        let triggers = vec![FallbackTrigger::Timeout, FallbackTrigger::RateLimit];

        let timeout = GatewayError::Timeout {
            adapter: "x".into(),
            model: "y".into(),
            duration_ms: 0,
        };
        assert!(timeout.should_trigger_fallback(&triggers));

        let rate_limit = GatewayError::RateLimit {
            adapter: "x".into(),
            retry_after_ms: None,
        };
        assert!(rate_limit.should_trigger_fallback(&triggers));

        let provider_err = GatewayError::ProviderError {
            adapter: "x".into(),
            message: "500".into(),
            status: Some(500),
        };
        assert!(!provider_err.should_trigger_fallback(&triggers));
    }

    #[test]
    fn should_trigger_fallback_empty_triggers() {
        let err = GatewayError::Timeout {
            adapter: "x".into(),
            model: "y".into(),
            duration_ms: 0,
        };
        assert!(!err.should_trigger_fallback(&[]));
    }

    #[test]
    fn auth_error_never_triggers_fallback() {
        let all_triggers = vec![
            FallbackTrigger::RateLimit,
            FallbackTrigger::Timeout,
            FallbackTrigger::ProviderError,
            FallbackTrigger::ModelUnavailable,
            FallbackTrigger::BudgetExceeded,
        ];
        let err = GatewayError::Authentication {
            adapter: "x".into(),
            message: "bad".into(),
        };
        assert!(!err.should_trigger_fallback(&all_triggers));
    }
}
```

- [ ] **Step 2: Add error to types/mod.rs**

```rust
pub mod capability;
pub mod config;
pub mod cost;
pub mod error;
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path crates/gateway/Cargo.toml -- types::error`
Expected: all 4 tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/gateway/src/types/error.rs crates/gateway/src/types/mod.rs
git commit -m "feat: add GatewayError enum with fallback trigger matching"
```

---

### Task 5: Core types — request and trace

**Files:**
- Create: `crates/gateway/src/types/request.rs`
- Create: `crates/gateway/src/types/trace.rs`
- Modify: `crates/gateway/src/types/mod.rs`

- [ ] **Step 1: Write request types**

In `crates/gateway/src/types/request.rs`:

```rust
use serde::{Deserialize, Serialize};
use super::capability::Capability;
use super::cost::{Cost, CostEstimate, TokenUsage};
use super::trace::{Attempt, ExecutionTrace};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Payload {
    Chat {
        messages: Vec<Message>,
        #[serde(skip_serializing_if = "Option::is_none")]
        system: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        max_tokens: Option<u32>,
        #[serde(skip_serializing_if = "Option::is_none")]
        temperature: Option<f32>,
    },
    Embed {
        texts: Vec<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceRequest {
    pub capability: Capability,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub router: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chain: Option<String>,
    pub payload: Payload,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub budget: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InferenceResponse {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embeddings: Option<Vec<Vec<f32>>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<TokenUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_cost: Option<CostEstimate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_cost: Option<Cost>,
    pub attempts: Vec<Attempt>,
}

#[derive(Debug, Clone)]
pub struct StreamChunk {
    pub content: String,
    pub finish_reason: Option<String>,
    pub usage: Option<TokenUsage>,
}

#[derive(Debug, Clone)]
pub enum StreamEvent {
    Chunk { content: String },
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
    Error { code: String, message: String },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chat_request_serde() {
        let req = InferenceRequest {
            capability: Capability::Chat,
            model: Some("gemma3:27b".into()),
            router: None,
            chain: None,
            payload: Payload::Chat {
                messages: vec![Message {
                    role: MessageRole::User,
                    content: "hello".into(),
                    tool_call_id: None,
                }],
                system: None,
                max_tokens: Some(1024),
                temperature: None,
            },
            budget: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        let parsed: InferenceRequest = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.capability, Capability::Chat);
        assert_eq!(parsed.model.as_deref(), Some("gemma3:27b"));
    }

    #[test]
    fn embed_request_serde() {
        let req = InferenceRequest {
            capability: Capability::Embed,
            model: None,
            router: None,
            chain: None,
            payload: Payload::Embed {
                texts: vec!["hello world".into()],
            },
            budget: None,
        };
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains(r#""type":"embed"#));
    }

    #[test]
    fn response_with_attempts() {
        let resp = InferenceResponse {
            success: true,
            content: Some("hello".into()),
            embeddings: None,
            model: Some("gemma3:27b".into()),
            usage: Some(TokenUsage {
                input_tokens: 10,
                output_tokens: 5,
                total_tokens: 15,
            }),
            estimated_cost: None,
            actual_cost: None,
            attempts: vec![],
        };
        let json = serde_json::to_string(&resp).unwrap();
        let parsed: InferenceResponse = serde_json::from_str(&json).unwrap();
        assert!(parsed.success);
    }
}
```

- [ ] **Step 2: Write trace types**

In `crates/gateway/src/types/trace.rs`:

```rust
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use super::capability::Capability;
use super::cost::{Cost, CostEstimate, TokenUsage};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TraceStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AttemptStatus {
    Success,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Attempt {
    pub sequence: u8,
    pub adapter: String,
    pub model: String,
    pub api_model_id: String,
    pub status: AttemptStatus,
    pub duration_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens: Option<TokenUsage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub fallback_triggered: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateInfo {
    pub model: String,
    pub router: String,
    pub priority: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkippedInfo {
    pub model: String,
    pub router: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionTrace {
    pub request_id: String,
    pub capability: Capability,
    pub status: TraceStatus,
    pub duration_ms: u64,
    pub candidates: Vec<CandidateInfo>,
    pub skipped: Vec<SkippedInfo>,
    pub attempts: Vec<Attempt>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_cost: Option<CostEstimate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_cost: Option<Cost>,
    pub created_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attempt_serde_roundtrip() {
        let attempt = Attempt {
            sequence: 0,
            adapter: "ollama".into(),
            model: "gemma3:27b".into(),
            api_model_id: "gemma3:27b".into(),
            status: AttemptStatus::Success,
            duration_ms: 1200,
            tokens: Some(TokenUsage {
                input_tokens: 100,
                output_tokens: 50,
                total_tokens: 150,
            }),
            cost: Some(0.0),
            error: None,
            fallback_triggered: false,
        };
        let json = serde_json::to_string(&attempt).unwrap();
        let parsed: Attempt = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.adapter, "ollama");
        assert_eq!(parsed.status, AttemptStatus::Success);
    }

    #[test]
    fn trace_status_serde() {
        let status = TraceStatus::Failed;
        let json = serde_json::to_string(&status).unwrap();
        assert_eq!(json, r#""failed""#);
    }
}
```

- [ ] **Step 3: Update types/mod.rs**

```rust
pub mod capability;
pub mod config;
pub mod cost;
pub mod error;
pub mod request;
pub mod trace;
```

- [ ] **Step 4: Run all type tests**

Run: `cargo test --manifest-path crates/gateway/Cargo.toml -- types`
Expected: all tests PASS

- [ ] **Step 5: Commit**

```bash
git add crates/gateway/src/types/
git commit -m "feat: add request, response, trace, and stream types"
```

---

### Task 6: Circuit breaker

**Files:**
- Create: `crates/gateway/src/circuit_breaker.rs`
- Modify: `crates/gateway/src/lib.rs`

- [ ] **Step 1: Write circuit breaker with full test suite**

In `crates/gateway/src/circuit_breaker.rs`:

```rust
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub threshold: usize,
    pub timeout: Duration,
    pub half_open_max_requests: usize,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            threshold: 5,
            timeout: Duration::from_secs(300),
            half_open_max_requests: 3,
        }
    }
}

#[derive(Debug, Clone)]
pub enum BreakerState {
    Closed { failure_count: usize },
    Open { next_retry: Instant },
    HalfOpen { success_count: usize },
}

impl BreakerState {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Closed { .. } => "closed",
            Self::Open { .. } => "open",
            Self::HalfOpen { .. } => "half_open",
        }
    }
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerManager {
    states: Arc<Mutex<HashMap<String, BreakerState>>>,
    config: CircuitBreakerConfig,
}

impl CircuitBreakerManager {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            states: Arc::new(Mutex::new(HashMap::new())),
            config,
        }
    }

    pub fn can_execute(&self, endpoint: &str) -> bool {
        let mut states = self.states.lock().unwrap();
        let state = states
            .entry(endpoint.to_string())
            .or_insert(BreakerState::Closed { failure_count: 0 });

        match state {
            BreakerState::Closed { .. } => true,
            BreakerState::Open { next_retry } => {
                if Instant::now() >= *next_retry {
                    *state = BreakerState::HalfOpen { success_count: 0 };
                    true
                } else {
                    false
                }
            }
            BreakerState::HalfOpen { .. } => true,
        }
    }

    pub fn record_success(&self, endpoint: &str) {
        let mut states = self.states.lock().unwrap();
        let state = states
            .entry(endpoint.to_string())
            .or_insert(BreakerState::Closed { failure_count: 0 });

        match state {
            BreakerState::Closed { failure_count } => {
                *failure_count = 0;
            }
            BreakerState::HalfOpen { success_count } => {
                *success_count += 1;
                if *success_count >= self.config.half_open_max_requests {
                    *state = BreakerState::Closed { failure_count: 0 };
                }
            }
            BreakerState::Open { .. } => {}
        }
    }

    pub fn record_failure(&self, endpoint: &str) {
        let mut states = self.states.lock().unwrap();
        let state = states
            .entry(endpoint.to_string())
            .or_insert(BreakerState::Closed { failure_count: 0 });

        match state {
            BreakerState::Closed { failure_count } => {
                *failure_count += 1;
                if *failure_count >= self.config.threshold {
                    *state = BreakerState::Open {
                        next_retry: Instant::now() + self.config.timeout,
                    };
                }
            }
            BreakerState::HalfOpen { .. } => {
                *state = BreakerState::Open {
                    next_retry: Instant::now() + self.config.timeout,
                };
            }
            BreakerState::Open { .. } => {}
        }
    }

    pub fn get_state(&self, endpoint: &str) -> BreakerState {
        let states = self.states.lock().unwrap();
        states
            .get(endpoint)
            .cloned()
            .unwrap_or(BreakerState::Closed { failure_count: 0 })
    }

    pub fn reset(&self, endpoint: &str) {
        let mut states = self.states.lock().unwrap();
        states.remove(endpoint);
    }

    pub fn reset_all(&self) {
        let mut states = self.states.lock().unwrap();
        states.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_manager(threshold: usize, timeout_ms: u64) -> CircuitBreakerManager {
        CircuitBreakerManager::new(CircuitBreakerConfig {
            threshold,
            timeout: Duration::from_millis(timeout_ms),
            half_open_max_requests: 3,
        })
    }

    #[test]
    fn starts_closed_allows_execution() {
        let mgr = make_manager(5, 5000);
        assert!(mgr.can_execute("ollama:gemma3"));
        assert_eq!(mgr.get_state("ollama:gemma3").name(), "closed");
    }

    #[test]
    fn stays_closed_below_threshold() {
        let mgr = make_manager(5, 5000);
        for _ in 0..4 {
            mgr.record_failure("ep");
        }
        assert!(mgr.can_execute("ep"));
        assert_eq!(mgr.get_state("ep").name(), "closed");
    }

    #[test]
    fn transitions_to_open_at_threshold() {
        let mgr = make_manager(3, 5000);
        for _ in 0..3 {
            mgr.record_failure("ep");
        }
        assert!(!mgr.can_execute("ep"));
        assert_eq!(mgr.get_state("ep").name(), "open");
    }

    #[test]
    fn open_blocks_execution() {
        let mgr = make_manager(1, 60_000);
        mgr.record_failure("ep");
        assert!(!mgr.can_execute("ep"));
        assert!(!mgr.can_execute("ep"));
    }

    #[test]
    fn open_transitions_to_half_open_after_timeout() {
        let mgr = make_manager(1, 0); // 0ms timeout = instant retry
        mgr.record_failure("ep");
        // timeout is 0ms so next_retry is already in the past
        assert!(mgr.can_execute("ep"));
        assert_eq!(mgr.get_state("ep").name(), "half_open");
    }

    #[test]
    fn half_open_transitions_to_closed_after_successes() {
        let mgr = make_manager(1, 0);
        mgr.record_failure("ep");
        mgr.can_execute("ep"); // transitions to half_open

        mgr.record_success("ep");
        mgr.record_success("ep");
        mgr.record_success("ep"); // 3 successes = half_open_max_requests
        assert_eq!(mgr.get_state("ep").name(), "closed");
    }

    #[test]
    fn half_open_transitions_to_open_on_failure() {
        let mgr = make_manager(1, 0);
        mgr.record_failure("ep");
        mgr.can_execute("ep"); // half_open

        mgr.record_failure("ep");
        assert_eq!(mgr.get_state("ep").name(), "open");
    }

    #[test]
    fn success_resets_failure_count_in_closed() {
        let mgr = make_manager(3, 5000);
        mgr.record_failure("ep");
        mgr.record_failure("ep");
        mgr.record_success("ep"); // resets count
        mgr.record_failure("ep");
        mgr.record_failure("ep");
        // only 2 failures since reset, threshold is 3
        assert!(mgr.can_execute("ep"));
    }

    #[test]
    fn independent_endpoints() {
        let mgr = make_manager(1, 60_000);
        mgr.record_failure("ep1");
        assert!(!mgr.can_execute("ep1"));
        assert!(mgr.can_execute("ep2")); // ep2 is independent
    }

    #[test]
    fn reset_clears_single_endpoint() {
        let mgr = make_manager(1, 60_000);
        mgr.record_failure("ep1");
        mgr.record_failure("ep2");
        mgr.reset("ep1");
        assert!(mgr.can_execute("ep1"));
        assert!(!mgr.can_execute("ep2"));
    }

    #[test]
    fn reset_all_clears_everything() {
        let mgr = make_manager(1, 60_000);
        mgr.record_failure("ep1");
        mgr.record_failure("ep2");
        mgr.reset_all();
        assert!(mgr.can_execute("ep1"));
        assert!(mgr.can_execute("ep2"));
    }
}
```

- [ ] **Step 2: Add circuit_breaker to lib.rs**

```rust
pub mod circuit_breaker;
pub mod types;
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path crates/gateway/Cargo.toml -- circuit_breaker`
Expected: all 11 tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/gateway/src/circuit_breaker.rs crates/gateway/src/lib.rs
git commit -m "feat: add circuit breaker — closed/open/half-open state machine"
```

---

### Task 7: Adapter trait + noop adapter

**Files:**
- Create: `crates/gateway/src/adapters/mod.rs`
- Create: `crates/gateway/src/adapters/noop.rs`
- Modify: `crates/gateway/src/lib.rs`

- [ ] **Step 1: Write adapter trait and registry**

In `crates/gateway/src/adapters/mod.rs`:

```rust
pub mod noop;

use std::collections::HashMap;
use std::pin::Pin;
use std::sync::{Arc, RwLock};

use async_trait::async_trait;
use futures::Stream;

use crate::types::capability::Capability;
use crate::types::config::RouterConfig;
use crate::types::error::GatewayError;
use crate::types::request::{InferenceRequest, InferenceResponse, StreamChunk};

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
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StreamChunk, GatewayError>> + Send>>,
        GatewayError,
    >;
}

#[derive(Clone)]
pub struct AdapterRegistry {
    adapters: Arc<RwLock<HashMap<String, Arc<dyn InferenceAdapter>>>>,
}

impl AdapterRegistry {
    pub fn new() -> Self {
        Self {
            adapters: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register(&self, adapter: Arc<dyn InferenceAdapter>) {
        let id = adapter.id().to_string();
        self.adapters.write().unwrap().insert(id, adapter);
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn InferenceAdapter>> {
        self.adapters.read().unwrap().get(id).cloned()
    }

    pub fn list(&self) -> Vec<String> {
        self.adapters.read().unwrap().keys().cloned().collect()
    }

    pub fn unregister(&self, id: &str) -> bool {
        self.adapters.write().unwrap().remove(id).is_some()
    }
}

impl Default for AdapterRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use noop::NoopAdapter;

    #[test]
    fn registry_register_and_get() {
        let registry = AdapterRegistry::new();
        registry.register(Arc::new(NoopAdapter));
        assert!(registry.get("noop").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn registry_list() {
        let registry = AdapterRegistry::new();
        registry.register(Arc::new(NoopAdapter));
        let ids = registry.list();
        assert_eq!(ids, vec!["noop"]);
    }

    #[test]
    fn registry_unregister() {
        let registry = AdapterRegistry::new();
        registry.register(Arc::new(NoopAdapter));
        assert!(registry.unregister("noop"));
        assert!(!registry.unregister("noop"));
        assert!(registry.get("noop").is_none());
    }
}
```

- [ ] **Step 2: Write noop adapter**

In `crates/gateway/src/adapters/noop.rs`:

```rust
use std::pin::Pin;

use async_trait::async_trait;
use futures::Stream;

use super::InferenceAdapter;
use crate::types::capability::Capability;
use crate::types::config::RouterConfig;
use crate::types::cost::TokenUsage;
use crate::types::error::GatewayError;
use crate::types::request::{InferenceRequest, InferenceResponse, StreamChunk};
use crate::types::trace::{Attempt, AttemptStatus};

pub struct NoopAdapter;

#[async_trait]
impl InferenceAdapter for NoopAdapter {
    fn id(&self) -> &str {
        "noop"
    }

    fn supports(&self, _capability: &Capability) -> bool {
        true
    }

    async fn execute(
        &self,
        _config: &RouterConfig,
        request: &InferenceRequest,
    ) -> Result<InferenceResponse, GatewayError> {
        Ok(InferenceResponse {
            success: false,
            content: Some(format!(
                "No inference provider available for capability {:?}. \
                 Install Ollama or configure an API key.",
                request.capability
            )),
            embeddings: None,
            model: Some("noop".into()),
            usage: Some(TokenUsage::default()),
            estimated_cost: None,
            actual_cost: None,
            attempts: vec![Attempt {
                sequence: 0,
                adapter: "noop".into(),
                model: "noop".into(),
                api_model_id: "noop".into(),
                status: AttemptStatus::Failed,
                duration_ms: 0,
                tokens: None,
                cost: Some(0.0),
                error: Some("no provider available".into()),
                fallback_triggered: false,
            }],
        })
    }

    async fn stream(
        &self,
        _config: &RouterConfig,
        _request: &InferenceRequest,
    ) -> Result<
        Pin<Box<dyn Stream<Item = Result<StreamChunk, GatewayError>> + Send>>,
        GatewayError,
    > {
        let chunk = StreamChunk {
            content: "No inference provider available.".into(),
            finish_reason: Some("noop".into()),
            usage: Some(TokenUsage::default()),
        };
        Ok(Box::pin(futures::stream::once(async move { Ok(chunk) })))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::capability::Capability;
    use crate::types::request::Payload;
    use std::collections::HashMap;

    fn test_config() -> RouterConfig {
        RouterConfig {
            url: "http://noop".into(),
            api_key_env: None,
            enabled: true,
            timeout_ms: None,
            headers: HashMap::new(),
        }
    }

    fn test_request() -> InferenceRequest {
        InferenceRequest {
            capability: Capability::Chat,
            model: None,
            router: None,
            chain: None,
            payload: Payload::Chat {
                messages: vec![],
                system: None,
                max_tokens: None,
                temperature: None,
            },
            budget: None,
        }
    }

    #[test]
    fn noop_supports_all_capabilities() {
        let adapter = NoopAdapter;
        assert!(adapter.supports(&Capability::Chat));
        assert!(adapter.supports(&Capability::Embed));
        assert!(adapter.supports(&Capability::Classify));
        assert!(adapter.supports(&Capability::VoiceStt));
    }

    #[tokio::test]
    async fn noop_execute_returns_unsuccessful() {
        let adapter = NoopAdapter;
        let response = adapter.execute(&test_config(), &test_request()).await.unwrap();
        assert!(!response.success);
        assert!(response.content.unwrap().contains("No inference provider"));
    }

    #[tokio::test]
    async fn noop_execute_never_errors() {
        let adapter = NoopAdapter;
        // Should always return Ok, never Err
        let result = adapter.execute(&test_config(), &test_request()).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn noop_stream_returns_single_chunk() {
        use futures::StreamExt;
        let adapter = NoopAdapter;
        let mut stream = adapter.stream(&test_config(), &test_request()).await.unwrap();
        let chunk = stream.next().await.unwrap().unwrap();
        assert!(chunk.content.contains("No inference provider"));
        assert!(stream.next().await.is_none());
    }
}
```

- [ ] **Step 3: Add adapters to lib.rs**

```rust
pub mod adapters;
pub mod circuit_breaker;
pub mod types;
```

- [ ] **Step 4: Run tests**

Run: `cargo test --manifest-path crates/gateway/Cargo.toml -- adapters`
Expected: all 7 tests PASS (3 registry + 4 noop)

- [ ] **Step 5: Commit**

```bash
git add crates/gateway/src/adapters/
git commit -m "feat: add adapter trait, registry, and noop adapter"
```

---

### Task 8: Model selection service

**Files:**
- Create: `crates/gateway/src/selection.rs`
- Modify: `crates/gateway/src/lib.rs`

- [ ] **Step 1: Write selection service with full test suite**

In `crates/gateway/src/selection.rs`:

```rust
use crate::circuit_breaker::CircuitBreakerManager;
use crate::types::capability::Capability;
use crate::types::config::{
    ChainEntry, FallbackChainConfig, GatewayConfig, ModelConfig, ModelPricing, RouterConfig,
};
use crate::types::cost::CostEstimate;

#[derive(Debug, Clone)]
pub struct SelectionCriteria {
    pub capability: Capability,
    pub model: Option<String>,
    pub router: Option<String>,
    pub chain: Option<String>,
    pub budget: Option<f64>,
    pub input_tokens: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SelectedModel {
    pub model: String,
    pub router: String,
    pub router_config: RouterConfig,
    pub model_config: ModelConfig,
    pub api_model_id: String,
    pub priority: u8,
    pub cost_estimate: Option<CostEstimate>,
}

#[derive(Debug, Clone)]
pub struct SkippedCandidate {
    pub model: String,
    pub router: String,
    pub reason: String,
}

#[derive(Debug, Clone)]
pub struct SelectionResult {
    pub selected: Option<SelectedModel>,
    pub all_candidates: Vec<SelectedModel>,
    pub skipped: Vec<SkippedCandidate>,
    pub chain: Option<FallbackChainConfig>,
}

pub struct ModelSelectionService<'a> {
    config: &'a GatewayConfig,
    circuit_breaker: &'a CircuitBreakerManager,
}

impl<'a> ModelSelectionService<'a> {
    pub fn new(config: &'a GatewayConfig, circuit_breaker: &'a CircuitBreakerManager) -> Self {
        Self {
            config,
            circuit_breaker,
        }
    }

    pub fn select(&self, criteria: &SelectionCriteria) -> SelectionResult {
        let result = self.select_all(criteria);
        SelectionResult {
            selected: result.all_candidates.first().cloned(),
            ..result
        }
    }

    pub fn select_all(&self, criteria: &SelectionCriteria) -> SelectionResult {
        // Tier 1: Direct — router + model specified
        if let (Some(router_id), Some(model_id)) = (&criteria.router, &criteria.model) {
            return self.select_direct(router_id, model_id, criteria);
        }

        // Tier 2: Named chain
        if let Some(chain_id) = &criteria.chain {
            if let Some(chain) = self.config.chains.get(chain_id) {
                return self.select_from_chain(chain, criteria);
            }
            return SelectionResult {
                selected: None,
                all_candidates: vec![],
                skipped: vec![SkippedCandidate {
                    model: "".into(),
                    router: "".into(),
                    reason: format!("chain '{}' not found", chain_id),
                }],
                chain: None,
            };
        }

        // Tier 3: Capability — find chain matching capability
        for chain in self.config.chains.values() {
            if chain.capability == criteria.capability {
                return self.select_from_chain(chain, criteria);
            }
        }

        SelectionResult {
            selected: None,
            all_candidates: vec![],
            skipped: vec![],
            chain: None,
        }
    }

    fn select_direct(
        &self,
        router_id: &str,
        model_id: &str,
        criteria: &SelectionCriteria,
    ) -> SelectionResult {
        let mut skipped = vec![];

        let router_config = match self.config.routers.get(router_id) {
            Some(r) => r,
            None => {
                skipped.push(SkippedCandidate {
                    model: model_id.into(),
                    router: router_id.into(),
                    reason: "router not found".into(),
                });
                return SelectionResult {
                    selected: None,
                    all_candidates: vec![],
                    skipped,
                    chain: None,
                };
            }
        };

        let model_config = match self.config.models.get(model_id) {
            Some(m) => m,
            None => {
                skipped.push(SkippedCandidate {
                    model: model_id.into(),
                    router: router_id.into(),
                    reason: "model not found".into(),
                });
                return SelectionResult {
                    selected: None,
                    all_candidates: vec![],
                    skipped,
                    chain: None,
                };
            }
        };

        if let Some(reason) = self.validate_candidate(
            router_id,
            router_config,
            model_id,
            model_config,
            criteria,
        ) {
            skipped.push(SkippedCandidate {
                model: model_id.into(),
                router: router_id.into(),
                reason,
            });
            return SelectionResult {
                selected: None,
                all_candidates: vec![],
                skipped,
                chain: None,
            };
        }

        let selected = SelectedModel {
            model: model_id.into(),
            router: router_id.into(),
            router_config: router_config.clone(),
            model_config: model_config.clone(),
            api_model_id: model_config
                .api_model_id
                .clone()
                .unwrap_or_else(|| model_id.to_string()),
            priority: 0,
            cost_estimate: self.estimate_cost(model_config, criteria),
        };

        SelectionResult {
            selected: Some(selected.clone()),
            all_candidates: vec![selected],
            skipped,
            chain: None,
        }
    }

    fn select_from_chain(
        &self,
        chain: &FallbackChainConfig,
        criteria: &SelectionCriteria,
    ) -> SelectionResult {
        let mut all_candidates = vec![];
        let mut skipped = vec![];

        let mut entries = chain.models.clone();
        entries.sort_by_key(|e| e.priority);

        for entry in &entries {
            let router_id = entry
                .router
                .clone()
                .or_else(|| {
                    self.config
                        .models
                        .get(&entry.model)
                        .map(|m| m.provider.clone())
                })
                .unwrap_or_default();

            let router_config = match self.config.routers.get(&router_id) {
                Some(r) => r,
                None => {
                    skipped.push(SkippedCandidate {
                        model: entry.model.clone(),
                        router: router_id,
                        reason: "router not found".into(),
                    });
                    continue;
                }
            };

            let model_config = match self.config.models.get(&entry.model) {
                Some(m) => m,
                None => {
                    skipped.push(SkippedCandidate {
                        model: entry.model.clone(),
                        router: router_id,
                        reason: "model not found".into(),
                    });
                    continue;
                }
            };

            if let Some(reason) = self.validate_candidate(
                &router_id,
                router_config,
                &entry.model,
                model_config,
                criteria,
            ) {
                skipped.push(SkippedCandidate {
                    model: entry.model.clone(),
                    router: router_id,
                    reason,
                });
                continue;
            }

            let api_model_id = entry
                .api_model_id
                .clone()
                .or_else(|| model_config.api_model_id.clone())
                .unwrap_or_else(|| entry.model.clone());

            all_candidates.push(SelectedModel {
                model: entry.model.clone(),
                router: router_id.clone(),
                router_config: router_config.clone(),
                model_config: model_config.clone(),
                api_model_id,
                priority: entry.priority,
                cost_estimate: self.estimate_cost(model_config, criteria),
            });
        }

        SelectionResult {
            selected: all_candidates.first().cloned(),
            all_candidates,
            skipped,
            chain: Some(chain.clone()),
        }
    }

    fn validate_candidate(
        &self,
        router_id: &str,
        router_config: &RouterConfig,
        model_id: &str,
        model_config: &ModelConfig,
        criteria: &SelectionCriteria,
    ) -> Option<String> {
        if !router_config.enabled {
            return Some("router disabled".into());
        }

        if !model_config.capabilities.contains(&criteria.capability) {
            return Some(format!(
                "model does not support {:?}",
                criteria.capability
            ));
        }

        let endpoint = format!("{}:{}", router_id, model_id);
        if !self.circuit_breaker.can_execute(&endpoint) {
            return Some("circuit breaker open".into());
        }

        if let (Some(budget), Some(estimate)) =
            (criteria.budget, self.estimate_cost(model_config, criteria))
        {
            if estimate.estimated > budget {
                return Some(format!(
                    "over budget: estimated ${:.4}, budget ${:.4}",
                    estimate.estimated, budget
                ));
            }
        }

        None
    }

    fn estimate_cost(
        &self,
        model_config: &ModelConfig,
        criteria: &SelectionCriteria,
    ) -> Option<CostEstimate> {
        let pricing = model_config.pricing.as_ref()?;
        let input_tokens = criteria.input_tokens.unwrap_or(0);
        let max_output = model_config.max_output_tokens;

        let input_cost = input_tokens as f64 * pricing.input_per_1k / 1000.0;
        let max_output_cost = max_output as f64 * pricing.output_per_1k / 1000.0;

        Some(CostEstimate {
            estimated: input_cost + max_output_cost,
            minimum: input_cost,
            maximum: input_cost + max_output_cost,
            currency: "USD".into(),
            model: model_config.id.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::circuit_breaker::CircuitBreakerConfig;
    use std::collections::HashMap;
    use std::time::Duration;

    fn test_config() -> GatewayConfig {
        let mut routers = HashMap::new();
        routers.insert(
            "ollama".into(),
            RouterConfig {
                url: "http://localhost:11434".into(),
                api_key_env: None,
                enabled: true,
                timeout_ms: None,
                headers: HashMap::new(),
            },
        );
        routers.insert(
            "anthropic".into(),
            RouterConfig {
                url: "https://api.anthropic.com".into(),
                api_key_env: Some("ANTHROPIC_API_KEY".into()),
                enabled: true,
                timeout_ms: None,
                headers: HashMap::new(),
            },
        );

        let mut models = HashMap::new();
        models.insert(
            "gemma3:27b".into(),
            ModelConfig {
                id: "gemma3:27b".into(),
                api_model_id: None,
                provider: "ollama".into(),
                capabilities: vec![Capability::Chat, Capability::Classify, Capability::Summarize],
                context_window: 8192,
                max_output_tokens: 2048,
                pricing: None,
            },
        );
        models.insert(
            "all-minilm".into(),
            ModelConfig {
                id: "all-minilm".into(),
                api_model_id: None,
                provider: "ollama".into(),
                capabilities: vec![Capability::Embed],
                context_window: 512,
                max_output_tokens: 0,
                pricing: None,
            },
        );
        models.insert(
            "claude-haiku".into(),
            ModelConfig {
                id: "claude-haiku".into(),
                api_model_id: Some("claude-haiku-4-5-20251001".into()),
                provider: "anthropic".into(),
                capabilities: vec![Capability::Chat],
                context_window: 200_000,
                max_output_tokens: 4096,
                pricing: Some(ModelPricing {
                    input_per_1k: 0.0008,
                    output_per_1k: 0.004,
                    per_request: None,
                }),
            },
        );

        let mut chains = HashMap::new();
        chains.insert(
            "embed_chain".into(),
            FallbackChainConfig {
                id: "embed_chain".into(),
                capability: Capability::Embed,
                models: vec![ChainEntry {
                    model: "all-minilm".into(),
                    router: Some("ollama".into()),
                    api_model_id: None,
                    priority: 1,
                }],
                fallback_triggers: vec![],
            },
        );
        chains.insert(
            "chat_chain".into(),
            FallbackChainConfig {
                id: "chat_chain".into(),
                capability: Capability::Chat,
                models: vec![
                    ChainEntry {
                        model: "gemma3:27b".into(),
                        router: Some("ollama".into()),
                        api_model_id: None,
                        priority: 1,
                    },
                    ChainEntry {
                        model: "claude-haiku".into(),
                        router: Some("anthropic".into()),
                        api_model_id: None,
                        priority: 2,
                    },
                ],
                fallback_triggers: vec![
                    crate::types::config::FallbackTrigger::Timeout,
                    crate::types::config::FallbackTrigger::ProviderError,
                ],
            },
        );

        GatewayConfig {
            routers,
            models,
            chains,
        }
    }

    fn test_cb() -> CircuitBreakerManager {
        CircuitBreakerManager::new(CircuitBreakerConfig {
            threshold: 5,
            timeout: Duration::from_secs(300),
            half_open_max_requests: 3,
        })
    }

    #[test]
    fn tier1_direct_selection() {
        let config = test_config();
        let cb = test_cb();
        let svc = ModelSelectionService::new(&config, &cb);

        let result = svc.select(&SelectionCriteria {
            capability: Capability::Chat,
            model: Some("gemma3:27b".into()),
            router: Some("ollama".into()),
            chain: None,
            budget: None,
            input_tokens: None,
        });

        assert!(result.selected.is_some());
        let selected = result.selected.unwrap();
        assert_eq!(selected.model, "gemma3:27b");
        assert_eq!(selected.router, "ollama");
    }

    #[test]
    fn tier1_direct_unknown_router() {
        let config = test_config();
        let cb = test_cb();
        let svc = ModelSelectionService::new(&config, &cb);

        let result = svc.select(&SelectionCriteria {
            capability: Capability::Chat,
            model: Some("gemma3:27b".into()),
            router: Some("nonexistent".into()),
            chain: None,
            budget: None,
            input_tokens: None,
        });

        assert!(result.selected.is_none());
        assert_eq!(result.skipped.len(), 1);
        assert!(result.skipped[0].reason.contains("router not found"));
    }

    #[test]
    fn tier2_chain_selection() {
        let config = test_config();
        let cb = test_cb();
        let svc = ModelSelectionService::new(&config, &cb);

        let result = svc.select_all(&SelectionCriteria {
            capability: Capability::Chat,
            model: None,
            router: None,
            chain: Some("chat_chain".into()),
            budget: None,
            input_tokens: None,
        });

        assert_eq!(result.all_candidates.len(), 2);
        assert_eq!(result.all_candidates[0].model, "gemma3:27b");
        assert_eq!(result.all_candidates[1].model, "claude-haiku");
    }

    #[test]
    fn tier3_capability_selection() {
        let config = test_config();
        let cb = test_cb();
        let svc = ModelSelectionService::new(&config, &cb);

        let result = svc.select(&SelectionCriteria {
            capability: Capability::Embed,
            model: None,
            router: None,
            chain: None,
            budget: None,
            input_tokens: None,
        });

        assert!(result.selected.is_some());
        assert_eq!(result.selected.unwrap().model, "all-minilm");
    }

    #[test]
    fn skips_disabled_router() {
        let mut config = test_config();
        config.routers.get_mut("ollama").unwrap().enabled = false;
        let cb = test_cb();
        let svc = ModelSelectionService::new(&config, &cb);

        let result = svc.select(&SelectionCriteria {
            capability: Capability::Chat,
            model: None,
            router: None,
            chain: Some("chat_chain".into()),
            budget: None,
            input_tokens: None,
        });

        // ollama disabled → gemma3 skipped, claude-haiku selected
        assert!(result.selected.is_some());
        assert_eq!(result.selected.unwrap().model, "claude-haiku");
        assert_eq!(result.skipped.len(), 1);
        assert!(result.skipped[0].reason.contains("disabled"));
    }

    #[test]
    fn skips_wrong_capability() {
        let config = test_config();
        let cb = test_cb();
        let svc = ModelSelectionService::new(&config, &cb);

        // Try to use gemma3 for Embed (it only supports Chat)
        let result = svc.select(&SelectionCriteria {
            capability: Capability::Embed,
            model: Some("gemma3:27b".into()),
            router: Some("ollama".into()),
            chain: None,
            budget: None,
            input_tokens: None,
        });

        assert!(result.selected.is_none());
        assert!(result.skipped[0].reason.contains("does not support"));
    }

    #[test]
    fn skips_circuit_breaker_open() {
        let config = test_config();
        let cb = CircuitBreakerManager::new(CircuitBreakerConfig {
            threshold: 1,
            timeout: Duration::from_secs(300),
            half_open_max_requests: 3,
        });
        cb.record_failure("ollama:gemma3:27b");

        let svc = ModelSelectionService::new(&config, &cb);

        let result = svc.select_all(&SelectionCriteria {
            capability: Capability::Chat,
            model: None,
            router: None,
            chain: Some("chat_chain".into()),
            budget: None,
            input_tokens: None,
        });

        // gemma3 circuit open → skipped, claude-haiku selected
        assert_eq!(result.all_candidates.len(), 1);
        assert_eq!(result.all_candidates[0].model, "claude-haiku");
        assert!(result.skipped[0].reason.contains("circuit breaker"));
    }

    #[test]
    fn skips_over_budget() {
        let config = test_config();
        let cb = test_cb();
        let svc = ModelSelectionService::new(&config, &cb);

        let result = svc.select_all(&SelectionCriteria {
            capability: Capability::Chat,
            model: None,
            router: None,
            chain: Some("chat_chain".into()),
            budget: Some(0.001), // very low budget
            input_tokens: Some(1000),
        });

        // gemma3 has no pricing → passes (free), claude-haiku over budget → skipped
        assert_eq!(result.all_candidates.len(), 1);
        assert_eq!(result.all_candidates[0].model, "gemma3:27b");
    }

    #[test]
    fn api_model_id_override() {
        let config = test_config();
        let cb = test_cb();
        let svc = ModelSelectionService::new(&config, &cb);

        let result = svc.select(&SelectionCriteria {
            capability: Capability::Chat,
            model: Some("claude-haiku".into()),
            router: Some("anthropic".into()),
            chain: None,
            budget: None,
            input_tokens: None,
        });

        let selected = result.selected.unwrap();
        assert_eq!(selected.api_model_id, "claude-haiku-4-5-20251001");
    }

    #[test]
    fn no_chain_for_capability() {
        let config = test_config();
        let cb = test_cb();
        let svc = ModelSelectionService::new(&config, &cb);

        let result = svc.select(&SelectionCriteria {
            capability: Capability::VoiceStt,
            model: None,
            router: None,
            chain: None,
            budget: None,
            input_tokens: None,
        });

        assert!(result.selected.is_none());
        assert!(result.all_candidates.is_empty());
    }
}
```

- [ ] **Step 2: Add selection to lib.rs**

```rust
pub mod adapters;
pub mod circuit_breaker;
pub mod selection;
pub mod types;
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path crates/gateway/Cargo.toml -- selection`
Expected: all 10 tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/gateway/src/selection.rs crates/gateway/src/lib.rs
git commit -m "feat: add model selection — 3-tier resolution with validation pipeline"
```

---

### Task 9: Budget filtering

**Files:**
- Create: `crates/gateway/src/budget.rs`
- Modify: `crates/gateway/src/lib.rs`

- [ ] **Step 1: Write budget module with tests**

In `crates/gateway/src/budget.rs`:

```rust
use crate::types::config::ModelPricing;
use crate::types::cost::CostEstimate;

pub fn estimate_cost(
    pricing: &ModelPricing,
    model_id: &str,
    input_tokens: u32,
    max_output_tokens: u32,
) -> CostEstimate {
    let input_cost = input_tokens as f64 * pricing.input_per_1k / 1000.0;
    let max_output_cost = max_output_tokens as f64 * pricing.output_per_1k / 1000.0;

    CostEstimate {
        estimated: input_cost + max_output_cost,
        minimum: input_cost,
        maximum: input_cost + max_output_cost,
        currency: "USD".into(),
        model: model_id.into(),
    }
}

#[derive(Debug)]
pub struct AffordableModel {
    pub model: String,
    pub cost_estimate: CostEstimate,
    pub within_budget: bool,
}

#[derive(Debug)]
pub struct BudgetFilterResult {
    pub affordable: Vec<AffordableModel>,
    pub over_budget: Vec<AffordableModel>,
}

pub fn filter_by_budget(
    models: &[(String, CostEstimate)],
    budget: f64,
) -> BudgetFilterResult {
    let mut affordable = vec![];
    let mut over_budget = vec![];

    for (model_id, estimate) in models {
        let entry = AffordableModel {
            model: model_id.clone(),
            cost_estimate: estimate.clone(),
            within_budget: estimate.estimated <= budget,
        };
        if entry.within_budget {
            affordable.push(entry);
        } else {
            over_budget.push(entry);
        }
    }

    BudgetFilterResult {
        affordable,
        over_budget,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn haiku_pricing() -> ModelPricing {
        ModelPricing {
            input_per_1k: 0.0008,
            output_per_1k: 0.004,
            per_request: None,
        }
    }

    fn sonnet_pricing() -> ModelPricing {
        ModelPricing {
            input_per_1k: 0.003,
            output_per_1k: 0.015,
            per_request: None,
        }
    }

    #[test]
    fn estimate_cost_haiku() {
        let est = estimate_cost(&haiku_pricing(), "claude-haiku", 1000, 500);
        // input: 1000 * 0.0008/1000 = 0.0008
        // output: 500 * 0.004/1000 = 0.002
        assert!((est.estimated - 0.0028).abs() < 1e-10);
        assert!((est.minimum - 0.0008).abs() < 1e-10);
    }

    #[test]
    fn estimate_cost_zero_tokens() {
        let est = estimate_cost(&haiku_pricing(), "model", 0, 0);
        assert_eq!(est.estimated, 0.0);
    }

    #[test]
    fn filter_separates_affordable_and_over() {
        let models = vec![
            (
                "cheap".into(),
                estimate_cost(&haiku_pricing(), "cheap", 100, 100),
            ),
            (
                "expensive".into(),
                estimate_cost(&sonnet_pricing(), "expensive", 10000, 4096),
            ),
        ];

        let result = filter_by_budget(&models, 0.01);
        assert_eq!(result.affordable.len(), 1);
        assert_eq!(result.affordable[0].model, "cheap");
        assert_eq!(result.over_budget.len(), 1);
        assert_eq!(result.over_budget[0].model, "expensive");
    }

    #[test]
    fn filter_all_affordable() {
        let models = vec![(
            "cheap".into(),
            estimate_cost(&haiku_pricing(), "cheap", 100, 100),
        )];

        let result = filter_by_budget(&models, 100.0);
        assert_eq!(result.affordable.len(), 1);
        assert!(result.over_budget.is_empty());
    }

    #[test]
    fn filter_empty_input() {
        let result = filter_by_budget(&[], 100.0);
        assert!(result.affordable.is_empty());
        assert!(result.over_budget.is_empty());
    }
}
```

- [ ] **Step 2: Add budget to lib.rs**

```rust
pub mod adapters;
pub mod budget;
pub mod circuit_breaker;
pub mod selection;
pub mod types;
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path crates/gateway/Cargo.toml -- budget`
Expected: all 5 tests PASS

- [ ] **Step 4: Commit**

```bash
git add crates/gateway/src/budget.rs crates/gateway/src/lib.rs
git commit -m "feat: add budget — cost estimation and affordability filtering"
```

---

### Task 10: Gateway engine

**Files:**
- Create: `crates/gateway/src/engine.rs`
- Modify: `crates/gateway/src/lib.rs`

- [ ] **Step 1: Write engine with test suite**

In `crates/gateway/src/engine.rs`:

```rust
use std::sync::{Arc, RwLock};
use std::time::Instant;

use crate::adapters::AdapterRegistry;
use crate::circuit_breaker::CircuitBreakerManager;
use crate::selection::{ModelSelectionService, SelectionCriteria, SelectedModel};
use crate::types::capability::Capability;
use crate::types::config::GatewayConfig;
use crate::types::cost::TokenUsage;
use crate::types::error::GatewayError;
use crate::types::request::{InferenceRequest, InferenceResponse};
use crate::types::trace::{Attempt, AttemptStatus};

pub struct Gateway {
    config: Arc<RwLock<GatewayConfig>>,
    adapters: AdapterRegistry,
    circuit_breaker: CircuitBreakerManager,
}

impl Gateway {
    pub fn new(
        config: GatewayConfig,
        adapters: AdapterRegistry,
        circuit_breaker: CircuitBreakerManager,
    ) -> Self {
        Self {
            config: Arc::new(RwLock::new(config)),
            adapters,
            circuit_breaker,
        }
    }

    pub async fn execute(&self, request: &InferenceRequest) -> Result<InferenceResponse, GatewayError> {
        let config = self.config.read().unwrap().clone();
        let criteria = self.build_criteria(request);
        let service = ModelSelectionService::new(&config, &self.circuit_breaker);
        let result = service.select_all(&criteria);

        if result.all_candidates.is_empty() {
            return Err(GatewayError::NoCandidates {
                capability: request.capability.clone(),
            });
        }

        let chain = result.chain.as_ref();
        let triggers = chain
            .map(|c| c.fallback_triggers.as_slice())
            .unwrap_or(&[]);

        let mut attempts = vec![];

        for (i, candidate) in result.all_candidates.iter().enumerate() {
            let adapter = match self.adapters.get(&candidate.router) {
                Some(a) => a,
                None => {
                    attempts.push(Attempt {
                        sequence: i as u8,
                        adapter: candidate.router.clone(),
                        model: candidate.model.clone(),
                        api_model_id: candidate.api_model_id.clone(),
                        status: AttemptStatus::Failed,
                        duration_ms: 0,
                        tokens: None,
                        cost: None,
                        error: Some("adapter not found".into()),
                        fallback_triggered: true,
                    });
                    continue;
                }
            };

            let start = Instant::now();
            let endpoint = format!("{}:{}", candidate.router, candidate.model);

            match adapter.execute(&candidate.router_config, request).await {
                Ok(mut response) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    self.circuit_breaker.record_success(&endpoint);

                    attempts.push(Attempt {
                        sequence: i as u8,
                        adapter: candidate.router.clone(),
                        model: candidate.model.clone(),
                        api_model_id: candidate.api_model_id.clone(),
                        status: AttemptStatus::Success,
                        duration_ms,
                        tokens: response.usage.clone(),
                        cost: Some(0.0),
                        error: None,
                        fallback_triggered: false,
                    });

                    response.attempts = attempts;
                    response.model = Some(candidate.model.clone());
                    return Ok(response);
                }
                Err(err) => {
                    let duration_ms = start.elapsed().as_millis() as u64;
                    self.circuit_breaker.record_failure(&endpoint);

                    let should_fallback = err.should_trigger_fallback(triggers);

                    attempts.push(Attempt {
                        sequence: i as u8,
                        adapter: candidate.router.clone(),
                        model: candidate.model.clone(),
                        api_model_id: candidate.api_model_id.clone(),
                        status: AttemptStatus::Failed,
                        duration_ms,
                        tokens: None,
                        cost: None,
                        error: Some(err.to_string()),
                        fallback_triggered: should_fallback,
                    });

                    if !should_fallback {
                        break;
                    }
                }
            }
        }

        Err(GatewayError::AllAttemptsFailed {
            attempts: attempts.len(),
        })
    }

    pub fn update_config(&self, config: GatewayConfig) {
        *self.config.write().unwrap() = config;
    }

    fn build_criteria(&self, request: &InferenceRequest) -> SelectionCriteria {
        let input_tokens = match &request.payload {
            crate::types::request::Payload::Chat { messages, system, .. } => {
                let text_len: usize = messages.iter().map(|m| m.content.len()).sum::<usize>()
                    + system.as_ref().map_or(0, |s| s.len());
                Some((text_len / 4) as u32) // rough estimate: 4 chars per token
            }
            crate::types::request::Payload::Embed { texts } => {
                let text_len: usize = texts.iter().map(|t| t.len()).sum();
                Some((text_len / 4) as u32)
            }
        };

        SelectionCriteria {
            capability: request.capability.clone(),
            model: request.model.clone(),
            router: request.router.clone(),
            chain: request.chain.clone(),
            budget: request.budget,
            input_tokens,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::noop::NoopAdapter;
    use crate::circuit_breaker::CircuitBreakerConfig;
    use crate::types::config::*;
    use crate::types::request::*;
    use std::collections::HashMap;
    use std::time::Duration;

    fn test_config_with_noop() -> GatewayConfig {
        let mut routers = HashMap::new();
        routers.insert(
            "noop".into(),
            RouterConfig {
                url: "http://noop".into(),
                api_key_env: None,
                enabled: true,
                timeout_ms: None,
                headers: HashMap::new(),
            },
        );

        let mut models = HashMap::new();
        models.insert(
            "noop".into(),
            ModelConfig {
                id: "noop".into(),
                api_model_id: None,
                provider: "noop".into(),
                capabilities: vec![
                    Capability::Chat,
                    Capability::Embed,
                ],
                context_window: 4096,
                max_output_tokens: 1024,
                pricing: None,
            },
        );

        let mut chains = HashMap::new();
        chains.insert(
            "chat_chain".into(),
            FallbackChainConfig {
                id: "chat_chain".into(),
                capability: Capability::Chat,
                models: vec![ChainEntry {
                    model: "noop".into(),
                    router: Some("noop".into()),
                    api_model_id: None,
                    priority: 1,
                }],
                fallback_triggers: vec![],
            },
        );

        GatewayConfig {
            routers,
            models,
            chains,
        }
    }

    fn test_gateway() -> Gateway {
        let config = test_config_with_noop();
        let adapters = AdapterRegistry::new();
        adapters.register(Arc::new(NoopAdapter));
        let cb = CircuitBreakerManager::new(CircuitBreakerConfig {
            threshold: 5,
            timeout: Duration::from_secs(300),
            half_open_max_requests: 3,
        });
        Gateway::new(config, adapters, cb)
    }

    fn chat_request() -> InferenceRequest {
        InferenceRequest {
            capability: Capability::Chat,
            model: None,
            router: None,
            chain: None,
            payload: Payload::Chat {
                messages: vec![Message {
                    role: MessageRole::User,
                    content: "hello".into(),
                    tool_call_id: None,
                }],
                system: None,
                max_tokens: None,
                temperature: None,
            },
            budget: None,
        }
    }

    #[tokio::test]
    async fn execute_with_noop_adapter() {
        let gw = test_gateway();
        let response = gw.execute(&chat_request()).await.unwrap();
        // noop returns success=false but doesn't error
        assert!(!response.success);
        assert!(response.content.unwrap().contains("No inference provider"));
    }

    #[tokio::test]
    async fn execute_no_candidates_errors() {
        let gw = test_gateway();
        let result = gw
            .execute(&InferenceRequest {
                capability: Capability::VoiceStt, // no chain for this
                model: None,
                router: None,
                chain: None,
                payload: Payload::Chat {
                    messages: vec![],
                    system: None,
                    max_tokens: None,
                    temperature: None,
                },
                budget: None,
            })
            .await;

        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            GatewayError::NoCandidates { .. }
        ));
    }

    #[tokio::test]
    async fn execute_with_direct_model() {
        let gw = test_gateway();
        let req = InferenceRequest {
            capability: Capability::Chat,
            model: Some("noop".into()),
            router: Some("noop".into()),
            chain: None,
            payload: Payload::Chat {
                messages: vec![Message {
                    role: MessageRole::User,
                    content: "test".into(),
                    tool_call_id: None,
                }],
                system: None,
                max_tokens: None,
                temperature: None,
            },
            budget: None,
        };

        let response = gw.execute(&req).await.unwrap();
        assert_eq!(response.model.as_deref(), Some("noop"));
    }

    #[tokio::test]
    async fn execute_records_attempts() {
        let gw = test_gateway();
        let response = gw.execute(&chat_request()).await.unwrap();
        assert!(!response.attempts.is_empty());
        assert_eq!(response.attempts[0].adapter, "noop");
    }

    #[tokio::test]
    async fn update_config_takes_effect() {
        let gw = test_gateway();
        // Verify current config works
        assert!(gw.execute(&chat_request()).await.is_ok());

        // Update to empty config
        gw.update_config(GatewayConfig::default());

        // Now should fail (no chains)
        let result = gw.execute(&chat_request()).await;
        assert!(result.is_err());
    }
}
```

- [ ] **Step 2: Add engine to lib.rs**

```rust
pub mod adapters;
pub mod budget;
pub mod circuit_breaker;
pub mod engine;
pub mod selection;
pub mod types;
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path crates/gateway/Cargo.toml -- engine`
Expected: all 5 tests PASS

- [ ] **Step 4: Run full test suite**

Run: `cargo test --manifest-path crates/gateway/Cargo.toml`
Expected: all tests PASS (types + circuit_breaker + adapters + selection + budget + engine)

- [ ] **Step 5: Commit**

```bash
git add crates/gateway/src/engine.rs crates/gateway/src/lib.rs
git commit -m "feat: add gateway engine — execution with fallback chain walking"
```

---

### Task 11: Config builder

**Files:**
- Create: `crates/gateway/src/config.rs`
- Modify: `crates/gateway/src/lib.rs`

- [ ] **Step 1: Write builder with validation and tests**

In `crates/gateway/src/config.rs`:

```rust
use std::collections::HashMap;
use crate::types::config::*;
use crate::types::capability::Capability;

pub struct GatewayBuilder {
    routers: HashMap<String, RouterConfig>,
    models: HashMap<String, ModelConfig>,
    chains: HashMap<String, FallbackChainConfig>,
}

impl GatewayBuilder {
    pub fn new() -> Self {
        Self {
            routers: HashMap::new(),
            models: HashMap::new(),
            chains: HashMap::new(),
        }
    }

    pub fn add_router(mut self, id: &str, config: RouterConfig) -> Self {
        self.routers.insert(id.into(), config);
        self
    }

    pub fn add_model(mut self, config: ModelConfig) -> Self {
        self.models.insert(config.id.clone(), config);
        self
    }

    pub fn add_chain(mut self, config: FallbackChainConfig) -> Self {
        self.chains.insert(config.id.clone(), config);
        self
    }

    pub fn validate(&self) -> Vec<String> {
        let mut errors = vec![];

        if self.routers.is_empty() {
            errors.push("no routers configured".into());
        }

        for (id, router) in &self.routers {
            if router.url.is_empty() {
                errors.push(format!("router '{}' has empty URL", id));
            }
        }

        for (id, chain) in &self.chains {
            if chain.models.is_empty() {
                errors.push(format!("chain '{}' has no models", id));
            }
            for entry in &chain.models {
                if !self.models.contains_key(&entry.model) {
                    errors.push(format!(
                        "chain '{}' references unknown model '{}'",
                        id, entry.model
                    ));
                }
            }
        }

        for (id, model) in &self.models {
            if !self.routers.contains_key(&model.provider) {
                errors.push(format!(
                    "model '{}' provider '{}' has no corresponding router",
                    id, model.provider
                ));
            }
        }

        errors
    }

    pub fn build(self) -> Result<GatewayConfig, Vec<String>> {
        let errors = self.validate();
        if !errors.is_empty() {
            return Err(errors);
        }
        Ok(GatewayConfig {
            routers: self.routers,
            models: self.models,
            chains: self.chains,
        })
    }

    pub fn from_config(config: GatewayConfig) -> Self {
        Self {
            routers: config.routers,
            models: config.models,
            chains: config.chains,
        }
    }
}

impl Default for GatewayBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ollama_router() -> RouterConfig {
        RouterConfig {
            url: "http://localhost:11434".into(),
            api_key_env: None,
            enabled: true,
            timeout_ms: None,
            headers: HashMap::new(),
        }
    }

    fn gemma_model() -> ModelConfig {
        ModelConfig {
            id: "gemma3:27b".into(),
            api_model_id: None,
            provider: "ollama".into(),
            capabilities: vec![Capability::Chat],
            context_window: 8192,
            max_output_tokens: 2048,
            pricing: None,
        }
    }

    fn chat_chain() -> FallbackChainConfig {
        FallbackChainConfig {
            id: "chat".into(),
            capability: Capability::Chat,
            models: vec![ChainEntry {
                model: "gemma3:27b".into(),
                router: Some("ollama".into()),
                api_model_id: None,
                priority: 1,
            }],
            fallback_triggers: vec![FallbackTrigger::Timeout],
        }
    }

    #[test]
    fn valid_config_builds() {
        let config = GatewayBuilder::new()
            .add_router("ollama", ollama_router())
            .add_model(gemma_model())
            .add_chain(chat_chain())
            .build();

        assert!(config.is_ok());
        let config = config.unwrap();
        assert_eq!(config.routers.len(), 1);
        assert_eq!(config.models.len(), 1);
        assert_eq!(config.chains.len(), 1);
    }

    #[test]
    fn fails_with_no_routers() {
        let result = GatewayBuilder::new()
            .add_model(gemma_model())
            .build();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("no routers")));
    }

    #[test]
    fn fails_with_empty_router_url() {
        let mut router = ollama_router();
        router.url = "".into();

        let result = GatewayBuilder::new()
            .add_router("ollama", router)
            .add_model(gemma_model())
            .build();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("empty URL")));
    }

    #[test]
    fn fails_with_dangling_model_ref_in_chain() {
        let chain = FallbackChainConfig {
            id: "bad_chain".into(),
            capability: Capability::Chat,
            models: vec![ChainEntry {
                model: "nonexistent".into(),
                router: Some("ollama".into()),
                api_model_id: None,
                priority: 1,
            }],
            fallback_triggers: vec![],
        };

        let result = GatewayBuilder::new()
            .add_router("ollama", ollama_router())
            .add_chain(chain)
            .build();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("unknown model")));
    }

    #[test]
    fn fails_with_model_missing_router() {
        let mut model = gemma_model();
        model.provider = "nonexistent".into();

        let result = GatewayBuilder::new()
            .add_router("ollama", ollama_router())
            .add_model(model)
            .build();

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(errors.iter().any(|e| e.contains("no corresponding router")));
    }

    #[test]
    fn collects_all_errors() {
        // Empty router URL + dangling model ref + model without router
        let router = RouterConfig {
            url: "".into(),
            ..ollama_router()
        };
        let mut model = gemma_model();
        model.provider = "ghost".into();
        let chain = FallbackChainConfig {
            id: "c".into(),
            capability: Capability::Chat,
            models: vec![ChainEntry {
                model: "ghost_model".into(),
                router: None,
                api_model_id: None,
                priority: 1,
            }],
            fallback_triggers: vec![],
        };

        let result = GatewayBuilder::new()
            .add_router("ollama", router)
            .add_model(model)
            .add_chain(chain)
            .build();

        let errors = result.unwrap_err();
        assert!(errors.len() >= 3, "expected 3+ errors, got: {:?}", errors);
    }

    #[test]
    fn from_config_roundtrip() {
        let original = GatewayBuilder::new()
            .add_router("ollama", ollama_router())
            .add_model(gemma_model())
            .add_chain(chat_chain())
            .build()
            .unwrap();

        let rebuilt = GatewayBuilder::from_config(original.clone()).build().unwrap();
        assert_eq!(rebuilt.routers.len(), original.routers.len());
        assert_eq!(rebuilt.models.len(), original.models.len());
        assert_eq!(rebuilt.chains.len(), original.chains.len());
    }
}
```

- [ ] **Step 2: Add config to lib.rs and update re-exports**

```rust
pub mod adapters;
pub mod budget;
pub mod circuit_breaker;
pub mod config;
pub mod engine;
pub mod selection;
pub mod types;

// Re-export key types for ergonomic API
pub use config::GatewayBuilder;
pub use engine::Gateway;
pub use types::capability::Capability;
pub use types::error::GatewayError;
pub use types::request::{InferenceRequest, InferenceResponse};
```

- [ ] **Step 3: Run tests**

Run: `cargo test --manifest-path crates/gateway/Cargo.toml -- config`
Expected: all 7 tests PASS

- [ ] **Step 4: Run full test suite**

Run: `cargo test --manifest-path crates/gateway/Cargo.toml`
Expected: ALL tests PASS across all modules

- [ ] **Step 5: Commit**

```bash
git add crates/gateway/src/config.rs crates/gateway/src/lib.rs
git commit -m "feat: add config builder with validation — completes Phase 1 gateway"
```

---

### Task 12: Final verification and initial commit of docs

**Files:**
- Modify: none (verification only)

- [ ] **Step 1: Run full test suite**

Run: `cargo test --manifest-path crates/gateway/Cargo.toml 2>&1`
Expected: all tests pass, 0 failures

- [ ] **Step 2: Run clippy**

Run: `cargo clippy --manifest-path crates/gateway/Cargo.toml -- -D warnings 2>&1`
Expected: no warnings

- [ ] **Step 3: Fix any clippy warnings**

Address each warning. Common ones: unused imports, unnecessary clones, missing `Default` implementations.

- [ ] **Step 4: Run tests again after clippy fixes**

Run: `cargo test --manifest-path crates/gateway/Cargo.toml`
Expected: still all pass

- [ ] **Step 5: Commit docs**

```bash
git add docs/
git commit -m "docs: add ideas, journeys, design, and implementation plan"
```
