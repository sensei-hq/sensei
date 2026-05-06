---
name: Configuration
description: Builder pattern, validation, DB-backed config, hot reload
date: 2026-04-24
status: idea
related: 01-gateway-engine.md, 02-adapter-system.md
reference: /Users/Jerry/Developer/strategos/packages/gateway/src/builder.ts
---

# Configuration

## Problem

The gateway needs to be configured with routers, models, chains, and budget settings. This config must be:
- **Validated** — bad config should fail at startup, not at runtime
- **Stored** — DB-backed so changes persist across restarts
- **Hot-reloadable** — config changes apply without daemon restart
- **Buildable** — ergonomic API for tests and initial setup

## GatewayConfig

```rust
pub struct GatewayConfig {
    pub routers: HashMap<String, RouterConfig>,
    pub models: HashMap<String, ModelConfig>,
    pub chains: HashMap<String, FallbackChainConfig>,
    pub defaults: DefaultsConfig,
    pub budget: Budget,
}

pub struct DefaultsConfig {
    pub timeout: Duration,           // default request timeout
    pub max_retries: u8,             // within a single adapter
    pub streaming: bool,             // default to streaming
}
```

## Builder pattern

```rust
pub struct GatewayBuilder { /* ... */ }

impl GatewayBuilder {
    pub fn new() -> Self;

    // Add components
    pub fn add_router(mut self, id: &str, config: RouterConfig) -> Self;
    pub fn add_model(mut self, config: ModelConfig) -> Self;
    pub fn add_chain(mut self, config: FallbackChainConfig) -> Self;
    pub fn set_defaults(mut self, defaults: DefaultsConfig) -> Self;
    pub fn set_budget(mut self, budget: Budget) -> Self;

    // Validate and build
    pub fn validate(&self) -> Vec<String>;
    pub fn build(self) -> Result<GatewayConfig, Vec<String>>;

    // Load from existing
    pub fn from_config(config: GatewayConfig) -> Self;
}
```

### Validation rules

| Rule | Error message |
|------|--------------|
| At least one router | "No routers configured" |
| Router has URL | "Router '{id}' missing URL" |
| Chain models exist | "Chain '{id}' references unknown model '{model}'" |
| Model provider has router | "Model '{id}' provider '{p}' has no corresponding router" |
| Chain capability valid | "Chain '{id}' has invalid capability '{cap}'" |
| No duplicate chain capabilities | "Multiple chains for capability '{cap}'" |

`build()` returns **all** errors, not just the first — so the user can fix everything at once.

## DB-backed configuration

### Adapter configs in `services` table

Each provider adapter is stored as a service:

```json
{"name": "ollama-local", "kind": "inference", "protocol": "ollama",
 "config": {"url": "http://localhost:11434"}}

{"name": "anthropic", "kind": "inference", "protocol": "anthropic",
 "config": {"api_key_env": "ANTHROPIC_API_KEY"}}
```

### Gateway settings in `settings` table

Chains, budget, defaults stored as JSON settings:

```json
{"key": "gateway.chains", "value": { /* chain configs */ }}
{"key": "gateway.budget", "value": {"daily_limit": 5.0, "monthly_limit": 50.0}}
{"key": "gateway.defaults", "value": {"timeout_ms": 30000, "streaming": true}}
```

### Loading from DB

```rust
impl GatewayBuilder {
    pub async fn from_db(store: &PgStore) -> Result<Self, Error> {
        let services = store.list_services_by_kind("inference").await?;
        let chains = store.get_config("gateway.chains").await?;
        let budget = store.get_config("gateway.budget").await?;
        let defaults = store.get_config("gateway.defaults").await?;

        let mut builder = GatewayBuilder::new();
        // ... hydrate from DB values
        Ok(builder)
    }
}
```

## Hot reload

When config changes in DB:

```rust
impl Gateway {
    pub fn update_config(&self, config: GatewayConfig) {
        *self.config.write() = config;
        // Selection service and budget manager read from the same Arc<RwLock>
    }
}
```

Trigger: DB change notification, API call, or periodic poll.

## Default configuration

On first run with no DB config, the gateway uses sensible defaults:

```rust
impl GatewayConfig {
    pub fn default_local_only() -> Self {
        GatewayBuilder::new()
            .add_router("ollama", RouterConfig {
                url: "http://localhost:11434".into(),
                ..Default::default()
            })
            .add_model(ModelConfig {
                id: "gemma3:27b".into(),
                provider: "ollama".into(),
                capabilities: vec![Chat, Classify, Summarize],
                ..Default::default()
            })
            .add_model(ModelConfig {
                id: "all-minilm:l6-v2".into(),
                provider: "ollama".into(),
                capabilities: vec![Embed],
                ..Default::default()
            })
            .add_chain(/* embed chain */)
            .add_chain(/* inference chain */)
            .add_chain(/* chat chain */)
            .build()
            .expect("default config should be valid")
    }
}
```

## Open questions

| # | Question |
|---|----------|
| 1 | Should config changes be versioned? Track what changed, when, by whom. |
| 2 | Should there be a CLI command to manage gateway config? `gateway config add-router ollama ...` |
| 3 | Should the builder support TOML/YAML file loading in addition to DB? Useful for version-controlled config. |
