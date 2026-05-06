# sensei gateway

LLM routing library for Sensei. Multi-provider routing with fallback chains, circuit breakers, and per-request cost tracking.

## Providers

| Provider | Local | Notes |
|----------|-------|-------|
| Ollama | Yes | Preferred for local inference (Gemma 4) |
| Anthropic | No | Claude models |
| OpenAI | No | GPT models |
| Google | No | Gemini models |

## Capabilities

`chat` · `reasoning` · `embed` · `classify` · `summarize` · `vision` · `audio`

Each capability has an independently configured provider chain with automatic fallback.

## Build

```bash
# From this directory
cargo build

# Tests
cargo test --all-features
```

## Publish

```bash
cargo publish -p gateway
```
