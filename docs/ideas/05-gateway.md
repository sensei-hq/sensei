# Gateway

The gateway is Sensei's inference routing layer. It decides which AI model handles which task, manages provider health and costs, and degrades gracefully when something is unavailable. You rarely interact with it directly -- it works behind the scenes whenever Sensei needs to call a model.

---

## Where it surfaces

The gateway is not a standalone screen. It shows up in three places:

- **Setup step 8 (Instruments)** -- configure which models and providers Sensei uses
- **Observatory cost tracking** -- see what inference is costing you, daily and monthly
- **Project search quality** -- semantic search depends on embeddings the gateway produces

Everything else is invisible. The gateway picks models, routes requests, handles failures, and tracks costs without you needing to think about it.

---

## Zero-config start

If you have Ollama running locally with a few models pulled, Sensei detects it automatically during bootstrap. It probes `localhost:11434`, discovers which models are available, builds default chains for each task type, and starts working. No configuration needed.

On first indexing run, the gateway routes embedding generation, classification, and summarization through your local models. When you search your project, it embeds your query locally and matches against stored vectors. All of this is free and entirely on your machine.

If Ollama isn't running, the gateway degrades silently. Semantic search falls back to keyword matching. Classification falls back to heuristics. Nothing breaks -- features just have less data to work with.

---

## Configuring inference

You can configure inference providers and how tasks are routed in settings.

### Local models (Ollama)

Sensei auto-detects models you've pulled. Common setups:

- **gemma3:27b** -- general reasoning, classification, summarization
- **all-minilm** -- fast embedding generation for semantic search
- **qwen3:14b** -- alternative reasoning model for consensus panels

### External providers

Add API keys for cloud providers when you want higher-quality responses or need capabilities your local models can't match:

- **Anthropic** -- Claude models for complex reasoning and code generation
- **OpenAI** -- GPT models as alternatives or fallbacks
- **Google** -- Gemini models (future)

### Per-task routing

Different tasks have different needs. Sensei lets you route each task type to the best model for the job:

| Task | Default routing | Why |
|------|----------------|-----|
| Embedding | Local only | High volume, privacy-sensitive, needs consistency |
| Classification | Local first | High frequency, simple enough for local models |
| Chat | External first | Quality-critical, user is waiting for the response |
| Consensus | Mixed | Multiple models debate, can blend local and external |

You can override these defaults. Want everything local? Set routing preference to local-only. Want maximum quality? Point everything at your external provider. Sensei adapts.

---

## Budget management

Local models are free. External providers charge per token. The gateway tracks both.

### Setting limits

Configure daily and monthly spending caps in settings. When you approach a limit, Sensei logs a warning. When you hit it, external calls automatically fall back to local models. If local models aren't available either, the system degrades to heuristics.

The rule is simple: **never block, always degrade.** Sensei will never stop working because of a budget limit. It will use less capable methods, but it will keep going.

### Spend tracking

The observatory shows your inference spending:

- Today's cost and remaining daily budget
- Monthly cost and remaining monthly budget
- Breakdown by model -- which models cost the most and why
- Trend over time -- is spending going up or down

All local inference shows as zero cost, so you can see exactly what the external providers are charging.

---

## Provider health

The gateway monitors every provider endpoint and handles failures automatically.

### Circuit breaker

Each provider endpoint has a circuit breaker. After several consecutive failures, the gateway stops sending requests to that endpoint for a cooldown period. After the cooldown, it sends a test request. If the test succeeds, the endpoint goes back to normal. If it fails, the cooldown restarts.

This prevents hammering a failing provider and lets the gateway recover automatically when the provider comes back online.

### Fallback chains

Every task type has a fallback chain -- an ordered list of models to try. If the first model fails, the gateway tries the next one. If all real models fail, a built-in noop adapter returns a structured "unavailable" response so the calling system can fall back to its own heuristics.

### Status in settings

Settings shows the current health of each configured provider: which endpoints are active, which are in cooldown, and the current degradation level (full capability, reduced quality, local only, or noop).

---

## Consensus

For high-stakes decisions -- diagnosing why your FTR dropped, evaluating an emerging pattern, assessing the impact of a change -- the gateway runs a consensus panel. This is a structured debate between multiple models:

1. **Proposer** analyzes the situation and proposes a conclusion
2. **Challenger** reviews the proposal and pushes back or refines it
3. **Synthesizer** reads both perspectives and produces a final consensus with a confidence score

The result includes the conclusion, the reasoning trace from all three models, any unresolved disagreements, and a confidence rating. Low-confidence results are flagged so Sensei can present them as uncertain rather than authoritative.

You can configure which models fill each role. A common setup uses two different local models for proposer and challenger (to get genuine diversity of perspective) and either a local or external model for synthesis.

---

## Voice

The gateway handles speech-to-text and text-to-speech through the same chain and fallback infrastructure.

### Speech-to-text

Whisper models (local via Ollama or external via API) transcribe spoken input. Use cases include dictating coding instructions, narrating code reviews, or transcribing meeting notes. Streaming transcription supports real-time input from a microphone.

### Text-to-speech

Local TTS engines or external APIs synthesize spoken output. Sensei can read explanations aloud while you read code, narrate review comments, or provide audio output for accessibility.

### Voice pipeline

The natural flow is: microphone to STT to chat to TTS to speaker. The gateway handles all three steps independently through their own chains, so you can mix local STT with external chat with local TTS -- whatever gives the best latency and quality for your setup.

---

## Reference

- Implementation details: [design/03-gateway.md](../design/03-gateway.md)
