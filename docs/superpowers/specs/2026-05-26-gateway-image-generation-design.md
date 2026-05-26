---
name: Gateway Image Generation
description: Expose the gateway's existing image-generation capability through the daemon HTTP API, the sensei-mcp tool surface, and the setup wizard's Inference stage so AI assistants can generate imagery alongside code.
date: 2026-05-26
status: draft
---

# Gateway Image Generation — design

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
- OpenAI as the default provider; the gateway's existing adapter
  selection picks others when configured.
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
                  │  - lists providers     │
                  │  - paste API key       │
                  └──────────┬─────────────┘
                             │ (1) provider config
                             ▼
                  ┌──────────────────────────┐
                  │  Daemon                  │
                  │  POST /api/gateway/      │
                  │       providers/{id}/key │      ┌─────────────┐
                  │  - stores in Keychain    │◀────▶│  Keychain   │
                  └──────────┬───────────────┘      └─────────────┘
                             │ (2) ImageGenerateRequest
                             ▼
                  ┌────────────────────────┐
                  │  Gateway (existing)    │
                  │  Engine + selection +  │
                  │  OpenAI/Stability/etc. │
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
                             │ (4) {path, model, ...}
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

The gateway's `RouterConfig` gains a third resolution path:

```rust
pub struct RouterConfig {
    pub provider: String,
    pub api_key_env: Option<String>,    // unchanged — env var name
    pub api_key:     Option<String>,    // NEW — literal key (caller-resolved)
    // ...
}
```

Adapter call sites resolve in order: `config.api_key` → `api_key_env`
→ `None`. Keeps env-var configuration working unchanged.

Daemon owns key resolution: it reads the keychain entry by service
name (`com.sensei.gateway.<provider>`), populates `api_key` on the
RouterConfig before passing to the gateway. The gateway itself never
talks to the Keychain.

### 2. Daemon endpoints

```
POST /api/gateway/image/generate     - text → image
GET  /api/gateway/providers          - list known providers + configured flag
POST /api/gateway/providers/{id}/key - set the API key for a provider
DELETE /api/gateway/providers/{id}/key - clear it
```

**Request shape** for `image/generate`:

```jsonc
{
  "prompt":        "chibi-style ninja character holding a katana, soft inks",
  "model":         "dall-e-3",          // optional — defaults to provider default
  "size":          "1024x1024",         // optional — provider-specific
  "quality":       "hd",                // optional — provider-specific
  "style":         "vivid",             // optional — provider-specific
  "n":             1,                   // optional — number of images
  "output_path":   "./static/chibi.png" // optional — see below
}
```

**Response shape:**

```jsonc
{
  "ok":      true,
  "paths":   ["/Users/jerry/proj/static/chibi.png"],
  "model":   "dall-e-3",
  "provider":"openai",
  "usage":   { "image_count": 1, "cost_estimate_cents": 4 }
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
provider-keys configuration page. It loads
`GET /api/gateway/providers` and renders one card per known provider
showing:

- provider name + logo / kanji
- which capabilities it supports (chat, embed, image generate, etc.)
- the configured key status (`Configured ✓` or `Not configured`)
- a password input + Save button to set / update the key
- a Clear link to remove the key

Behaviour mirrors the Assistants stage:

- `Continue & Configure` button persists any pasted keys to the
  Keychain (one POST per provider) before advancing.
- A `Skip` link advances without setting any keys — the user can
  configure later from Settings.
- Per-card state machine: `idle | saving | done | failed`.

Providers list (initial set, all already in the gateway crate):

| id          | name        | capabilities                          |
|-------------|-------------|---------------------------------------|
| openai      | OpenAI      | text_chat, text_embed, image_generate |
| anthropic   | Anthropic   | text_chat                             |
| ollama      | Ollama      | text_chat, text_embed (local)         |
| stability   | Stability AI| image_generate                        |
| fal         | Fal         | image_generate, video_generate        |
| replicate   | Replicate   | image_generate                        |

Ollama needs no key (it's local); the card hides the input.

### 5. Inference slice in wizardState

```ts
export interface ProviderConfig {
  id: string;
  name: string;
  capabilities: string[];
  configured: boolean;        // daemon-reported truth
  // local edit state
  draftKey: string;           // never sent until commit
  saveState: 'idle' | 'saving' | 'done' | 'failed';
  saveError: string;
}

export interface InferenceSlice {
  providers: ProviderConfig[];
}
```

Commit handler iterates providers where `draftKey` is non-empty,
POSTs each key, updates `configured` and `saveState`. Failures don't
block navigation (skip with warning) — provider keys are
non-essential to setup completion.

## Data flow

**Generate request (happy path):**

1. AI assistant calls MCP tool `generate_image({prompt, output_path})`.
2. MCP server POSTs `/api/gateway/image/generate` to the local
   daemon.
3. Daemon resolves the provider's API key from Keychain (errors out
   400 if not configured).
4. Daemon constructs an `InferenceRequest` with
   `Capability::ImageGenerate` and the resolved key on the
   RouterConfig.
5. Gateway engine selects the OpenAI adapter, hits
   `https://api.openai.com/v1/images/generations`, returns the
   base64-encoded image bytes.
6. Daemon decodes the bytes, writes to `output_path` (resolved per
   the contract above), returns `{ok, paths, ...}`.
7. MCP tool returns the JSON to the assistant.

**Provider config (happy path):**

1. User pastes `sk-...` in the Inference stage's OpenAI card.
2. Clicks `Continue & Configure`.
3. App POSTs `/api/gateway/providers/openai/key` with `{key: "sk-..."}`.
4. Daemon writes to Keychain at service
   `com.sensei.gateway.openai`.
5. Daemon returns `{ok: true, configured: true}`.
6. App updates wizardState, advances to next stage.

## Error handling

- **Key missing** → HTTP 400 with body
  `{error: "openai api key not configured", provider: "openai"}`.
  MCP tool surfaces this to the assistant so it can prompt the user.
- **Provider error (rate limit, invalid key, etc.)** → HTTP 502 with
  the upstream message; gateway's existing error path handles it.
- **File write failure** → HTTP 500 with the OS error; daemon
  attempts no retries.
- **Keychain failure (Keychain locked, denied)** → HTTP 500 with
  `{error: "keychain access denied"}`.
- **Unknown provider** → HTTP 404.

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
  `/api/gateway/providers` and `/api/gateway/providers/{id}/key`
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
- **Per-project provider overrides** — some teams want different keys
  per project (e.g. billing isolation). Out of scope for v1; the
  config key namespace (`com.sensei.gateway.<provider>`) leaves room
  for `<provider>.<project_id>` later.
- **Image inlining for chat models** — once OpenAI's image responses
  return cleanly, we could optionally embed the image in the
  assistant's chat as base64 instead of a path. Defer until UX
  feedback says paths feel awkward.

## Follow-ups (not in this plan)

- Image edit (`image → image`) — adds reference-image upload to the
  MCP tool and `multipart/form-data` to the daemon endpoint.
- Audio (STT/TTS) and video — symmetric endpoint pattern, separate
  spec.
- Provider-keys page in `/settings` (outside the wizard) once the
  Inference stage exists.
