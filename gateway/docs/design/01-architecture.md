---
name: Architecture
description: Crate structure, module layout, dependency graph, and key decisions for the Rust port of Strategos gateway
date: 2026-04-24
status: design
reference: /Users/Jerry/Developer/strategos/packages/gateway
---

# Architecture

## Crate structure

The gateway is a standalone Rust crate with no sensei-specific dependencies. Sensei consumes it as a library; other applications can too.

```
gateway/
├── crates/
│   └── gateway/
│       ├── Cargo.toml
│       └── src/
│           ├── lib.rs              ← public API: Gateway, GatewayBuilder
│           ├── engine.rs           ← execution engine, fallback chain walking
│           ├── selection.rs        ← model selection service (3-tier)
│           ├── circuit_breaker.rs  ← per-endpoint failure management
│           ├── budget.rs           ← cost estimation, budget filtering
│           ├── config.rs           ← GatewayBuilder, validation, defaults
│           ├── consensus.rs        ← MOE consensus protocol (wrapper)
│           ├── types/
│           │   ├── mod.rs
│           │   ├── config.rs       ← RouterConfig, ModelConfig, ChainConfig
│           │   ├── request.rs      ← InferenceRequest, InferenceResponse, StreamEvent
│           │   ├── cost.rs         ← Cost, CostEstimate, ModelPricing, Budget
│           │   ├── error.rs        ← GatewayError enum
│           │   └── trace.rs        ← ExecutionTrace, Attempt, hook events
│           └── adapters/
│               ├── mod.rs          ← InferenceAdapter trait, AdapterRegistry
│               ├── base.rs         ← HTTP utilities, SSE parsing, error extraction
│               ├── ollama.rs       ← Ollama adapter (OpenAI-compatible)
│               ├── anthropic.rs    ← Anthropic adapter
│               ├── openai.rs       ← OpenAI-compatible adapter
│               └── noop.rs         ← Graceful degradation adapter
└── docs/
    ├── ideas/
    ├── journeys/
    └── design/
```

## What we port from Strategos

| Strategos component | Gateway equivalent | Changes |
|--------------------|-------------------|---------|
| `engine.ts` (767 lines) | `engine.rs` | Same architecture; Rust async/await instead of Promise |
| `model-selection.ts` | `selection.rs` | Same 3-tier resolution |
| `circuit-breaker.ts` | `circuit_breaker.rs` | Same state machine; `Arc<Mutex<>>` for thread safety |
| `budget-filter.ts` | `budget.rs` | Same logic; add in-memory spend cache |
| `builder.ts` | `config.rs` | Same builder pattern; add `from_db()` for persistent config |
| `core/types/*.ts` | `types/*.rs` | Subset — drop image/video types initially |
| `adapters/ollama.ts` | `adapters/ollama.rs` | Same (OpenAI-compatible endpoints) |
| `adapters/anthropic.ts` | `adapters/anthropic.rs` | Same request/response translation |
| `adapters/openai.ts` | `adapters/openai.rs` | Same; also covers OpenRouter |

## What we don't port

- Image generation (DALL-E, Midjourney) — not needed
- Video generation (Sora, Runway) — not needed
- Azure/AWS Bedrock adapters — Phase 1 doesn't need cloud provider variants
- OpenRouter/Grok adapters — can add later (OpenRouter uses OpenAI format anyway)
- Webhook/async polling — synchronous is fine for gateway's use cases

## What's new (not in Strategos)

| Component | Purpose |
|-----------|---------|
| `consensus.rs` | MOE consensus protocol — multi-model debate |
| `adapters/noop.rs` | Graceful degradation when no providers available |
| Consolidation chain | Knowledge merging (new capability type) |
| Voice chains (STT/TTS) | Speech-to-text, text-to-speech |
| DB-backed config | Persistent config via PostgreSQL |
| Spend caching | In-memory budget tracking with periodic DB sync |

## Dependency graph

```
                    ┌──────────────┐
                    │   lib.rs     │ (public API)
                    │   Gateway    │
                    │   Builder    │
                    └──────┬───────┘
                           │
              ┌────────────┼────────────┐
              ▼            ▼            ▼
        ┌──────────┐ ┌──────────┐ ┌──────────┐
        │ engine   │ │ config   │ │consensus │
        │          │ │ (builder)│ │ (MOE)    │
        └────┬─────┘ └──────────┘ └────┬─────┘
             │                         │
    ┌────────┼─────────┐               │ (uses engine)
    ▼        ▼         ▼               │
┌────────┐┌────────┐┌────────┐         │
│selection││circuit ││ budget │         │
│        ││breaker ││        │         │
└────┬───┘└────────┘└────────┘         │
     │                                 │
     ▼                                 ▼
┌─────────────────────────────────────────┐
│              adapters/                   │
│  InferenceAdapter trait                  │
│  ollama | anthropic | openai | noop      │
└─────────────────────────────────────────┘
     │
     ▼
┌─────────────────────────────────────────┐
│              types/                      │
│  config | request | cost | error | trace │
└─────────────────────────────────────────┘
```

## Key Cargo dependencies

```toml
[dependencies]
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json", "stream"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
async-trait = "0.1"
futures = "0.3"
thiserror = "2"
uuid = { version = "1", features = ["v4", "serde"] }
chrono = { version = "0.4", features = ["serde"] }
tracing = "0.1"
pin-project-lite = "0.2"
```

## Concurrency model

- **Gateway** is `Arc`-wrapped and shared across tasks
- **Config** is `Arc<RwLock<GatewayConfig>>` — reads are non-blocking, writes are rare
- **Circuit breaker** uses `Arc<Mutex<HashMap>>` — short critical sections
- **Adapters** are `Arc<dyn InferenceAdapter>` — stateless, shared freely
- **Hooks** fire via `tokio::spawn` — never block the engine
- **Streaming** returns `Pin<Box<dyn Stream + Send>>` — composable with any async runtime

## MOE consensus as wrapper

The MOE consensus protocol is a **caller of the gateway**, not part of it:

```rust
// Consensus uses the gateway, not the other way around
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

This keeps the gateway engine clean — it routes single requests. Consensus composes multiple requests into a protocol.

## Build strategy

1. **Types first** — all types compile, serde round-trips tested
2. **Circuit breaker** — standalone, no dependencies on other gateway modules
3. **Adapters** — trait + Ollama first, others follow
4. **Selection + Budget** — pure logic, testable without network
5. **Engine** — brings everything together
6. **Consensus** — wrapper on top of working engine
7. **Config builder** — ergonomics layer, tested against validation rules
