---
name: Adapter System
description: Provider abstraction — trait definition, adapter registry, base HTTP utilities, SSE stream parsing
date: 2026-04-24
status: idea
related: 01-gateway-engine.md, 04-chain-inference.md
reference: /Users/Jerry/Developer/strategos/packages/adapters/src/
---

# Adapter System

## Problem

Each LLM provider has a different API: different auth, different request format, different streaming protocol, different error shapes. The gateway needs a uniform interface so the engine doesn't know or care which provider it's talking to.

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

`Send + Sync` bounds are required because adapters are shared across async tasks via `Arc`.

## Adapters

### Phase 1: Local

| Adapter | Protocol | Auth | Capabilities | Notes |
|---------|----------|------|-------------|-------|
| **Ollama** | OpenAI-compatible `/v1/` | None | Chat, Embed, Classify, Summarize | Extends OpenAI adapter logic; no auth; health check via `/api/tags` |
| **Noop** | None | None | All (returns "unavailable") | Graceful degradation fallback; never errors; always lowest priority |

### Phase 2: External

| Adapter | Protocol | Auth | Capabilities | Notes |
|---------|----------|------|-------------|-------|
| **Anthropic** | Anthropic API | `x-api-key` header | Chat | System prompt separate from messages; tool_use format differs from OpenAI |
| **OpenAI** | OpenAI API | `Bearer` token | Chat, Embed | Also covers OpenAI-compatible providers (OpenRouter, vLLM, etc.) |

### Phase 3: Media

| Adapter | Protocol | Auth | Capabilities | Notes |
|---------|----------|------|-------------|-------|
| **Whisper** | OpenAI `/v1/audio/transcriptions` or local whisper.cpp | Varies | Voice (STT) | Streaming input support for real-time transcription |
| **TTS** | OpenAI `/v1/audio/speech` or local Piper/Bark | Varies | Voice (TTS) | Streaming output for real-time speech |

## Adapter registry

```rust
pub struct AdapterRegistry {
    adapters: Arc<RwLock<HashMap<String, Arc<dyn InferenceAdapter>>>>,
}

impl AdapterRegistry {
    pub fn register(&self, adapter: Arc<dyn InferenceAdapter>);
    pub fn get(&self, id: &str) -> Option<Arc<dyn InferenceAdapter>>;
    pub fn list(&self) -> Vec<String>;
    pub fn unregister(&self, id: &str) -> bool;
}
```

## Base HTTP utilities

Shared across adapters to avoid duplication:

```rust
// POST JSON, parse response
async fn http_json<T: DeserializeOwned>(
    client: &reqwest::Client,
    router: &RouterConfig,
    path: &str,
    body: &impl Serialize,
) -> Result<T, GatewayError>;

// POST JSON, return SSE byte stream
async fn http_stream(
    client: &reqwest::Client,
    router: &RouterConfig,
    path: &str,
    body: &impl Serialize,
) -> Result<impl Stream<Item = Result<Bytes, reqwest::Error>>, GatewayError>;

// Parse SSE stream: yield JSON objects from `data: ` lines, stop on `[DONE]`
fn parse_sse_stream(
    stream: impl Stream<Item = Result<Bytes, reqwest::Error>>,
) -> impl Stream<Item = Result<serde_json::Value, GatewayError>>;
```

### Error extraction

Providers return errors in diverse formats. Normalize them:

```
OpenAI/Anthropic:  { "error": { "message": "..." } }
xAI/Grok:         { "error": "string message" }
FastAPI:           { "detail": "..." }
OAuth:             { "error_description": "..." }
HTTP status only:  (no body)
```

All mapped to `GatewayError` with appropriate variant (RateLimit, Auth, Timeout, etc.).

## Provider-specific request building

Each adapter translates the generic `InferenceRequest` to provider format:

### Anthropic differences from OpenAI
- System prompt: extracted to top-level `system` field (not in messages)
- Tool definitions: `{ name, description, input_schema }` (not `function`)
- Tool calls: `{ type: "tool_use", id, name, input }` (not `function_call`)
- Tool results: `{ type: "tool_result", tool_use_id, content }`
- Stop reasons: `end_turn | stop_sequence | max_tokens | tool_use`
- Auth: `x-api-key` + `anthropic-version: 2023-06-01`
- Streaming: `content_block_delta` events (not `choices[0].delta`)

### Ollama specifics
- No auth required (local)
- Uses OpenAI-compatible `/v1/` endpoints
- Health check: `GET /api/tags` returns available models
- Model names: `gemma3:27b`, `qwen3:14b` (no provider prefix)

## Open questions

| # | Question |
|---|----------|
| 1 | Should adapters own their HTTP client or share one? Shared = connection pooling benefits. Owned = isolation. |
| 2 | Should adapter registration be static (at startup) or dynamic (hot-plug when a provider becomes available)? |
| 3 | For the Ollama adapter, should we periodically probe `/api/tags` to update available models, or only on startup? |
