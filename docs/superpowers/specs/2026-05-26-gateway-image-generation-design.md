---
name: Gateway Image Generation
description: Expose the gateway's existing image-generation capability through the daemon HTTP API, the sensei-mcp tool surface, and the setup wizard's Inference stage so AI assistants can generate imagery alongside code.
date: 2026-05-26
status: draft
---

# Gateway Image Generation — design

## Vocabulary

The gateway's existing type system distinguishes three things; this
spec uses them consistently:

- **Router** — a network endpoint with credentials (URL + API key +
  headers + timeout). Each `RouterConfig` is one router. Examples
  shipped today: `openai`, `anthropic`, `ollama`. A router *can*
  front more than one provider (Bedrock, OpenRouter) — the type
  system supports that, but no such adapter exists yet.
- **Provider** — the origin/owner of a model (`openai`, `anthropic`,
  `mistral`, `meta`, …). Lives on `ModelConfig.provider`. A single
  router can carry models from multiple providers (Bedrock case).
- **Model** — a specific named model (`gpt-4o`, `claude-sonnet-3.6`).
  Each `ModelConfig` has an id, an `api_model_id` for the upstream
  call, and a `provider`.

The wizard configures **routers** (where credentials apply); the
model registry maps each router to its providers and the models that
flow through it.

## Problem

The gateway crate already supports image generation end-to-end: the
`Capability::ImageGenerate` variant is defined, the OpenAI adapter
implements DALL-E 3 / gpt-image-1, and adapters exist for Stability,
Flux, Recraft, Fal, and Replicate. But nothing at the surface layer
uses it:

- `POST /api/gateway/infer` explicitly rejects any capability that is
  not `text_chat | text_complete | text_embed`.
- The `sensei-mcp` server exposes 20 tools — none touch image
  generation.
- The setup wizard's Inference stage is a placeholder; there is no way
  for the user to enter an OpenAI key.
- Adapters resolve API keys from environment variables only — no
  Keychain support.

The desired user flow: while working on a project the user asks their
AI assistant ("generate a chibi character for the Kata logo") and the
assistant calls a `generate_image` MCP tool that writes the image to
the project's assets folder. The same gateway path is reused by future
app-internal features (e.g. project icon generation).

## Scope

In scope for this design:

- Image generation only (`text → image`). Image editing
  (`image → image` with reference), audio, and video follow later.
- OpenAI as the default router for the first cut; the gateway's
  existing adapter selection picks others when configured.
- macOS Keychain for API key storage. Linux/Windows keyring support
  is a follow-up; the abstraction will accommodate it.

Out of scope:

- Image editing / variations.
- Image analysis (vision OCR).
- Audio / video modalities.
- A gallery / browser view of generated images inside the app.

## Architecture

Three new surfaces, all driven by the same underlying gateway
execution path:

```
                  ┌────────────────────────┐
                  │  Setup wizard (app)    │
                  │  Inference stage UI    │
                  │  - lists routers       │
                  │  - paste API key       │
                  └──────────┬─────────────┘
                             │ (1) router credential
                             ▼
                  ┌──────────────────────────┐
                  │  Daemon                  │
                  │  POST /api/gateway/      │
                  │       routers/{id}/key   │      ┌─────────────┐
                  │  - stores in Keychain    │◀────▶│  Keychain   │
                  └──────────┬───────────────┘      └─────────────┘
                             │ (2) ImageGenerateRequest
                             ▼
                  ┌────────────────────────┐
                  │  Gateway (existing)    │
                  │  Engine + selection +  │
                  │  router adapters       │
                  │  (openai/stability/…)  │
                  └──────────┬─────────────┘
                             │ (3) bytes / base64
                             ▼
                  ┌────────────────────────┐
                  │  Daemon                │
                  │  POST /api/gateway/    │
                  │       image/generate   │
                  │  - writes file         │
                  │  - returns path        │
                  └──────────┬─────────────┘
                             │ (4) {path, model, router, …}
                             ▼
                  ┌────────────────────────┐
                  │  sensei-mcp            │
                  │  generate_image tool   │
                  └────────────────────────┘
                             ▲
                             │ called by
                  ┌────────────────────────┐
                  │  AI assistant          │
                  │  (Claude/Cursor/etc.)  │
                  └────────────────────────┘
```

## Components

### 1. Keychain key store (`crates/gateway`)

New `KeyStore` abstraction so adapters can resolve keys from sources
other than env vars. Two impls in v1: `EnvKeyStore` (existing
behaviour) and `KeychainKeyStore` (new, macOS-only).

