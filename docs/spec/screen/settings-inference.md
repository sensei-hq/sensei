# 推 · Preferences · Inference

**Segment:** 02 · First run & Preferences (Inference pane)
**Route:** `/preferences/inference`
**Source mockup:** [`lib/setup/inference-settings.jsx`](../../mockups/Sensei/lib/setup/inference-settings.jsx)
**App file:** `app/src/routes/preferences/inference/+page.svelte`

## Purpose

The Inference pane in Preferences is where the user tunes the
sensei gateway ([[pipeline/inferencing]]): which chains use which
models, whether to allow remote providers, current cost, model
recommendation based on hardware.

Kanji is 推 — *inference*.

## Data invariants

- `GET /api/preferences/inference` returns:
  ```json
  {
    "chains": [ { "name": "narration-cache", "primary": {…}, "fallback": [] }, … ],
    "installed_models": [ { "provider": "ollama", "model": "gemma4:12b", "size_gb": 8 }, … ],
    "recommended_tier": "balanced" | "advanced" | "lite" | "no-inference",
    "budget": { "daily_usd": number, "monthly_usd": number,
                 "consumed_today_usd": number, "consumed_month_usd": number },
    "circuit_state": { "chain": "narration-cache", "state": "closed|open|half-open", "next_probe_at": iso? }
  }
  ```
- Read/edit via `PUT /api/preferences/inference/…`.
- Hardware tier read via [[pipeline/bootstrap-resolution]].

## Signals shown

| Element | Value |
|---|---|
| Header | `推 · Inference` |
| Hardware chip | `RAM · N GB · tier X` from bootstrap |
| Model list | installed models with size + delete + set-default |
| Add model | pull a new Ollama model with progress bar |
| Chain editor | per chain: pick primary provider/model + fallback list + timeout |
| Budget strip | daily / monthly with consumed vs cap |
| Circuit-state chip | one per chain — closed (healthy) / half-open (probing) / open (cut) |
| Cost breakdown | last 30d per chain per provider |

## Done gate

- The list of installed Ollama models matches what `ollama
  list` returns.
- Pulling a new model streams progress to the UI.
- Changing a chain's primary takes effect on next call.
- Budget consumed values match `sensei.inference_calls`
  aggregate.
- Circuit-state chip flips to `open` when the actual breaker
  trips.
- Recommendation banner appears if the hardware tier changed
  ("your machine can now run `gemma4:27b` — pull it?").

## Wrong gate

- **Model list stale** — deleting a model in Ollama doesn't
  update the list.
- **Chain primary edit doesn't take effect** — a caller keeps
  hitting the old provider.
- **Budget consumed = 0 despite calls flowing.** Insert path
  broken in inferencing pipeline.
- **Circuit state shows `closed` after N failures.** Breaker
  logic bypassed.
- **Pulling a large model blocks the UI thread.** Should be
  background with progress.

## Related

- [[pipeline/inferencing]] — the runtime this configures
- [[pipeline/bootstrap-resolution]] — hardware tier detection
- [[screen/preferences]] — parent
- Source designs (external archive, not part of the spec tree):
  `docs/archive/ideas/28-inference-gateway.md` +
  `docs/archive/ideas/20-local-inference.md`
