---
name: Inference Gateway
description: Rust LLM routing engine — local models via Ollama, external providers via API, MOE consensus panel, fallback chains, budget management. Port of Strategos gateway concepts.
date: 2026-04-23
status: idea
related: 20-local-inference.md, 25-playground-and-insights.md, 27-developer-preferences.md
reference: /Users/Jerry/Developer/strategos/packages/gateway
moved_to: /Users/Jerry/Developer/gateway
---

> **Note:** The gateway has been extracted to its own repository: [`mizukisu/gateway`](https://github.com/mizukisu/gateway). Full documentation (ideas, journeys, design) lives there. This doc remains as the sensei-side reference for the integration points (MOE consensus wrapper, MCP exposure, daemon wiring).

# Inference Gateway

## Problem

Sensei needs LLM inference for multiple internal tasks: embedding generation, pattern classification, prompt classification, L2 summaries, MOE consensus reasoning, and eventually user-facing features. These tasks have different requirements:

- **Embeddings** — fast, cheap, runs constantly during indexing → local model
- **Classification** — simple, high-frequency → local model
- **L2 summaries** — moderate complexity → local model
- **MOE reasoning** — complex, multi-model debate → local or mixed local+external
- **User-facing queries** — highest quality needed → external provider (if configured)

No single model or provider serves all needs. We need a **routing layer** that picks the right model for each task, handles failures gracefully, manages costs, and supports the MOE consensus protocol.

## Prior art: Strategos Gateway

We've already built this in TypeScript: `/Users/Jerry/Developer/strategos/packages/gateway`. It's production-grade with:

| Capability | Strategos implementation |
|-----------|------------------------|
| 7 provider adapters | OpenAI, Anthropic, OpenRouter, Grok, Azure, AWS Bedrock, Ollama |
| Fallback chains | Sequential model attempts with triggerable failure detection |
| Circuit breaker | Per-endpoint: closed → open → half-open (5 failures → 5min timeout) |
| Budget management | Cost estimation, affordable model filtering, quota validation |
| Execution tracing | Full trace with attempt history, cost breakdown, timing |
| Streaming | AsyncGenerator-based with provider switching on failure |
| Hooks | onTaskStart, onAttempt, onTaskEnd, onCostIncurred, onActualCost |
| Builder pattern | Fluent config assembly for routers, models, chains |

### What we port to Rust

The **concepts and architecture** — not the TypeScript code. The Rust implementation will be simpler (sensei doesn't need image/video generation) but gains the MOE consensus protocol.

### What we don't port

- Image/video capabilities (not needed)
- Azure/AWS Bedrock adapters (Phase 1 is Ollama + Anthropic/OpenAI only)
- OpenRouter/Grok adapters (can add later)
- Webhook/async polling (synchronous is fine for sensei's use cases)

---

## Architecture

```
senseid (daemon)
├── inference/
│   ├── gateway.rs          — execution engine (port of engine.ts)
│   ├── config.rs           — configuration types (port of config.ts)
│   ├── router.rs           — model selection + fallback chains
│   ├── circuit_breaker.rs  — per-endpoint failure management
│   ├── budget.rs           — cost estimation + filtering
│   ├── consensus.rs        — MOE consensus protocol (NEW)
│   ├── trace.rs            — execution trace + hooks
│   └── adapters/
│       ├── mod.rs           — Adapter trait
│       ├── ollama.rs        — Local inference via Ollama HTTP API
│       ├── anthropic.rs     — Anthropic API (Claude models)
│       ├── openai.rs        — OpenAI-compatible API
│       └── noop.rs          — Graceful degradation (no inference available)
```

### Adapter trait (Rust)

```rust
#[async_trait]
pub trait InferenceAdapter: Send + Sync {
    fn id(&self) -> &str;
    fn supports(&self, capability: Capability) -> bool;

    async fn execute(
        &self,
        config: &RouterConfig,
        request: &InferenceRequest,
    ) -> Result<InferenceResponse, InferenceError>;

    async fn stream(
        &self,
        config: &RouterConfig,
        request: &InferenceRequest,
    ) -> Result<Pin<Box<dyn Stream<Item = StreamChunk>>>, InferenceError>;
}
```

### Capabilities (subset of Strategos)

```rust
pub enum Capability {
    Chat,       // conversational — reasoning, analysis, consensus
    Embed,      // embedding generation (384-dim, 768-dim)
    Classify,   // prompt classification, pattern detection
    Summarize,  // L2 logic flow summaries, docstring generation
}
```

---

## MOE Consensus Protocol (new — not in Strategos)

The MOE panel is a structured multi-model debate for generating high-confidence insights and recommendations.

### Protocol flow

```mermaid
sequenceDiagram
    participant E as Insights Engine
    participant G as Gateway
    participant A as Model A (Proposer)
    participant B as Model B (Challenger)
    participant C as Model C (Synthesizer)

    E->>G: consensus_request(signal, context)
    G->>A: "Analyze this signal. Propose root cause and action."
    A-->>G: proposal {cause, action, confidence}

    G->>B: "Here is Model A's proposal. Challenge or refine it."
    Note over G,B: Model B sees Model A's full output
    B-->>G: challenge {agrees[], disagrees[], refinements[]}

    G->>C: "Here are both analyses. Synthesize a consensus."
    Note over G,C: Model C sees both A and B outputs
    C-->>G: consensus {conclusion, confidence, action, disagreements[]}

    G-->>E: ConsensusResult {
        conclusion, confidence,
        action_proposed,
        reasoning_trace: [A, B, C],
        disagreements[]
    }
```

### Consensus configuration

```rust
pub struct ConsensusConfig {
    pub proposer: ModelRef,      // e.g. "ollama/gemma3:27b"
    pub challenger: ModelRef,    // e.g. "ollama/qwen3:14b"
    pub synthesizer: ModelRef,   // e.g. "ollama/gemma3:27b" or "anthropic/claude-haiku"
    pub max_rounds: u8,          // usually 1 (propose → challenge → synthesize)
    pub confidence_threshold: f32, // below this → flag as uncertain
}
```

### Consensus result

```rust
pub struct ConsensusResult {
    pub conclusion: String,
    pub confidence: f32,           // 0.0 - 1.0
    pub action_proposed: Option<Action>,
    pub reasoning_trace: Vec<ModelExchange>,
    pub disagreements: Vec<Disagreement>,
    pub models_used: Vec<String>,
    pub total_tokens: u32,
    pub duration_ms: u64,
}

pub struct ModelExchange {
    pub model: String,
    pub role: ConsensusRole,       // Proposer | Challenger | Synthesizer
    pub content: String,
    pub tokens: u32,
}

pub struct Disagreement {
    pub topic: String,
    pub model_a_position: String,
    pub model_b_position: String,
    pub resolved: bool,
}
```

### When consensus runs

| Trigger | Input | Expected output |
|---------|-------|----------------|
| FTR drop detected | FTR delta + recent sessions | Root cause + recommended action |
| Recurring correction | 3+ corrections on same topic | Persona/rule recommendation |
| Pattern emerging | 2+ similar code structures | Pattern classification + promote/ignore |
| Change impact negative | Before/after metrics | Why it went wrong + revision suggestion |
| Stale code risk | Untouched file + related changes | Assessment: safe to ignore or needs review |

---

## Fallback chains (ported from Strategos)

```rust
pub struct FallbackChain {
    pub id: String,
    pub capability: Capability,
    pub models: Vec<ChainEntry>,
    pub triggers: Vec<FallbackTrigger>,
}

pub struct ChainEntry {
    pub model: ModelRef,
    pub adapter: String,       // "ollama", "anthropic", "openai"
    pub priority: u8,
}

pub enum FallbackTrigger {
    RateLimit,
    Timeout,
    ProviderError,
    ModelUnavailable,
}
```

**Default chains:**

```
embed_chain:
  1. ollama/all-minilm (local, free)
  → fallback: none (embeddings must be local)

classify_chain:
  1. ollama/gemma3:12b (local, free)
  2. ollama/gemma3:27b (local, free, higher quality)
  → fallback on: timeout, model_unavailable

reasoning_chain:
  1. ollama/gemma3:27b (local, free)
  2. anthropic/claude-haiku (external, $)
  3. openai/gpt-4o-mini (external, $)
  → fallback on: timeout, rate_limit, provider_error

consensus_chain:
  proposer:    ollama/gemma3:27b
  challenger:  ollama/qwen3:14b
  synthesizer: ollama/gemma3:27b OR anthropic/claude-haiku
```

---

## Budget management (ported from Strategos)

Local models are free. External providers have per-token costs. The gateway tracks both.

```rust
pub struct CostEstimate {
    pub input_cost: f64,
    pub output_cost: f64,
    pub total_cost: f64,
    pub currency: String,
    pub model: String,
}

pub struct Budget {
    pub daily_limit: f64,       // e.g. $5.00/day for external calls
    pub monthly_limit: f64,     // e.g. $50/month
    pub spent_today: f64,
    pub spent_this_month: f64,
}
```

When budget is exceeded:
- External calls → fall back to local models
- If local models unavailable → graceful degradation (heuristics)
- Never block the user — always degrade, never fail

---

## Circuit breaker (ported from Strategos)

Per-endpoint (`adapter:model`) failure tracking:

```
Closed (normal) → 5 consecutive failures → Open (blocked)
Open → 5 minutes timeout → Half-open (test)
Half-open → 3 successes → Closed
Half-open → 1 failure → Open
```

Prevents hammering a failing provider. Auto-recovers when provider comes back.

---

## Integration with sensei

### Who calls the gateway

| Caller | Capability | Frequency |
|--------|-----------|-----------|
| Indexing pipeline | Embed, Summarize | During scan/re-index |
| Session lifecycle | Classify | Every user message |
| Insights engine | Chat (consensus) | After FTR changes |
| Workspace intelligence | Classify, Summarize | Daily scheduled |
| User (via MCP) | Chat | On-demand |

### Configuration (in `services` table)

Each adapter is a row in `services` with `kind = 'inference'`:

```json
{"name": "ollama-local", "protocol": "ollama", "kind": "inference",
 "config": {"url": "http://localhost:11434", "models": ["gemma3:27b", "qwen3:14b"]}}

{"name": "anthropic", "protocol": "anthropic", "kind": "inference",
 "config": {"api_key_env": "ANTHROPIC_API_KEY", "default_model": "claude-haiku-4-5"}}

{"name": "openai", "protocol": "openai", "kind": "inference",
 "config": {"api_key_env": "OPENAI_API_KEY", "default_model": "gpt-4o-mini"}}
```

### Consensus configuration (in `config` table or project settings)

```json
{
  "consensus": {
    "proposer": "ollama/gemma3:27b",
    "challenger": "ollama/qwen3:14b",
    "synthesizer": "ollama/gemma3:27b",
    "confidence_threshold": 0.7,
    "max_rounds": 1
  },
  "budget": {
    "daily_limit_usd": 5.0,
    "monthly_limit_usd": 50.0
  },
  "default_chains": {
    "embed": ["ollama/all-minilm"],
    "classify": ["ollama/gemma3:12b", "ollama/gemma3:27b"],
    "reasoning": ["ollama/gemma3:27b", "anthropic/claude-haiku", "openai/gpt-4o-mini"]
  }
}
```

---

## Phased rollout

### Phase 1: Ollama adapter (current focus)

- Single adapter: Ollama HTTP API at localhost:11434
- Capabilities: Embed, Classify, Summarize, Chat
- No fallback chains (only one provider)
- No budget management (local = free)
- Graceful degradation: if Ollama is down, skip inference tasks

### Phase 2: External providers + routing

- Add Anthropic and OpenAI adapters
- Fallback chains: local → external
- Budget management: daily/monthly limits for external calls
- Circuit breaker: per-endpoint failure tracking

### Phase 3: MOE consensus

- Consensus protocol: propose → challenge → synthesize
- Reasoning traces stored in `reasoning_traces` table
- Integration with insights engine for automated recommendations

### Phase 4: Rust-native inference (future)

- Replace Ollama HTTP adapter with ollama.cpp / candle / mistral.rs linked as Rust library
- Eliminates Ollama process dependency
- Direct GPU/Metal access for inference
- Model loading managed by the daemon

---

## Execution trace (ported from Strategos)

Every inference call produces a trace for debugging and analytics:

```rust
pub struct ExecutionTrace {
    pub request_id: String,
    pub capability: Capability,
    pub attempts: Vec<Attempt>,
    pub final_model: String,
    pub total_duration_ms: u64,
    pub estimated_cost: CostEstimate,
    pub actual_cost: Option<CostEstimate>,
}

pub struct Attempt {
    pub adapter: String,
    pub model: String,
    pub status: AttemptStatus,  // Success | Failed { reason }
    pub duration_ms: u64,
    pub tokens: Option<TokenUsage>,
}
```

Traces are stored for the insights engine to analyze tool/model effectiveness.

---

## Open questions

| # | Question |
|---|----------|
| 1 | Should the gateway be a separate crate (`sensei-inference`) or part of `senseid`? Separate crate is cleaner but adds build complexity. |
| 2 | For Phase 4 (Rust-native), which library? llama.cpp bindings (mature), candle (Hugging Face, pure Rust), mistral.rs? |
| 3 | Should consensus run synchronously (block until all 3 models respond) or stream partial results? |
| 4 | How do we handle model version changes? If user upgrades gemma3 from 12b to 27b, do we re-run cached inferences? |
| 5 | Should the gateway expose an HTTP API so other tools (not just senseid) can use it? This would make it a general-purpose local LLM router. |
| 6 | Token counting: use tiktoken (external) or approximate by character count? Accuracy matters for budget but tiktoken adds a dependency. |
