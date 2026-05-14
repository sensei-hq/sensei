---
name: Chat Chain
description: Conversational interaction chain — multi-turn, tool use, quality-optimized routing for user-facing requests
date: 2026-04-24
status: idea
related: 04-chain-inference.md, 08-chain-voice.md, 09-fallback-degradation.md
---

# Chat Chain

## Purpose

The chat chain handles **user-facing conversational requests** — the highest quality bar in the gateway. These are interactions where a human is waiting for a response:

- **Direct user queries** via MCP tools ("explain this function", "suggest a refactor")
- **Tool-assisted reasoning** — multi-turn with tool calls (search, get_callers, etc.)
- **Code generation** — write, review, or modify code based on instructions
- **Interactive analysis** — back-and-forth investigation of issues

## Why quality-first routing

Unlike inference tasks (background, latency-tolerant), chat is:
- **Latency-sensitive** — user is waiting; first-token time matters
- **Quality-critical** — bad answers erode trust immediately
- **Low frequency** — one request at a time, not thousands per indexing run
- **Context-heavy** — multi-turn conversations, large code snippets

This inverts the inference chain's priorities: prefer quality over cost, accept external providers, and optimize for streaming latency.

## Default chain

```yaml
chat_chain:
  capability: chat
  fallback_triggers: [timeout, rate_limit, provider_error, budget_exceeded]
  models:
    - model: claude-sonnet-4-6
      router: anthropic
      priority: 1          # highest quality for code tasks
    - model: gpt-4o
      router: openai
      priority: 2          # strong alternative
    - model: gemma3:27b
      router: ollama
      priority: 3          # local fallback, lower quality but free
    - model: gemma3:12b
      router: ollama
      priority: 4          # minimal fallback
```

**Note:** Chat defaults to external providers first (quality > cost). Local models are fallback for when external is unavailable or budget is exhausted.

For users who prefer local-only:

```yaml
chat_chain_local:
  capability: chat
  fallback_triggers: [timeout, model_unavailable]
  models:
    - model: gemma3:27b
      router: ollama
      priority: 1
    - model: qwen3:14b
      router: ollama
      priority: 2
```

## Multi-turn context

Chat requests include conversation history:

```rust
pub struct ChatPayload {
    pub messages: Vec<Message>,       // full conversation history
    pub system: Option<String>,       // system prompt
    pub tools: Option<Vec<ToolDef>>,  // available tools
    pub max_tokens: Option<u32>,
    pub temperature: Option<f32>,
}
```

The engine passes the full payload to the adapter. Context window limits are validated during model selection — a model with a 8K context window is skipped if the conversation exceeds 8K tokens.

## Tool use

Chat is the only chain that supports tool calling:

```rust
pub struct ToolDef {
    pub name: String,
    pub description: String,
    pub input_schema: serde_json::Value,
}

pub struct ToolCall {
    pub id: String,
    pub name: String,
    pub arguments: serde_json::Value,
}
```

Tool use requires adapter-specific formatting:
- **Anthropic:** `tools` array → `tool_use` content blocks → `tool_result` messages
- **OpenAI:** `tools` array → `tool_calls` in assistant message → `tool` role messages
- **Ollama:** follows OpenAI format (when model supports it)

The gateway handles format translation — the caller sends a generic `ToolDef`, and the adapter converts to provider format.

## Streaming priority

Chat always streams when possible. First-token latency is the primary UX metric:

| Model | Typical first-token | Throughput |
|-------|-------------------|------------|
| claude-sonnet-4-6 (Anthropic) | ~300ms | ~80 tok/s |
| gpt-4o (OpenAI) | ~200ms | ~100 tok/s |
| gemma3:27b (Ollama, M4 Max) | ~500ms | ~40 tok/s |

Mid-stream fallback: if the stream breaks (provider error), the engine switches to the next candidate and emits `ProviderSwitch`. The user sees a brief pause, then content resumes from the beginning.

## Budget interaction

Chat uses external providers by default, so budget matters:
- If budget exceeded → fall back to local models (priority 3+)
- If no local models → noop with message "inference budget exhausted"
- Budget check happens during model selection, not mid-stream

## Open questions

| # | Question |
|---|----------|
| 1 | Should the chain auto-select between quality tiers based on question complexity? Simple questions → local, complex → external. |
| 2 | Should mid-stream fallback attempt to summarize partial output from the failed provider, or always start fresh? |
| 3 | Should tool definitions be filtered per-model? Some local models handle tool use poorly. |
| 4 | Should there be a separate "code generation" chain optimized for code-specific models (Codestral, DeepSeek Coder)? |
