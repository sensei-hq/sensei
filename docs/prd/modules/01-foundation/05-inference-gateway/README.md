# Inference Gateway · M5

**Plane:** foundation · **Status:** [roadmap/01-foundation.md](../../../../roadmap/01-foundation.md) ·
**Serves:** every persona — narration-cache (M16), memory consolidation, drift fixes, insights, and MOE reasoning all route through here

## Purpose

Every LLM call in sensei — narration-cache generation, memory consolidation, drift-fix
suggestions, MOE reasoning panels — goes through one gateway. The gateway picks the right
provider chain for the task (fast small model for copy, larger model for reasoning, embedded
for anything the user wants private), falls back gracefully when a provider is down, tracks
budget and cost per user and per Dōjō, circuit-breaks flaky endpoints, and runs
propose → challenge → synthesize consensus for high-stakes calls. The user's feeling: *sensei
reasoning just works, stays within budget, and never blocks on a dead provider* — and on an
offline machine the whole thing runs on the local model.

## In scope / out of scope

- In: chain configuration (named ordered provider lists with fallback rules); provider
  adapters (ollama, anthropic, openai, groq — trait-isolated); MOE consensus protocol with
  confidence + raw traces; budget/cost tracking (`sensei.inference_calls`, per-user and
  per-Dōjō daily/monthly budgets, 80% soft / 100% hard cut routing to embedded); circuit
  breaker per endpoint; the `gateway_status` surface; model inventory + hardware-tier
  recommendation feed into Preferences → Inference.
- Out: which callers use what chains (their modules); embedding/vector-store internals
  (M6/M4); transmit/receive speech or image generation (deferred chains `image`, `voice`).

## Surfaces

- **Gateway** — external repo `sensei-hq/gateway` (git dep `gateway-embedded`); chains at
  `~/.sensei/gateway/chains/*.yaml`; adapters per provider; `consensus.rs`, `budget.rs`,
  `breaker.rs`.
- **API** — `GET /api/gateway/status` (`crates/senseid/src/api/handlers/gateway.rs`), the
  `mcp_call gateway_status` tool, `GET/PUT /api/preferences/inference` (chains, models,
  budget, circuit state).
- **App** — Preferences → Inference pane: hardware chip, installed models, cost running
  total, circuit state.

## Data invariants

- A chain is a named ordered list: `primary {provider, model, timeout_ms, max_tokens,
  temperature}` + `fallback[]` + `circuit_breaker {window_secs, max_failures,
  cool_down_secs}`. Default chains ship: `narration-cache`, `text-chat`, `reasoning`,
  `consensus`, `embedding` (`image`, `voice` deferred).
- **No direct provider calls outside the gateway** — sensei/mcp/senseid code never imports
  an anthropic/openai SDK directly; a new provider = a new adapter file.
- `sensei.inference_calls` — one row per completion/embed call:
  `{chain, provider, model, tokens_in, tokens_out, cost_usd, ms, ok, error, called_at,
  called_by}`.
- Budgets: per-user daily + monthly, per-Dōjō monthly; soft warning at 80%, hard cut at
  100%; a hard cut routes to gateway-embedded (ollama) — the user is never fully blocked.
- Circuit breaker: an endpoint failing N times in a rolling window is skipped for a
  cool-down, then re-probed (half-open) before full service resumes.
- MOE consensus: propose (2–3 models in parallel) → challenge (each sees the others) →
  synthesize (all-agree=high, partial=majority+medium, none=primary+low) — raw traces
  retained for the reasoning panel.

## Signals shown

| Element | Value | Meaning |
|---|---|---|
| Gateway status | `.healthy`, `.chains` | the routing surface in one call |
| Inference pane | hardware chip · installed models · cost total · circuit state | what the user tunes/pays |
| Cost surface | running total from `sensei.inference_calls` | real cost, not estimate |
| Circuit state | `closed \| open \| half-open` + `next_probe_at` | why a provider is skipped |
| MOE panel | confidence (high/medium/low) + disagreement list + traces | the debate, visible |

## Done gate

- Every LLM call in sensei goes through the gateway — grep clean of direct SDK imports.
- An Ollama-only offline install resolves every chain to the embedded model, with a defined
  fallback path when a chain has a remote-only primary.
- MOE returns confidence-labelled answers and the raw traces are accessible for UI.
- The breaker trips on N failures and clears after cool-down with a half-open re-probe.
- A budget hard cut routes to embedded ollama automatically without breaking the caller.
- The cost surface reads real cost from `sensei.inference_calls`.

```bash
mcp_call gateway_status | jq '{healthy: .healthy, chains: .chains}'
# flowing?
psql -A -t -c "select chain, provider, count(*) from sensei.inference_calls
               where called_at > now() - interval '1 hour'
               group by chain, provider" -d sensei
```

## Wrong gate

- **Sensei code imports an anthropic/openai SDK directly** — adapter isolation broken.
- **A chain's primary fails silently** — the caller renders fallback text as if it were the
  model's, or crashes.
- **The breaker never trips** — a failing endpoint eats every request.
- **Budget exceeds 100% but calls continue on the expensive provider** — cut logic didn't
  route to embedded.
- **MOE returns identical model responses always** — all models pointed at the same endpoint.
- **The cost surface shows 0 while calls flow** — `inference_calls` insert missing on the
  success path.
- **Reasoning traces are not preserved** — the MOE panel can't render the debate.

## Scenarios

- [`scenarios/inferencing.feature`](scenarios/inferencing.feature) — routing, fallback,
  consensus, budget/breaker, offline mode.

## Open gaps

- [C-12](../../../../roadmap/06-gaps.md) — inference-pipeline review not scoped as a task
  (P1, also touches M14); the review should cover the embedded-dep pin and the adapter trait
  as the seam for expansion.
- Non-embedded model pulls for a detected hardware tier are a wiring surface shared with M1.

## Related

- [spec/pipeline/inferencing](../../../../spec/pipeline/inferencing.md) — chains, adapters,
  MOE, budget (legacy depth, folds here).
- [spec/screen/settings-inference](../../../../spec/screen/settings-inference.md) — the
  Preferences pane.
- [prd/02-objectives.md](../../../02-objectives.md) §Cross-cutting ·
  [prd/01-vision.md §The core loop](../../../01-vision.md#the-core-loop).
- Companion ideas: docs/archive/ideas/28-inference-gateway.md, 20-local-inference.md.
- Crate: `gateway-embedded` @ `sensei-hq/gateway` 01d0ab2 (see memory project_p2_sweep_2026_07).