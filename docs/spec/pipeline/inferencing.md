# 推 · Pipeline · Inferencing

**Owner files:**
- Gateway: external repo — `github.com/sensei-hq/gateway` (git dep `gateway-embedded` in senseid)
- Chain definitions: `~/.sensei/gateway/chains/*.yaml`
- Adapters (per provider): `gateway/src/adapters/{ollama,anthropic,openai,groq,...}.rs`
- MOE consensus: `gateway/src/consensus.rs`
- Budget / circuit breaker: `gateway/src/budget.rs`, `gateway/src/breaker.rs`
- Sensei-side status: `crates/senseid/src/api/handlers/gateway.rs::gateway_status`

**Companion design doc:** `docs/archive/ideas/28-inference-gateway.md` + `docs/archive/ideas/20-local-inference.md`.

## Purpose

Every LLM call in sensei — narration-cache generation, memory
consolidation, drift-fix suggestions, MOE reasoning panels — goes
through the inference gateway. The gateway does five things:

1. **Route by capability** — pick a provider chain suited for the
   task (fast small-model for insight copy; larger model for
   reasoning; embedded for anything the user wants private).
2. **Fallback gracefully** — if the primary provider is down or
   times out, try the next one; if everything fails, surface a
   specific error and let the caller fall back to the deterministic
   template.
3. **Track budget and cost** — per-user (and per-Dōjō) daily /
   monthly budgets with soft warnings and hard cuts. Costs
   attributed per chain per provider.
4. **Circuit-break per endpoint** — an endpoint that fails N times
   in a rolling window is skipped for a cool-down period so a
   single flaky provider doesn't stall every request.
5. **Consensus (MOE)** — for high-stakes calls (a memory that
   might contradict an existing one, a pattern promotion, a
   negative-verdict analysis), run **propose → challenge →
   synthesize** across two or three models and return the
   synthesized answer with a confidence score and the raw traces.

Kanji is 推 — *inference / reasoning*.

## Data invariants

### Chain configuration

A chain is a named ordered list of providers with fallback rules.

    # ~/.sensei/gateway/chains/narration-cache.yaml
    name: narration-cache
    primary:
      provider: ollama
      model: gemma4
      timeout_ms: 400
      max_tokens: 120
      temperature: 0.3
    fallback: []       # offline-first: no remote fallback for this chain
    circuit_breaker:
      window_secs: 60
      max_failures: 5
      cool_down_secs: 120

Sensei ships default chains and users override in Preferences →
Inference:

| Chain | Primary | Purpose |
|---|---|---|
| `narration-cache` | ollama gemma4 | Mentor voice ([[pipeline/narration-cache]]) |
| `text-chat` | ollama gemma4 (offline-first) | Fallback assistant chat when no external ACP set |
| `reasoning` | ollama gemma4 → optional remote | Memory consolidation, pattern promotion analysis |
| `consensus` | 2–3 models in parallel | MOE panel |
| `embedding` | ollama nomic-embed-text | Vector search + libs docs |
| `image` | (deferred) | Image generation |
| `voice` | (deferred) | Voice input/output |

### Adapters

Each provider is an adapter — same trait, isolated file. No
provider-specific code leaks into shared infrastructure.

    trait ProviderAdapter {
        fn name(&self) -> ProviderName;
        async fn complete(&self, req: CompletionRequest) -> Result<CompletionResponse>;
        async fn embed(&self, req: EmbedRequest) -> Result<EmbedResponse>;
        fn healthcheck(&self) -> ProviderHealth;
        fn pricing(&self) -> ProviderPricing;
    }

Live adapters: ollama, anthropic, openai, groq. New family = new
file. No spaghetti.

### MOE consensus protocol

    propose:
      run 2–3 models in parallel with the same prompt
      each returns { text, reasoning_trace }
    challenge:
      show each model the others' answers
      each returns { agreements, disagreements, refined_answer }
    synthesize:
      if all agree → return the shared answer with confidence=high
      if partial agreement → return the majority answer with confidence=medium + disagreements
      if disagreement → return the primary model's answer with confidence=low + disagreements
      also return raw traces for the reasoning panel

The panel is exposed on the insights UI so the user can see the
debate ([[screen/observatory-insights]] optional reasoning drawer).

### Budget & cost tracking

- `sensei.inference_calls` — one row per completion / embed call:
  `{ chain, provider, model, tokens_in, tokens_out, cost_usd, ms, ok, error, called_at, called_by (user_id | session_id) }`
- Budgets: per-user daily + monthly, per-Dōjō monthly. Soft
  warning at 80%, hard cut at 100%. Cuts always route to
  gateway-embedded (ollama) so the user never fully blocks.
- Cost surface: Preferences → Inference shows the running total.

## Signals produced

| Signal | Consumer |
|---|---|
| `text/embed/reason` responses | every LLM caller |
| `sensei.inference_calls` rows | cost surface + gateway_status endpoint |
| MOE consensus + confidence | insight generation, memory consolidation, pattern promotion |
| Circuit-breaker trip events | logs + a warning banner if the primary chain is degraded |
| Fallback usage | told to narration-cache so it knows to render fallback text |

## Done gate

- Every LLM call goes through the gateway — no direct provider
  calls in sensei / mcp / senseid code.
- Ollama-only offline install works: every chain resolves to
  gemma4 with a specific fallback path when a chain has a
  remote-only primary.
- MOE consensus returns confidence-labelled answers with the raw
  traces accessible for UI display.
- Circuit-breaker trips on N failures and clears after the
  cool-down.
- Budget cuts route to embedded ollama automatically without
  breaking the caller.
- Cost surface reads real cost from `sensei.inference_calls`.

Optional check:
```
mcp_call gateway_status | jq '{healthy: .healthy, chains: .chains}'

# Are calls flowing?
psql -A -t -c "select chain, provider, count(*) from sensei.inference_calls
                where called_at > now() - interval '1 hour'
                group by chain, provider" -d sensei
```

## Wrong gate

- **Sensei code imports an anthropic / openai SDK directly.**
  Adapter isolation broken. New provider must be added under
  the adapter trait.
- **A chain's primary fails silently and the caller doesn't
  know.** Insight-copy renders model output that's actually
  fallback text OR the caller crashes.
- **Circuit breaker never trips.** Failing endpoint eats every
  request.
- **Budget over 100% but calls continue on the expensive
  provider.** Cut logic didn't route to embedded.
- **MOE panel returns identical model responses always.**
  All models pointed at the same endpoint.
- **Cost surface shows 0 despite calls flowing.**
  `inference_calls` insert missing on the successful path.
- **Reasoning-trace not preserved.** The MOE panel can't render
  the debate.

## Related

- [[pipeline/narration-cache]] — biggest consumer today
- [[pipeline/memory]] — consolidation via MOE reasoning
- [[pipeline/insights]] — MOE reasoning panel for high-stakes
  recommendations
- [[pipeline/patterns]] — pattern promotion analysis
- [[screen/preferences]] — Inference pane (chain config + costs)
- (memory: project_p2_sweep_2026_07) (memory) — gateway-embedded git
  dep at `sensei-hq/gateway` @ 01d0ab2