`RouterConfig` (existing — keys live at the router level, since
that's where the credential applies) gains a literal-key field:

```rust
pub struct RouterConfig {
    pub url:         String,
    pub api_key_env: Option<String>,    // unchanged — env var name
    pub api_key:     Option<String>,    // NEW — literal key (caller-resolved)
    pub enabled:     bool,
    pub timeout_ms:  Option<u64>,
    pub headers:     HashMap<String, String>,
}
```

Adapter call sites resolve in order: `config.api_key` → `api_key_env`
→ `None`. Keeps env-var configuration working unchanged.

Daemon owns key resolution: it reads the keychain entry by service
name (`com.sensei.gateway.router.<router_id>`), populates `api_key`
on the RouterConfig before passing to the gateway. The gateway itself
never talks to the Keychain.

The service-name namespace is keyed on router id rather than provider
because that's where the credential lives — a Bedrock router carries
AWS credentials, not per-provider keys.

### 2. Daemon endpoints

```
POST   /api/gateway/image/generate              text → image (capability call)

GET    /api/gateway/routers                     all known routers + configured flag
POST   /api/gateway/routers/{id}/key            set the router's API key (Keychain)
DELETE /api/gateway/routers/{id}/key            clear it
GET    /api/gateway/routers/{id}/providers      providers this router fronts
GET    /api/gateway/routers/{id}/models         models reachable through this router

GET    /api/gateway/models                      flat model list (each entry router-qualified)
```

`/api/gateway/routers/{id}/providers` returns `["openai"]` for the
OpenAI router today. Once a Bedrock adapter exists it will return
`["openai", "anthropic", "mistral", …]` without changing this
endpoint contract.

**Request shape** for `image/generate`:

```jsonc
{
  "prompt":        "chibi-style ninja character holding a katana, soft inks",
  "model":         "dall-e-3",          // optional — fully-qualified ok ("openai/dall-e-3")
  "router":        "openai",            // optional — pins which router to use
  "size":          "1024x1024",         // optional — router-specific
  "quality":       "hd",                // optional — router-specific
  "style":         "vivid",             // optional — router-specific
  "n":             1,                   // optional — number of images
  "output_path":   "/abs/path/img.png"  // optional — see below; MUST be absolute
}
```

`model` may be bare (`dall-e-3`) or fully-qualified
(`openai/dall-e-3`). When bare and `router` is unset, the gateway's
existing selection picks the highest-priority capable router. When
fully-qualified, the prefix selects the router and the suffix the
model.

**Response shape:**

```jsonc
{
  "ok":       true,
  "paths":    ["/Users/jerry/proj/static/chibi.png"],
  "model":    "dall-e-3",
  "provider": "openai",
  "router":   "openai",
  "usage":    { "image_count": 1, "cost_estimate_cents": 4 }
}
```

**Output path resolution** (the daemon's contract):

- `output_path` MUST be absolute. The MCP tool is responsible for
  resolving relative paths against the assistant's CWD (which the
  MCP subprocess inherits) before posting. This avoids the
  daemon-vs-MCP CWD ambiguity — the daemon runs as a service and its
  CWD is irrelevant to the user.
- If `output_path` is omitted, write to `~/.sensei/generated/<sha256
  of prompt + provider + model>.png`. Deterministic so a re-request
  with the same prompt overwrites the same cache entry.
- If `n > 1`, suffix each path with `-1.png`, `-2.png`, ...
- Daemon refuses paths outside `$HOME` unless the caller passes
  `allow_outside_home: true` — guard against an LLM hallucinating
  `/etc/something.png`.

Errors bubble up as standard HTTP 4xx/5xx with a JSON body matching
the existing gateway error shape.

### 3. MCP tool

One new tool in `crates/mcp/src/main.rs`:

```
generate_image(prompt, output_path?, size?, quality?, style?, n?, model?)
```

Required: `prompt`. All others optional. The tool POSTs to
`/api/gateway/image/generate` and returns the daemon's JSON verbatim
to the assistant. Description tuned for assistant discovery:

> Generate an image from a text prompt using the configured image
> provider (OpenAI by default). Returns the file path of the saved
> image. Use this when the user asks for visual assets — logos,
> illustrations, diagrams, character art, mockup imagery — that
> belong in the project.

### 4. Setup wizard — Inference stage

The Inference stage (currently a 7-line placeholder) becomes the
**router** configuration page. It loads
`GET /api/gateway/routers` and renders one card per known router
showing:

- router name + logo / kanji
- the providers the router fronts (chips; one for single-provider
  routers, many for Bedrock/OpenRouter)
- which capabilities it supports (chat, embed, image_generate, etc.)
- the configured key status (`Configured ✓` or `Not configured`)
- a password input + Save button to set / update the key
- a Clear link to remove the key

Behaviour mirrors the Assistants stage:

- `Continue & Configure` button persists any pasted keys to the
  Keychain (one POST per router) before advancing.
- A `Skip` link advances without setting any keys — the user can
  configure later from Settings.
- Per-card state machine: `idle | saving | done | failed`.

Routers list (initial set, all already in the gateway crate):

| id          | name        | providers     | capabilities                          |
|-------------|-------------|---------------|---------------------------------------|
| openai      | OpenAI      | openai        | text_chat, text_embed, image_generate |
| anthropic   | Anthropic   | anthropic     | text_chat                             |
| ollama      | Ollama      | (local)       | text_chat, text_embed                 |
| stability   | Stability AI| stability     | image_generate                        |
| fal         | Fal         | fal           | image_generate, video_generate        |
| replicate   | Replicate   | replicate     | image_generate                        |

Ollama needs no key (it's local); the card hides the input. Future
multi-provider routers (Bedrock, OpenRouter) slot into this same
table with multi-element `providers` columns and (for Bedrock)
non-API-key credentials handled by a router-specific credential
component.

### 5. Inference slice in wizardState

```ts
export interface RouterEntry {
  id: string;
  name: string;
  providers: string[];        // ["openai"] today; ["openai","anthropic",…] for Bedrock
  capabilities: string[];
  configured: boolean;        // daemon-reported truth
  needsKey: boolean;          // false for local routers (Ollama)
  // local edit state
  draftKey: string;           // never sent until commit
  saveState: 'idle' | 'saving' | 'done' | 'failed';
  saveError: string;
}

export interface InferenceSlice {
  routers: RouterEntry[];
}
```

Commit handler iterates routers where `draftKey` is non-empty, POSTs
each key, updates `configured` and `saveState`. Failures don't block
navigation (skip with warning) — router keys are non-essential to
setup completion.

## Data flow

**Generate request (happy path):**

1. AI assistant calls MCP tool `generate_image({prompt, output_path})`.
2. MCP tool resolves any relative `output_path` to absolute using
   its own (assistant-inherited) CWD.
3. MCP server POSTs `/api/gateway/image/generate` to the local
   daemon.
4. Daemon picks the router (from `router` arg, fully-qualified
   `model`, or gateway selection), reads that router's API key from
   Keychain (errors 400 if not configured), populates `api_key` on
   the cached RouterConfig.
5. Daemon constructs an `InferenceRequest` with
   `Capability::ImageGenerate` and dispatches through the gateway
   engine.
6. Adapter hits the upstream (e.g.
   `https://api.openai.com/v1/images/generations`) and returns
   base64-encoded image bytes.
7. Daemon decodes the bytes, writes to `output_path` (resolved per
   the contract above), returns `{ok, paths, model, provider,
   router, ...}`.
8. MCP tool returns the JSON to the assistant.

**Router config (happy path):**

1. User pastes `sk-...` in the Inference stage's OpenAI router card.
2. Clicks `Continue & Configure`.
3. App POSTs `/api/gateway/routers/openai/key` with `{key: "sk-..."}`.
4. Daemon writes to Keychain at service
   `com.sensei.gateway.router.openai`.
5. Daemon returns `{ok: true, configured: true}`.
6. App updates wizardState, advances to next stage.

## Error handling

- **Key missing** → HTTP 400 with body
  `{error: "openai router key not configured", router: "openai"}`.
  MCP tool surfaces this to the assistant so it can prompt the user.
- **Router/upstream error (rate limit, invalid key, etc.)** → HTTP
  502 with the upstream message; gateway's existing error path
  handles it.
- **File write failure** → HTTP 500 with the OS error; daemon
  attempts no retries.
- **Path outside `$HOME` without `allow_outside_home`** → HTTP 400
  with `{error: "output path outside $HOME blocked"}`.
- **Keychain failure (Keychain locked, denied)** → HTTP 500 with
  `{error: "keychain access denied"}`.
- **Unknown router** → HTTP 404.

## Testing

**Unit (cargo)**

- `gateway::adapters::openai` already has image-generate tests with
  wiremock — extend them to cover the new `api_key` resolution
  precedence.
- New `keychain_store.rs` module — unit-tested on macOS only via
  `#[cfg(target_os = "macos")]`; CI gate.
- New daemon `image_generate` handler — tests with a mocked gateway
  asserting path resolution, fallback cache dir, multi-image suffix
  rule.

**Unit (vitest)**

- `mapInferenceProviders` loader test.
- `wizardState.inference` slice — saveState transitions, draftKey
  isolation, commit handler error semantics.

**E2E (Playwright)**

- New `e2e/tests/inference-stage.spec.ts`: stub the daemon's
  `/api/gateway/routers` and `/api/gateway/routers/{id}/key`
  endpoints, drive the wizard through the Inference stage with one
  paste, assert the POST fires with the expected payload, assert
  navigation to Assignments.

**Manual**

- Set a real OpenAI key in the wizard, call the MCP tool from a
  Claude Code session, verify the image lands at the requested path.
  Drop a chibi reference in `docs/` as the documented end-to-end
  proof.

## Open questions

- **Linux / Windows Keychain** — secret-service / Credential Manager
  support is needed before the app can ship beyond macOS. Tracking
  separately; the daemon's KeychainKeyStore returns a platform error
  on non-macOS for now and the wizard surfaces a fallback "set via
  env var" path.
- **Per-project router overrides** — some teams want different keys
  per project (e.g. billing isolation). Out of scope for v1; the
  Keychain namespace (`com.sensei.gateway.router.<id>`) leaves room
  for `router.<id>.<project_id>` later.
- **Image inlining for chat models** — once upstream image responses
  return cleanly, we could optionally embed the image in the
  assistant's chat as base64 instead of a path. Defer until UX
  feedback says paths feel awkward.

## Follow-ups (not in this plan)

- **Multi-provider router adapters** — Bedrock, OpenRouter,
  Together. The type system already supports them via
  `ModelConfig.provider`; each adapter needs request-formatting
  dispatch (per-model wire format) and credential shape beyond a
  single API key (AWS sig v4, etc.). Plan separately once at least
  one user asks.
- Image edit (`image → image`) — adds reference-image upload to the
  MCP tool and `multipart/form-data` to the daemon endpoint.
- Audio (STT/TTS) and video — symmetric endpoint pattern, separate
  spec.
- Router-keys page in `/settings` (outside the wizard) once the
  Inference stage exists.

## Verified

End-to-end live test against a real OpenAI account on 2026-05-27:

1. `POST /api/gateway/routers/openai/key` stored the key in macOS
   Keychain at `com.sensei.gateway.router.openai`. `GET /api/gateway/routers`
   returned `configured: true`.
2. Daemon restart preserved the key (Keychain-backed, not in-process).
3. `POST /api/gateway/image/generate` with `{model: "openai/gpt-image-1",
   prompt: "a tiny smiling onigiri ... chibi style ...", size: "1024x1024",
   quality: "high", output_path: "/tmp/sensei-chibi.png"}` returned
   `{ok: true, paths: ["/tmp/sensei-chibi.png"], model: "gpt-image-1",
   router: "openai"}` and wrote a valid 2 MB PNG (1024×1024 RGB) to disk.

Gaps discovered and resolved during verification:

- `init_gateway()` started with an empty `GatewayConfig::default()`, so
  even with adapters and Keychain keys present, all requests failed
  with `NotConfigured`. Replaced with `baseline_production_config()`
  containing the openai/anthropic/ollama routers, `dall-e-3` /
  `gpt-image-1` / `gpt-4o-mini` / `claude-sonnet` models, and
  `image_generate` / `text_chat` chains. This is the one-place edit
  when adding new shipped routers/models.
- External-provider adapters (OpenAI, Anthropic, Grok) were only
  registered when their `*_API_KEY` env var was set at daemon startup.
  With Keychain support, this gated valid configurations behind an
  irrelevant env-var check. Adapters now register unconditionally;
  `resolve_api_key` (Task 2) picks the literal Keychain-derived key
  before falling back to the env var. Missing keys still fail clearly
  at request time, not registration time.
- Router URLs in the baseline config were initially `…/v1`, but every
  OpenAI/Anthropic adapter call appends `/v1/...` itself. Fixed to
  bare hostnames (`https://api.openai.com`,
  `https://api.anthropic.com`).
- `GatewayError::AllAttemptsFailed` previously surfaced only an
  attempt count. Extended to include a joined `errors` string built
  from each failed `Attempt.error`, so handlers and clients can see
  the underlying provider response (e.g., "Invalid value: 'standard'.
  Supported values are: 'low', 'medium', 'high', 'auto'." for the
  `gpt-image-1` quality enum).
