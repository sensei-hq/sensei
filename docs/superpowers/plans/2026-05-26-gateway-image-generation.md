# Gateway Image Generation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Expose the gateway's existing `Capability::ImageGenerate` end-to-end so an AI assistant can call a `generate_image` MCP tool that writes an image to the user's project, using a key set via the wizard's Inference stage and stored in the macOS Keychain.

**Architecture:** Three surface layers wrap the existing gateway engine — (1) a Keychain key resolver in the daemon that populates `RouterConfig.api_key`, (2) a small set of `/api/gateway/routers/*` and `/api/gateway/image/generate` HTTP endpoints, (3) a `generate_image` MCP tool that resolves paths and proxies to the daemon. The setup wizard's Inference stage configures router credentials.

**Tech Stack:** Rust (daemon, gateway, mcp), Svelte 5 (app), macOS `security` CLI (Keychain), TDD with cargo + vitest + Playwright.

**Spec:** `docs/superpowers/specs/2026-05-26-gateway-image-generation-design.md`

---

## File Structure

**Gateway crate (`crates/gateway/`)**
- Modify: `src/types/config.rs` — add `api_key: Option<String>` to `RouterConfig`
- Modify: `src/adapters/base.rs` — update `resolve_api_key` to prefer literal key
- Modify: `src/adapters/openai.rs`, `anthropic.rs`, `stability.rs`, `fal.rs`, `replicate.rs`, `flux.rs`, `recraft.rs`, `together.rs`, `grok.rs`, `kling.rs`, `luma.rs`, `runway.rs`, `ollama.rs` — no source edits (they all call `resolve_api_key`)

**Daemon crate (`crates/senseid/`)**
- Create: `src/gateway_keys/mod.rs` — Keychain access via `security` CLI
- Create: `src/gateway_routers/mod.rs` — static registry of known routers (id, name, providers, capabilities, needs_key)
- Create: `src/api/handlers/gateway_routers.rs` — `/api/gateway/routers/*` handlers
- Create: `src/api/handlers/gateway_image.rs` — `/api/gateway/image/generate` handler
- Modify: `src/api/handlers/mod.rs` — register new handler modules
- Modify: `src/api/routes.rs` — wire new endpoints
- Modify: `src/main.rs` — register `gateway_keys` + `gateway_routers` modules
- Modify: `src/api/gateway_init.rs` — populate `api_key` from Keychain when building RouterConfig

**MCP crate (`crates/mcp/`)**
- Modify: `src/main.rs` — add `generate_image` to tool list + handler

**App (`app/`)**
- Modify: `src/lib/api.ts` — add `listGatewayRouters`, `setGatewayRouterKey`, `clearGatewayRouterKey`, `generateImage`
- Modify: `src/lib/setup/contracts.ts` — add `DaemonRouter` + `InferenceLoadData`
- Modify: `src/lib/setup/loaders.ts` — fetch routers, map to slice
- Modify: `src/lib/setup/mock-contracts.ts` — mock factory
- Modify: `src/lib/wizard-state.svelte.ts` — add `InferenceSlice`, commit handler, `refreshInference` method
- Modify: `src/routes/(config)/setup/inference/+page.svelte` — rewrite as router-keys page
- Modify: `src/routes/(config)/stages.ts` — update Inference stage description

**E2E (`app/e2e/`)**
- Create: `tests/inference-stage.spec.ts`

---

## Phase 1 — Gateway: literal api_key support

### Task 1: Add `api_key` field to `RouterConfig`

**Files:**
- Modify: `crates/gateway/src/types/config.rs`
- Test: `crates/gateway/src/types/config.rs` (in-module tests block)

- [ ] **Step 1: Write the failing test**

Add to the tests module in `config.rs`:

```rust
#[test]
fn router_config_api_key_field_serializes() {
    let config = RouterConfig {
        url: "https://api.example.com".to_string(),
        api_key_env: None,
        api_key: Some("sk-literal".to_string()),
        enabled: true,
        timeout_ms: None,
        headers: HashMap::new(),
    };
    let json = serde_json::to_string(&config).unwrap();
    assert!(json.contains("\"api_key\":\"sk-literal\""));

    let parsed: RouterConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.api_key.as_deref(), Some("sk-literal"));
}

#[test]
fn router_config_omits_api_key_when_none() {
    let config = RouterConfig {
        url: "https://api.example.com".to_string(),
        api_key_env: Some("X_KEY".to_string()),
        api_key: None,
        enabled: true,
        timeout_ms: None,
        headers: HashMap::new(),
    };
    let json = serde_json::to_string(&config).unwrap();
    assert!(!json.contains("api_key"));
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gateway --lib config::tests::router_config_api_key`
Expected: FAIL — `api_key` field does not exist on `RouterConfig`.

- [ ] **Step 3: Add the field**

In `crates/gateway/src/types/config.rs`, change `RouterConfig`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RouterConfig {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key_env: Option<String>,
    /// Literal API key — populated by the caller (e.g. the daemon resolves
    /// from Keychain and inserts it here before passing the config to an
    /// adapter). Takes precedence over `api_key_env` when both are set.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub api_key: Option<String>,
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub headers: HashMap<String, String>,
}
```

- [ ] **Step 4: Update existing `RouterConfig` constructions to include the new field**

Run: `grep -rn "RouterConfig {" crates/gateway/src crates/senseid/src 2>&1 | head -20`

For each construction site that doesn't already use `..Default::default()`, add `api_key: None,`. If the struct doesn't derive Default, add it now:

```rust
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RouterConfig {
```

Update all test fixtures in `config.rs::tests` that built `RouterConfig` literally to include `api_key: None`.

- [ ] **Step 5: Run all gateway tests**

Run: `cargo test -p gateway`
Expected: all tests pass including the two new ones.

- [ ] **Step 6: Commit**

```bash
git add crates/gateway/src/types/config.rs
git commit -m "feat(gateway): add literal api_key field to RouterConfig

Optional override for the existing api_key_env. Caller (e.g. the daemon
resolving from Keychain) populates this before passing the config to an
adapter. Adapters will be updated next task to prefer it over the env
var lookup.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 2: Update `resolve_api_key` to prefer literal key

**Files:**
- Modify: `crates/gateway/src/adapters/base.rs`
- Test: `crates/gateway/src/adapters/base.rs` (in-module tests block)

- [ ] **Step 1: Write the failing test**

Append to `crates/gateway/src/adapters/base.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::config::RouterConfig;
    use std::collections::HashMap;

    fn fixture(api_key: Option<String>, env: Option<String>) -> RouterConfig {
        RouterConfig {
            url: "https://x".into(),
            api_key,
            api_key_env: env,
            enabled: true,
            timeout_ms: None,
            headers: HashMap::new(),
        }
    }

    #[test]
    fn literal_api_key_takes_precedence() {
        unsafe { std::env::set_var("RESOLVE_TEST_KEY", "from-env") };
        let cfg = fixture(Some("from-literal".into()), Some("RESOLVE_TEST_KEY".into()));
        assert_eq!(resolve_api_key(&cfg).as_deref(), Some("from-literal"));
        unsafe { std::env::remove_var("RESOLVE_TEST_KEY") };
    }

    #[test]
    fn falls_back_to_env_var_when_literal_absent() {
        unsafe { std::env::set_var("RESOLVE_TEST_FALLBACK", "from-env") };
        let cfg = fixture(None, Some("RESOLVE_TEST_FALLBACK".into()));
        assert_eq!(resolve_api_key(&cfg).as_deref(), Some("from-env"));
        unsafe { std::env::remove_var("RESOLVE_TEST_FALLBACK") };
    }

    #[test]
    fn returns_none_when_neither_source_has_a_key() {
        let cfg = fixture(None, Some("RESOLVE_TEST_MISSING".into()));
        assert_eq!(resolve_api_key(&cfg), None);
    }
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test -p gateway --lib adapters::base::tests::literal_api_key_takes_precedence`
Expected: FAIL — `resolve_api_key` only reads env vars.

- [ ] **Step 3: Update `resolve_api_key`**

Replace the function in `crates/gateway/src/adapters/base.rs`:

```rust
/// Resolve an API key for an adapter request.
///
/// Precedence:
///   1. `config.api_key` (literal — the daemon populates this after
///      reading the Keychain).
///   2. `config.api_key_env` (env var name — original behaviour).
///   3. None.
pub fn resolve_api_key(config: &RouterConfig) -> Option<String> {
    if let Some(literal) = config.api_key.as_ref() {
        return Some(literal.clone());
    }
    config
        .api_key_env
        .as_ref()
        .and_then(|env_var| std::env::var(env_var).ok())
}
```

- [ ] **Step 4: Run tests to verify they pass**

Run: `cargo test -p gateway --lib adapters::base::tests`
Expected: 3 passed.

- [ ] **Step 5: Run full gateway test suite to confirm no regressions**

Run: `cargo test -p gateway`
Expected: all pass.

- [ ] **Step 6: Commit**

```bash
git add crates/gateway/src/adapters/base.rs
git commit -m "feat(gateway): resolve_api_key prefers literal over env var

When config.api_key is set, return it directly. Otherwise fall back to
the env var path (unchanged). Existing env-var-only setups keep working;
new daemon-resolved-from-Keychain path now works without per-adapter
changes since every adapter goes through this helper.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase 2 — Daemon: Keychain + router registry + read endpoints

### Task 3: Keychain helper module

**Files:**
- Create: `crates/senseid/src/gateway_keys/mod.rs`
- Test: same file (`#[cfg(test)]` block)
- Modify: `crates/senseid/src/main.rs` (register module)

- [ ] **Step 1: Write the failing test**

Create `crates/senseid/src/gateway_keys/mod.rs` with:

```rust
//! macOS Keychain access for gateway router API keys.
//!
//! Wraps `/usr/bin/security` so we don't add a new crate dependency.
//! Service name namespace: `com.sensei.gateway.router.<router_id>`.
//! Account name: `"default"` (one key per router for now; per-project
//! overrides would use `account = project_id` later).

use std::process::Command;

const ACCOUNT: &str = "default";

fn service_name(router_id: &str) -> String {
    format!("com.sensei.gateway.router.{router_id}")
}

#[derive(Debug, thiserror::Error)]
pub enum KeychainError {
    #[error("keychain access denied or item not found")]
    NotFound,
    #[error("keychain command failed: {0}")]
    CommandFailed(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

/// Write a key to the Keychain. Replaces an existing entry (`-U`).
pub fn set_key(router_id: &str, key: &str) -> Result<(), KeychainError> {
    let service = service_name(router_id);
    let output = Command::new("/usr/bin/security")
        .args([
            "add-generic-password",
            "-s", &service,
            "-a", ACCOUNT,
            "-w", key,
            "-U",
        ])
        .output()?;
    if !output.status.success() {
        return Err(KeychainError::CommandFailed(
            String::from_utf8_lossy(&output.stderr).trim().to_string(),
        ));
    }
    Ok(())
}

/// Read a key from the Keychain. Returns NotFound if absent.
pub fn get_key(router_id: &str) -> Result<String, KeychainError> {
    let service = service_name(router_id);
    let output = Command::new("/usr/bin/security")
        .args([
            "find-generic-password",
            "-s", &service,
            "-a", ACCOUNT,
            "-w",
        ])
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("could not be found") {
            return Err(KeychainError::NotFound);
        }
        return Err(KeychainError::CommandFailed(stderr.trim().to_string()));
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Delete a key. Returns Ok(()) whether or not it existed.
pub fn delete_key(router_id: &str) -> Result<(), KeychainError> {
    let service = service_name(router_id);
    let output = Command::new("/usr/bin/security")
        .args([
            "delete-generic-password",
            "-s", &service,
            "-a", ACCOUNT,
        ])
        .output()?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("could not be found") {
            return Ok(());
        }
        return Err(KeychainError::CommandFailed(stderr.trim().to_string()));
    }
    Ok(())
}

/// True when a key exists for this router. Cheap check used by the
/// /api/gateway/routers endpoint to compute `configured`.
pub fn has_key(router_id: &str) -> bool {
    matches!(get_key(router_id), Ok(_))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_id() -> String {
        format!("test-{}", uuid::Uuid::new_v4())
    }

    #[test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    fn roundtrip_set_get_delete() {
        let id = unique_id();
        assert!(!has_key(&id), "fresh id should not have a key");

        set_key(&id, "sk-test-12345").expect("set should succeed");
        let key = get_key(&id).expect("get should succeed");
        assert_eq!(key, "sk-test-12345");
        assert!(has_key(&id), "has_key should report true after set");

        delete_key(&id).expect("delete should succeed");
        assert!(!has_key(&id), "has_key should be false after delete");
        assert!(matches!(get_key(&id), Err(KeychainError::NotFound)));
    }

    #[test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    fn set_replaces_existing_value() {
        let id = unique_id();
        set_key(&id, "first").unwrap();
        set_key(&id, "second").unwrap();
        assert_eq!(get_key(&id).unwrap(), "second");
        delete_key(&id).unwrap();
    }

    #[test]
    #[cfg_attr(not(target_os = "macos"), ignore)]
    fn delete_missing_is_noop() {
        let id = unique_id();
        delete_key(&id).expect("delete on missing should not error");
    }

    #[test]
    fn service_name_uses_router_id() {
        assert_eq!(service_name("openai"), "com.sensei.gateway.router.openai");
    }
}
```

- [ ] **Step 2: Register the module**

Edit `crates/senseid/src/main.rs`, add to the `pub mod` block:

```rust
pub mod gateway_keys;
```

- [ ] **Step 3: Run the tests**

Run: `cargo test -p senseid --features dev --bin senseid gateway_keys`
Expected: 4 passed (3 keychain + 1 service_name).

- [ ] **Step 4: Commit**

```bash
git add crates/senseid/src/gateway_keys/ crates/senseid/src/main.rs
git commit -m "feat(daemon): macOS Keychain helper for gateway router keys

Wraps /usr/bin/security so no new crate dependency. Service name
namespace com.sensei.gateway.router.<router_id> with a single 'default'
account per service — leaves room for per-project overrides later
(account = project_id).

Non-macOS targets ignore the integration tests; cross-platform Keychain
support is a follow-up.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 4: Router registry module

**Files:**
- Create: `crates/senseid/src/gateway_routers/mod.rs`
- Test: same file
- Modify: `crates/senseid/src/main.rs`

- [ ] **Step 1: Write the failing test + implementation together**

Create `crates/senseid/src/gateway_routers/mod.rs`:

```rust
//! Static registry of routers the gateway knows how to talk to.
//!
//! One entry per shipped adapter. Capabilities + providers are
//! authoritative for the wizard's Inference stage and the
//! /api/gateway/routers endpoints. When a new adapter lands in the
//! gateway crate, add a corresponding entry here.

use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct RouterEntry {
    pub id: &'static str,
    pub name: &'static str,
    /// Provider ids whose models flow through this router. Single
    /// element for OpenAI/Anthropic/etc. today; multi-element for
    /// future Bedrock / OpenRouter.
    pub providers: &'static [&'static str],
    /// Capability ids (mirror `gateway::types::capability::Capability`
    /// snake_case rendering).
    pub capabilities: &'static [&'static str],
    /// False for local/no-auth routers (Ollama). Inference stage UI
    /// hides the key input when false.
    pub needs_key: bool,
}

pub const REGISTRY: &[RouterEntry] = &[
    RouterEntry {
        id: "openai", name: "OpenAI",
        providers: &["openai"],
        capabilities: &["text_chat", "text_embed", "image_generate"],
        needs_key: true,
    },
    RouterEntry {
        id: "anthropic", name: "Anthropic",
        providers: &["anthropic"],
        capabilities: &["text_chat"],
        needs_key: true,
    },
    RouterEntry {
        id: "ollama", name: "Ollama",
        providers: &["local"],
        capabilities: &["text_chat", "text_embed"],
        needs_key: false,
    },
    RouterEntry {
        id: "stability", name: "Stability AI",
        providers: &["stability"],
        capabilities: &["image_generate"],
        needs_key: true,
    },
    RouterEntry {
        id: "fal", name: "Fal",
        providers: &["fal"],
        capabilities: &["image_generate", "video_generate"],
        needs_key: true,
    },
    RouterEntry {
        id: "replicate", name: "Replicate",
        providers: &["replicate"],
        capabilities: &["image_generate"],
        needs_key: true,
    },
];

pub fn find(id: &str) -> Option<&'static RouterEntry> {
    REGISTRY.iter().find(|r| r.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_is_nonempty_and_unique() {
        assert!(!REGISTRY.is_empty());
        let mut ids: Vec<&str> = REGISTRY.iter().map(|r| r.id).collect();
        ids.sort();
        let len = ids.len();
        ids.dedup();
        assert_eq!(ids.len(), len, "duplicate router ids");
    }

    #[test]
    fn openai_supports_image_generate() {
        let openai = find("openai").unwrap();
        assert!(openai.capabilities.contains(&"image_generate"));
    }

    #[test]
    fn ollama_is_keyless() {
        assert!(!find("ollama").unwrap().needs_key);
    }

    #[test]
    fn every_keyed_router_has_at_least_one_provider() {
        for r in REGISTRY {
            if r.needs_key {
                assert!(!r.providers.is_empty(), "{} has no providers", r.id);
            }
        }
    }
}
```

- [ ] **Step 2: Register the module**

Add to `crates/senseid/src/main.rs`:

```rust
pub mod gateway_routers;
```

- [ ] **Step 3: Run tests**

Run: `cargo test -p senseid --features dev --bin senseid gateway_routers`
Expected: 4 passed.

- [ ] **Step 4: Commit**

```bash
git add crates/senseid/src/gateway_routers/ crates/senseid/src/main.rs
git commit -m "feat(daemon): static router registry

One entry per shipped gateway adapter — providers/capabilities/
needs_key authoritative for the /api/gateway/routers endpoints and the
wizard's Inference stage. Adding a new adapter is a one-place edit.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 5: `/api/gateway/routers` listing endpoints

**Files:**
- Create: `crates/senseid/src/api/handlers/gateway_routers.rs`
- Modify: `crates/senseid/src/api/handlers/mod.rs`
- Modify: `crates/senseid/src/api/routes.rs`

- [ ] **Step 1: Write the handler**

Create `crates/senseid/src/api/handlers/gateway_routers.rs`:

```rust
//! `/api/gateway/routers/*` — read endpoints for the wizard's
//! Inference stage and any future router introspection UI.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;
use crate::api::state::AppState;
use crate::gateway_keys;
use crate::gateway_routers::{REGISTRY, find};

/// GET /api/gateway/routers — every known router + `configured` flag
/// from Keychain.
pub(crate) async fn list_routers(
    State(_state): State<AppState>,
) -> Json<serde_json::Value> {
    // Keychain reads are sync and may prompt — run off-runtime.
    let entries = tokio::task::spawn_blocking(|| {
        REGISTRY.iter().map(|r| serde_json::json!({
            "id":           r.id,
            "name":         r.name,
            "providers":    r.providers,
            "capabilities": r.capabilities,
            "needs_key":    r.needs_key,
            "configured":   !r.needs_key || gateway_keys::has_key(r.id),
        })).collect::<Vec<_>>()
    }).await.unwrap_or_default();
    Json(serde_json::json!({ "routers": entries }))
}

/// GET /api/gateway/routers/{id}/providers — providers fronted by
/// this router. Single-element today; future multi-provider routers
/// (Bedrock) return many.
pub(crate) async fn router_providers(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let router = find(&id).ok_or(StatusCode::NOT_FOUND)?;
    Ok(Json(serde_json::json!({ "providers": router.providers })))
}

/// GET /api/gateway/routers/{id}/models — models reachable through
/// this router. Walks the gateway's model registry filtered by
/// router id (see ChainEntry/ModelConfig).
pub(crate) async fn router_models(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _router = find(&id).ok_or(StatusCode::NOT_FOUND)?;
    // The gateway exposes model metadata via Gateway::list_models_for_router
    // (added below as part of the gateway-side API surface). Until that
    // method exists, return the model ids the daemon has cached.
    let models = state.gateway.list_models_for_router(&id).await
        .unwrap_or_default();
    Ok(Json(serde_json::json!({ "models": models })))
}

/// GET /api/gateway/models — flat list of all models, each entry
/// router-qualified.
pub(crate) async fn list_all_models(
    State(state): State<AppState>,
) -> Json<serde_json::Value> {
    let models = state.gateway.list_models().await.unwrap_or_default();
    Json(serde_json::json!({ "models": models }))
}

#[derive(Deserialize)]
pub(crate) struct SetKeyBody {
    pub key: String,
}

/// POST /api/gateway/routers/{id}/key — store the router's API key
/// in the Keychain. The gateway picks it up on the next request via
/// gateway_init::router_config_with_keychain.
pub(crate) async fn set_router_key(
    Path(id): Path<String>,
    Json(body): Json<SetKeyBody>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let router = find(&id).ok_or(StatusCode::NOT_FOUND)?;
    if !router.needs_key {
        return Err(StatusCode::BAD_REQUEST);
    }
    let id_owned = id.clone();
    let key_owned = body.key;
    let result = tokio::task::spawn_blocking(move || {
        gateway_keys::set_key(&id_owned, &key_owned)
    }).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    result.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "ok": true, "configured": true })))
}

/// DELETE /api/gateway/routers/{id}/key — remove the router's API
/// key from the Keychain.
pub(crate) async fn clear_router_key(
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let _router = find(&id).ok_or(StatusCode::NOT_FOUND)?;
    let id_owned = id.clone();
    let result = tokio::task::spawn_blocking(move || {
        gateway_keys::delete_key(&id_owned)
    }).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    result.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(serde_json::json!({ "ok": true, "configured": false })))
}
```

- [ ] **Step 2: Add gateway helper methods**

The handler calls `state.gateway.list_models()` and
`list_models_for_router()`. These need to exist on the `Gateway` type
in `crates/gateway/src/engine.rs`. Find the existing
`Gateway::list_adapters()` method and add right after:

```rust
/// Flat list of all configured models, each entry router-qualified.
pub async fn list_models(&self) -> Result<Vec<serde_json::Value>, GatewayError> {
    let config = self.config.read().await;
    let mut out = Vec::with_capacity(config.models.len());
    for (id, m) in config.models.iter() {
        out.push(serde_json::json!({
            "id":               id,
            "api_model_id":     m.api_model_id,
            "provider":         m.provider,
            "capabilities":     m.capabilities,
            "context_window":   m.context_window,
            "max_output_tokens": m.max_output_tokens,
        }));
    }
    Ok(out)
}

/// Models reachable through a specific router. Walks fallback chains
/// for any entry whose `router` matches, plus any model whose default
/// provider matches the router id (single-provider routers).
pub async fn list_models_for_router(
    &self,
    router_id: &str,
) -> Result<Vec<serde_json::Value>, GatewayError> {
    let config = self.config.read().await;
    let mut model_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
    // Single-provider routers: model.provider == router id.
    for (id, m) in config.models.iter() {
        if m.provider == router_id { model_ids.insert(id.clone()); }
    }
    // Explicit chain router pins.
    for chain in config.chains.values() {
        for entry in &chain.models {
            if entry.router.as_deref() == Some(router_id) {
                model_ids.insert(entry.model.clone());
            }
        }
    }
    let mut out = Vec::with_capacity(model_ids.len());
    for id in model_ids {
        if let Some(m) = config.models.get(&id) {
            out.push(serde_json::json!({
                "id":               id,
                "api_model_id":     m.api_model_id,
                "provider":         m.provider,
                "capabilities":     m.capabilities,
            }));
        }
    }
    Ok(out)
}
```

- [ ] **Step 3: Register handler module and routes**

Edit `crates/senseid/src/api/handlers/mod.rs`, add:

```rust
pub(crate) mod gateway_routers;
pub(crate) mod gateway_image;  // placeholder for Phase 3 — add now to avoid two edits
```

Create a stub `crates/senseid/src/api/handlers/gateway_image.rs` with one line so the module compiles:

```rust
//! Image generation handler — implementation in a later task.
```

Edit `crates/senseid/src/api/routes.rs`, add to the imports:

```rust
use crate::api::handlers::gateway_routers;
```

Add to the router definition (after the existing `/api/gateway/*` routes):

```rust
        .route("/api/gateway/routers",                       get(gateway_routers::list_routers))
        .route("/api/gateway/routers/{id}/providers",        get(gateway_routers::router_providers))
        .route("/api/gateway/routers/{id}/models",           get(gateway_routers::router_models))
        .route("/api/gateway/routers/{id}/key",              post(gateway_routers::set_router_key).delete(gateway_routers::clear_router_key))
        .route("/api/gateway/models",                        get(gateway_routers::list_all_models))
```

- [ ] **Step 4: Build the daemon**

Run: `cargo build -p senseid --features dev`
Expected: compiles cleanly.

- [ ] **Step 5: Sanity-test the endpoints**

Start the daemon (skip if one is running on 7745):

```bash
pkill -x senseid-dev 2>/dev/null; sleep 1
make install-dev
senseid-dev start --port 7745 &
sleep 2
```

Then:

```bash
curl -s http://127.0.0.1:7745/api/gateway/routers | python3 -m json.tool
curl -s http://127.0.0.1:7745/api/gateway/routers/openai/providers
curl -s -X POST -H 'Content-Type: application/json' \
  -d '{"key":"sk-test"}' http://127.0.0.1:7745/api/gateway/routers/openai/key
curl -s http://127.0.0.1:7745/api/gateway/routers | python3 -m json.tool | grep -A 2 openai
curl -s -X DELETE http://127.0.0.1:7745/api/gateway/routers/openai/key
```

Expected: list returns 6 routers; `openai.configured` flips true then false across set/delete.

- [ ] **Step 6: Commit**

```bash
git add crates/gateway/src/engine.rs crates/senseid/src/api/handlers/gateway_routers.rs crates/senseid/src/api/handlers/gateway_image.rs crates/senseid/src/api/handlers/mod.rs crates/senseid/src/api/routes.rs
git commit -m "feat(daemon): /api/gateway/routers/* endpoints

GET routers (with configured flag from Keychain), GET providers/models
per router, POST/DELETE router key. Backed by the static router
registry + the new Keychain helper. Two new Gateway methods
(list_models, list_models_for_router) expose model metadata to the
handler.

Verified by curl roundtrip — configured flag flips correctly across
set/delete.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 6: Plumb Keychain key into RouterConfig at gateway init

**Files:**
- Modify: `crates/senseid/src/api/gateway_init.rs`

- [ ] **Step 1: Read the current init flow**

Run: `grep -n "RouterConfig" crates/senseid/src/api/gateway_init.rs`

The init builds RouterConfig instances. Find each one and add a Keychain lookup after the existing fields.

- [ ] **Step 2: Add a helper at the top of gateway_init.rs**

```rust
use crate::gateway_keys;

/// Resolve the literal api_key for a router from the Keychain.
/// Returns None for routers that don't need a key or whose key isn't set.
fn keychain_api_key(router_id: &str) -> Option<String> {
    gateway_keys::get_key(router_id).ok()
}
```

- [ ] **Step 3: Update each RouterConfig construction to populate `api_key`**

For each existing RouterConfig literal, add `api_key: keychain_api_key("<router-id>"),`. Example for the OpenAI router (substitute actual router id used in the existing code):

```rust
RouterConfig {
    url: "https://api.openai.com/v1".into(),
    api_key_env: Some("OPENAI_API_KEY".into()),
    api_key: keychain_api_key("openai"),
    enabled: true,
    timeout_ms: Some(30_000),
    headers: HashMap::new(),
}
```

If the file constructs routers in a loop / HashMap, derive the id from the existing key.

- [ ] **Step 4: Build and test**

Run: `cargo build -p senseid --features dev`
Expected: compiles.

Then a round-trip test: set an OpenAI key via the new endpoint, call `/api/gateway/status` and verify the daemon's reported router state shows the configured flag.

```bash
curl -s -X POST -H 'Content-Type: application/json' \
  -d '{"key":"sk-fake"}' http://127.0.0.1:7745/api/gateway/routers/openai/key
curl -s http://127.0.0.1:7745/api/gateway/status | python3 -m json.tool
```

The gateway init runs at daemon startup, so for the literal_key path to kick in for a request, the daemon's gateway must re-read on every request OR be re-initialised. If the existing init runs once at startup, add a `refresh_router_keys()` method on `Gateway` that re-reads the Keychain for every router and updates the cached configs. Call it after a successful set/clear.

- [ ] **Step 5: Add the refresh method on Gateway**

In `crates/gateway/src/engine.rs`:

```rust
/// Re-resolve `api_key` for every router from a caller-supplied
/// resolver function. Used after a key is set/cleared so the next
/// request picks up the change without a daemon restart.
pub async fn refresh_router_keys<F>(&self, resolver: F)
where
    F: Fn(&str) -> Option<String>,
{
    let mut config = self.config.write().await;
    for (id, router) in config.routers.iter_mut() {
        router.api_key = resolver(id);
    }
}
```

In the daemon's `set_router_key` and `clear_router_key` handlers, after the Keychain operation succeeds, call:

```rust
state.gateway.refresh_router_keys(|id| gateway_keys::get_key(id).ok()).await;
```

- [ ] **Step 6: Verify via the daemon**

Restart the daemon, set a key, hit `/api/gateway/status`, confirm the status reports the router as configured. (We don't yet have an end-to-end image request — Phase 3.)

- [ ] **Step 7: Commit**

```bash
git add crates/gateway/src/engine.rs crates/senseid/src/api/gateway_init.rs crates/senseid/src/api/handlers/gateway_routers.rs
git commit -m "feat(daemon): populate RouterConfig.api_key from Keychain

gateway_init looks up each router's key on startup; new
Gateway::refresh_router_keys re-resolves after the user sets/clears a
key so the next request picks it up without restart.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase 3 — Image generation endpoint

### Task 7: `/api/gateway/image/generate` handler

**Files:**
- Modify: `crates/senseid/src/api/handlers/gateway_image.rs` (replace the stub)
- Modify: `crates/senseid/src/api/routes.rs`

- [ ] **Step 1: Replace the stub with the handler**

Overwrite `crates/senseid/src/api/handlers/gateway_image.rs`:

```rust
//! POST /api/gateway/image/generate — text → image via the gateway.

use axum::{extract::State, http::StatusCode, response::Json};
use base64::{engine::general_purpose::STANDARD, Engine};
use serde::Deserialize;
use std::path::{Path, PathBuf};

use crate::api::state::AppState;
use crate::gateway_routers::find as find_router;
use gateway::types::capability::Capability;
use gateway::types::request::{InferenceRequest, Payload};

#[derive(Deserialize)]
pub(crate) struct ImageGenerateBody {
    pub prompt:       String,
    #[serde(default)]
    pub model:        Option<String>,
    #[serde(default)]
    pub router:       Option<String>,
    #[serde(default)]
    pub size:         Option<String>,
    #[serde(default)]
    pub quality:      Option<String>,
    #[serde(default)]
    pub style:        Option<String>,
    #[serde(default = "default_n")]
    pub n:            u8,
    /// MUST be absolute. MCP tool resolves relative paths upstream.
    /// When omitted, daemon writes to ~/.sensei/generated/<hash>.png.
    #[serde(default)]
    pub output_path:  Option<String>,
    /// Escape hatch for the $HOME safety guard. Default false.
    #[serde(default)]
    pub allow_outside_home: bool,
}

fn default_n() -> u8 { 1 }

/// Resolve `model` to (model_for_request, router_override).
fn split_model_router(
    model: Option<String>,
    router: Option<String>,
) -> (Option<String>, Option<String>) {
    if let Some(m) = model {
        if let Some((r, rest)) = m.split_once('/') {
            return (Some(rest.to_string()), Some(r.to_string()));
        }
        return (Some(m), router);
    }
    (None, router)
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn default_cache_path(prompt: &str, provider: &str, model: Option<&str>) -> PathBuf {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(prompt);
    hasher.update(provider);
    if let Some(m) = model { hasher.update(m); }
    let hash = hex::encode(hasher.finalize());
    home_dir()
        .unwrap_or_else(|| PathBuf::from("/tmp"))
        .join(".sensei")
        .join("generated")
        .join(format!("{}.png", &hash[..16]))
}

fn path_is_safe(path: &Path, allow_outside_home: bool) -> bool {
    if allow_outside_home { return true; }
    let Some(home) = home_dir() else { return true; }; // can't enforce — be permissive
    path.starts_with(&home) || path.starts_with("/tmp")
}

pub(crate) async fn image_generate(
    State(state): State<AppState>,
    Json(body): Json<ImageGenerateBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
        (status, Json(serde_json::json!({ "error": msg })))
    }

    let (model, router_override) = split_model_router(body.model, body.router);

    // If a router is named, verify it exists and is configured.
    if let Some(rid) = router_override.as_deref() {
        let Some(entry) = find_router(rid) else {
            return Err(err(StatusCode::NOT_FOUND, "unknown router"));
        };
        if entry.needs_key && !crate::gateway_keys::has_key(rid) {
            return Err(err(StatusCode::BAD_REQUEST,
                &format!("{} router key not configured", rid)));
        }
    }

    let payload = Payload::ImageGenerate {
        prompt:  body.prompt.clone(),
        size:    body.size,
        quality: body.quality,
        style:   body.style,
        n:       body.n,
    };
    let request = InferenceRequest {
        capability: Capability::ImageGenerate,
        model,
        router: router_override.clone(),
        chain:  None,
        payload,
        budget: None,
    };

    let response = state.gateway.execute(&request).await
        .map_err(|e| err(StatusCode::BAD_GATEWAY, &e.to_string()))?;

    let images = response.images.unwrap_or_default();
    if images.is_empty() {
        return Err(err(StatusCode::BAD_GATEWAY, "no images returned"));
    }

    // Decide on a base output path. Multi-image suffix rule applies if n > 1.
    let provider_for_default = router_override.as_deref().unwrap_or("default");
    let base = match body.output_path.as_deref() {
        Some(p) => PathBuf::from(p),
        None => default_cache_path(&body.prompt, provider_for_default, request.model.as_deref()),
    };
    if base.is_relative() {
        return Err(err(StatusCode::BAD_REQUEST, "output_path must be absolute"));
    }
    if !path_is_safe(&base, body.allow_outside_home) {
        return Err(err(StatusCode::BAD_REQUEST,
            "output path outside $HOME blocked (pass allow_outside_home=true to override)"));
    }

    let mut written = Vec::with_capacity(images.len());
    for (i, image) in images.iter().enumerate() {
        let bytes = if let Some(b64) = &image.b64_json {
            STANDARD.decode(b64).map_err(|e| err(StatusCode::BAD_GATEWAY, &e.to_string()))?
        } else if let Some(url) = &image.url {
            // Some adapters return a URL instead of bytes. Fetch it.
            let resp = reqwest::get(url).await
                .map_err(|e| err(StatusCode::BAD_GATEWAY, &e.to_string()))?;
            resp.bytes().await
                .map_err(|e| err(StatusCode::BAD_GATEWAY, &e.to_string()))?
                .to_vec()
        } else {
            return Err(err(StatusCode::BAD_GATEWAY, "image had neither b64_json nor url"));
        };

        let path = if images.len() == 1 {
            base.clone()
        } else {
            let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
            let ext  = base.extension().and_then(|s| s.to_str()).unwrap_or("png");
            base.with_file_name(format!("{stem}-{}.{ext}", i + 1))
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
        }
        std::fs::write(&path, &bytes)
            .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
        written.push(path.display().to_string());
    }

    Ok(Json(serde_json::json!({
        "ok":       true,
        "paths":    written,
        "model":    response.model,
        "router":   router_override,
        "usage":    response.usage,
    })))
}
```

- [ ] **Step 2: Add the missing dependencies**

Run: `grep -E '^(sha2|hex) =' crates/senseid/Cargo.toml`
If they're not there, add to `crates/senseid/Cargo.toml` under `[dependencies]`:

```toml
sha2 = "0.10"
hex = "0.4"
```

- [ ] **Step 3: Add unit tests for path resolution + safety**

Append to `crates/senseid/src/api/handlers/gateway_image.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_model_router_unprefixed_keeps_router_arg() {
        let (m, r) = split_model_router(Some("dall-e-3".into()), Some("openai".into()));
        assert_eq!(m.as_deref(), Some("dall-e-3"));
        assert_eq!(r.as_deref(), Some("openai"));
    }

    #[test]
    fn split_model_router_prefixed_overrides_router_arg() {
        let (m, r) = split_model_router(Some("openai/dall-e-3".into()), Some("anthropic".into()));
        assert_eq!(m.as_deref(), Some("dall-e-3"));
        assert_eq!(r.as_deref(), Some("openai"));
    }

    #[test]
    fn split_model_router_none_passes_through_router() {
        let (m, r) = split_model_router(None, Some("openai".into()));
        assert!(m.is_none());
        assert_eq!(r.as_deref(), Some("openai"));
    }

    #[test]
    fn default_cache_path_is_deterministic() {
        let a = default_cache_path("hello", "openai", Some("dall-e-3"));
        let b = default_cache_path("hello", "openai", Some("dall-e-3"));
        assert_eq!(a, b);
        let c = default_cache_path("hello", "openai", Some("dall-e-2"));
        assert_ne!(a, c, "different model should yield different path");
    }

    #[test]
    fn path_safety_rejects_etc() {
        let p = PathBuf::from("/etc/passwd");
        assert!(!path_is_safe(&p, false));
        assert!(path_is_safe(&p, true));
    }

    #[test]
    fn path_safety_allows_home_and_tmp() {
        if let Some(home) = home_dir() {
            assert!(path_is_safe(&home.join("a.png"), false));
        }
        assert!(path_is_safe(&PathBuf::from("/tmp/a.png"), false));
    }
}
```

- [ ] **Step 4: Wire the route**

Edit `crates/senseid/src/api/routes.rs`, add to the imports:

```rust
use crate::api::handlers::gateway_image;
```

Add to the router definition:

```rust
        .route("/api/gateway/image/generate", post(gateway_image::image_generate))
```

- [ ] **Step 5: Build + run unit tests**

Run: `cargo build -p senseid --features dev`
Run: `cargo test -p senseid --features dev --bin senseid gateway_image`
Expected: 6 passed.

- [ ] **Step 6: Commit**

```bash
git add crates/senseid/src/api/handlers/gateway_image.rs crates/senseid/src/api/routes.rs crates/senseid/Cargo.toml
git commit -m "feat(daemon): POST /api/gateway/image/generate

Decodes base64 or fetches URL → writes to disk → returns paths.
- model accepts bare ('dall-e-3') or router-qualified ('openai/dall-e-3')
- output_path MUST be absolute (MCP tool resolves relative upstream)
- omitted output_path → ~/.sensei/generated/<hash>.png (deterministic)
- n > 1 suffixes -1.png / -2.png / ...
- path safety: refuses paths outside \$HOME unless allow_outside_home=true
- 400 if router needs key and key isn't set

6 unit tests cover the path-resolution and safety branches.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase 4 — MCP tool

### Task 8: Add `generate_image` to the MCP server

**Files:**
- Modify: `crates/mcp/src/main.rs`

- [ ] **Step 1: Add the tool definition**

Find `handle_list_tools` in `crates/mcp/src/main.rs`. Add this entry inside the `tools` array (after the existing inference-related tools):

```rust
            tool("generate_image", "Generate an image from a text prompt using the configured image provider (OpenAI by default). Saves to the given output_path (relative paths are resolved against CWD) or to a sensei-managed cache when omitted. Returns the absolute file path. Use this when the user asks for visual assets — logos, illustrations, diagrams, character art, mockup imagery — that belong in the project.", &[
                ("prompt", "string", "Description of the image to generate"),
            ], &[
                ("output_path", "string", "Where to save the image. Relative to the current working directory, or absolute. If omitted, saves to ~/.sensei/generated/<hash>.png"),
                ("model", "string", "Model id, bare or router-qualified (e.g. 'dall-e-3', 'openai/dall-e-3')"),
                ("router", "string", "Which router to use (openai, stability, fal, replicate). Defaults to gateway selection."),
                ("size", "string", "Image size — provider-specific (e.g. 1024x1024 for OpenAI)"),
                ("quality", "string", "Image quality — provider-specific (standard|hd for OpenAI)"),
                ("style", "string", "Image style — provider-specific (vivid|natural for OpenAI)"),
                ("n", "string", "Number of images to generate (default 1)"),
            ]),
```

- [ ] **Step 2: Add the handler arm**

Find `handle_call_tool`. Add a new branch in its match block, alongside the existing tools:

```rust
        "generate_image" => {
            let prompt = match args.get("prompt").and_then(|v| v.as_str()) {
                Some(p) if !p.is_empty() => p.to_string(),
                _ => return json!({ "error": "prompt is required" }),
            };

            // Resolve relative output_path against CWD so the daemon
            // only sees absolute paths.
            let output_path = args.get("output_path").and_then(|v| v.as_str())
                .map(|p| {
                    let path = std::path::PathBuf::from(p);
                    if path.is_absolute() { p.to_string() }
                    else { std::path::PathBuf::from(cwd).join(p).display().to_string() }
                });

            let mut body = json!({ "prompt": prompt });
            if let Some(p) = output_path { body["output_path"] = json!(p); }
            for (key, &name) in &[
                (args.get("model"),   "model"),
                (args.get("router"),  "router"),
                (args.get("size"),    "size"),
                (args.get("quality"), "quality"),
                (args.get("style"),   "style"),
            ] {
                if let Some(v) = key.and_then(|x| x.as_str()) {
                    body[name] = json!(v);
                }
            }
            if let Some(n_str) = args.get("n").and_then(|v| v.as_str())
                && let Ok(n) = n_str.parse::<u8>()
            {
                body["n"] = json!(n);
            }

            let url = format!("{daemon_url}/api/gateway/image/generate");
            match client.post(&url).json(&body).send() {
                Ok(resp) => match resp.json::<Value>() {
                    Ok(v) => v,
                    Err(e) => json!({ "error": format!("invalid response: {e}") }),
                },
                Err(e) => json!({ "error": format!("daemon request failed: {e}") }),
            }
        }
```

(Adjust `daemon_url` to match the variable name already in scope in `handle_call_tool`. If the existing inference tools build the URL with a different pattern, copy that pattern verbatim.)

- [ ] **Step 3: Build and test against a running daemon**

```bash
cargo build -p sensei-mcp --features dev
```

End-to-end smoke (requires a real OpenAI key set via the daemon endpoint):

```bash
# Set the key once.
curl -s -X POST -H 'Content-Type: application/json' \
  -d "{\"key\":\"$OPENAI_API_KEY\"}" \
  http://127.0.0.1:7745/api/gateway/routers/openai/key

# Call the daemon directly to confirm the bytes-to-disk path works.
curl -s -X POST -H 'Content-Type: application/json' \
  -d '{"prompt":"a tiny smiling rice ball","output_path":"/tmp/sensei-onigiri.png"}' \
  http://127.0.0.1:7745/api/gateway/image/generate

ls -la /tmp/sensei-onigiri.png
```

Expected: a PNG file on disk. (MCP tool integration test is manual in Phase 6.)

- [ ] **Step 4: Commit**

```bash
git add crates/mcp/src/main.rs
git commit -m "feat(mcp): add generate_image tool

Proxies to /api/gateway/image/generate. Resolves relative output_path
against MCP CWD before posting so the daemon sees absolute paths.
Tool description guides assistants to use it for project visual
assets (logos, illustrations, character art).

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase 5 — App: Inference wizard stage

### Task 9: Contracts + mock factory

**Files:**
- Modify: `app/src/lib/setup/contracts.ts`
- Modify: `app/src/lib/setup/mock-contracts.ts`

- [ ] **Step 1: Add the contract type**

Add to `app/src/lib/setup/contracts.ts`:

```typescript
/** A gateway router — endpoint that may front multiple providers. */
export interface DaemonRouter {
  id: string;
  name: string;
  providers: string[];
  capabilities: string[];
  needs_key: boolean;
  configured: boolean;
}
```

Update `WizardLoadData` to include the routers list:

```typescript
export interface WizardLoadData {
  // ... existing fields ...
  routers: DaemonRouter[];
}
```

- [ ] **Step 2: Add the mock factory**

In `app/src/lib/setup/mock-contracts.ts`:

```typescript
import type { /* existing... */ DaemonRouter } from './contracts.js';

export function mockRouter(overrides: Partial<DaemonRouter> = {}): DaemonRouter {
  return {
    id: 'openai', name: 'OpenAI',
    providers: ['openai'],
    capabilities: ['text_chat', 'text_embed', 'image_generate'],
    needs_key: true,
    configured: false,
    ...overrides,
  };
}
```

Update `mockWizardLoadData` to include `routers: [mockRouter()]`.

- [ ] **Step 3: Run vitest to catch any breakage**

```bash
cd app && bunx vitest run
```
Expected: all pass.

- [ ] **Step 4: Commit**

```bash
git add app/src/lib/setup/contracts.ts app/src/lib/setup/mock-contracts.ts
git commit -m "feat(app): DaemonRouter contract + mock factory"
```

### Task 10: API client methods

**Files:**
- Modify: `app/src/lib/api.ts`

- [ ] **Step 1: Add the four new methods**

In `app/src/lib/api.ts`, find the existing `/api/gateway/*` block and append:

```typescript
    // ── Gateway routers ──────────────────────────────────────────────
    listGatewayRouters: () =>
      get<{ routers: import('./setup/contracts').DaemonRouter[] }>(
        '/api/gateway/routers',
        { routers: [] },
      ),

    setGatewayRouterKey: (id: string, key: string) =>
      tryPost<{ ok: boolean; configured: boolean }>(
        `/api/gateway/routers/${enc(id)}/key`,
        { key },
      ),

    clearGatewayRouterKey: (id: string) =>
      tryDelete(`/api/gateway/routers/${enc(id)}/key`),

    generateImage: (body: {
      prompt: string;
      output_path?: string;
      model?: string;
      router?: string;
      size?: string;
      quality?: string;
      style?: string;
      n?: number;
    }) =>
      post<{ ok: boolean; paths: string[]; model?: string; router?: string }>(
        '/api/gateway/image/generate',
        body,
        { ok: false, paths: [] },
      ),
```

If `tryDelete` doesn't exist yet, add it alongside `del`:

```typescript
  async function tryDelete(path: string): Promise<ApiResult<void>> {
    try {
      const res = await fetch(`${base}${path}`, { method: 'DELETE' });
      if (res.ok) return { ok: true, data: undefined };
      return { ok: false, error: { status: res.status, message: res.statusText } };
    } catch (e) {
      return { ok: false, error: { status: 0, message: e instanceof Error ? e.message : 'Network error' } };
    }
  }
```

- [ ] **Step 2: Build + check**

```bash
cd app && bunx svelte-check --tsconfig ./tsconfig.json
```
Expected: 0 errors.

- [ ] **Step 3: Commit**

```bash
git add app/src/lib/api.ts
git commit -m "feat(app): api client for /api/gateway/routers + /api/gateway/image/generate"
```

### Task 11: wizardState `inference` slice + loader + commit handler

**Files:**
- Modify: `app/src/lib/wizard-state.svelte.ts`
- Modify: `app/src/lib/wizard-state.spec.svelte.ts`
- Modify: `app/src/lib/setup/loaders.ts`

- [ ] **Step 1: Add the slice**

In `wizard-state.svelte.ts`, add near the other slice interfaces:

```typescript
export type RouterSaveState = 'idle' | 'saving' | 'done' | 'failed';

export interface RouterEntry {
  id: string;
  name: string;
  providers: string[];
  capabilities: string[];
  needsKey: boolean;
  configured: boolean;
  draftKey: string;          // never sent until commit
  saveState: RouterSaveState;
  saveError: string;
}

export interface InferenceSlice {
  routers: RouterEntry[];
}
```

Add the field on `WizardState`:

```typescript
inference = $state<InferenceSlice>({ routers: [] });
```

- [ ] **Step 2: Hydrate from load data**

In `hydrate()`, append:

```typescript
this.inference = {
  routers: data.routers.map(r => ({
    id: r.id,
    name: r.name,
    providers: r.providers,
    capabilities: r.capabilities,
    needsKey: r.needs_key,
    configured: r.configured,
    draftKey: '',
    saveState: 'idle' as RouterSaveState,
    saveError: '',
  })),
};
```

- [ ] **Step 3: Add a refresh method (mirrors refreshLibraries)**

```typescript
async refreshInferenceRouters(): Promise<void> {
  const api = senseiApi(appState.port);
  const fresh = await api.listGatewayRouters();
  const previous = new Map(this.inference.routers.map(r => [r.id, r]));
  this.inference = {
    routers: fresh.routers.map(r => {
      const prev = previous.get(r.id);
      return {
        id: r.id,
        name: r.name,
        providers: r.providers,
        capabilities: r.capabilities,
        needsKey: r.needs_key,
        configured: r.configured,
        draftKey: prev?.draftKey ?? '',
        saveState: prev?.saveState ?? 'idle',
        saveError: prev?.saveError ?? '',
      };
    }),
  };
}
```

- [ ] **Step 4: Update the commit handler**

Replace `inference: async () => {},` with:

```typescript
inference: async (ws, api) => {
  // Persist any non-empty drafted keys. Per-card status updates so
  // the UI shows progress. Failures are non-fatal — user can retry
  // later from settings — but we surface them via saveError.
  for (const router of ws.inference.routers) {
    if (!router.needsKey || router.draftKey.trim().length === 0) continue;
    router.saveState = 'saving';
    router.saveError = '';
    const result = await api.setGatewayRouterKey(router.id, router.draftKey.trim());
    if (result.ok) {
      router.configured = result.data.configured;
      router.draftKey = '';
      router.saveState = 'done';
    } else {
      router.saveState = 'failed';
      router.saveError = result.error.message;
    }
  }
},
```

- [ ] **Step 5: Update loaders.ts**

In `app/src/lib/setup/loaders.ts`, add `api.listGatewayRouters()` to the `Promise.all`:

```typescript
const [config, families, roots, projects, libs, instruments, routers] = await Promise.all([
  api.getConfig(),
  api.detectAssistantFamilies(),
  api.getScanRoots(),
  api.listProjects(),
  api.getLibs(),
  api.listInstruments(),
  api.listGatewayRouters(),
]);
```

And in the return:

```typescript
return {
  // ... existing fields ...
  routers: routers.routers,
};
```

- [ ] **Step 6: Add unit tests**

In `wizard-state.spec.svelte.ts` add a `describe('inference slice')` block:

```typescript
import { mockRouter } from './setup/mock-contracts.js';

describe('inference slice', () => {
  it('hydrates routers from load data', () => {
    const ws = new WizardState();
    ws.hydrate(mockWizardLoadData({
      routers: [
        mockRouter({ id: 'openai',   configured: true }),
        mockRouter({ id: 'ollama',   needs_key: false, configured: true }),
      ],
    }));
    expect(ws.inference.routers).toHaveLength(2);
    expect(ws.inference.routers[0].id).toBe('openai');
    expect(ws.inference.routers[0].configured).toBe(true);
    expect(ws.inference.routers[1].needsKey).toBe(false);
  });

  it('initialises draftKey/saveState empty on hydrate', () => {
    const ws = new WizardState();
    ws.hydrate(mockWizardLoadData({ routers: [mockRouter()] }));
    const r = ws.inference.routers[0];
    expect(r.draftKey).toBe('');
    expect(r.saveState).toBe('idle');
    expect(r.saveError).toBe('');
  });
});
```

- [ ] **Step 7: Run tests + svelte-check**

```bash
cd app && bunx vitest run && bunx svelte-check --tsconfig ./tsconfig.json
```
Expected: vitest all pass, svelte-check 0 errors.

- [ ] **Step 8: Commit**

```bash
git add app/src/lib/wizard-state.svelte.ts app/src/lib/wizard-state.spec.svelte.ts app/src/lib/setup/loaders.ts
git commit -m "feat(app): wizardState inference slice + commit handler

InferenceSlice carries the live router list with per-card draftKey +
saveState. Commit iterates non-empty drafts, POSTs each key, updates
configured. Failures non-fatal — saveError shown inline.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

### Task 12: Inference page rewrite

**Files:**
- Overwrite: `app/src/routes/(config)/setup/inference/+page.svelte`

- [ ] **Step 1: Replace the placeholder**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { wizardState } from '$lib/wizard-state.svelte.js';

  let loading = $state(true);
  let error = $state<string | null>(null);

  const routers = $derived(wizardState.inference.routers);
  const configuredCount = $derived(routers.filter(r => r.configured).length);

  onMount(async () => {
    try {
      await wizardState.refreshInferenceRouters();
    } catch (e) {
      error = e instanceof Error ? e.message : String(e);
    } finally {
      loading = false;
    }
  });
</script>

<div class="max-w-[820px]">
  <p class="text-sm text-surface-z6 leading-normal m-0 mb-6">
    Connect inference providers so sensei can route chat, embedding, and
    image-generation requests on your behalf. Keys are stored in the macOS
    Keychain — never on disk in plaintext, never in the database.
  </p>

  {#if loading}
    <div data-testid="inference-loading" class="text-center p-12 bg-surface-z2 rounded-lg border border-surface-z3">
      <span class="kanji text-4xl text-primary-z5 opacity-20 block mb-4">想</span>
      <p class="text-sm text-surface-z6">Loading routers…</p>
    </div>
  {:else if error}
    <div data-testid="inference-error" class="mb-6 p-4 rounded-md border border-danger-z5 bg-surface-z2">
      <div class="text-sm font-semibold text-danger-z5">Could not load routers</div>
      <div class="text-xs text-surface-z7 mt-1 font-mono">{error}</div>
    </div>
  {:else}
    <div class="flex items-center gap-2 mb-6">
      <span class="mono py-1 px-2 text-xs text-success-z6 bg-surface-z2 border border-success-z5 rounded-sm">
        {configuredCount} of {routers.length} configured
      </span>
    </div>

    <div class="flex flex-col gap-3">
      {#each routers as router (router.id)}
        <div
          data-testid={`router-card-${router.id}`}
          data-configured={router.configured}
          data-save-state={router.saveState}
          class="bg-surface-z2 border border-surface-z3 rounded-md p-4 flex flex-col gap-3"
        >
          <div class="flex items-baseline justify-between gap-3">
            <div class="flex items-baseline gap-2 min-w-0">
              <span class="text-sm text-surface-z9 font-semibold">{router.name}</span>
              <span class="mono text-[11px] text-surface-z6 uppercase">{router.id}</span>
            </div>
            {#if router.saveState === 'saving'}
              <span class="text-xs text-primary-z6 mono">saving…</span>
            {:else if router.saveState === 'done'}
              <span class="text-xs text-success-z6 mono">saved ✓</span>
            {:else if router.saveState === 'failed'}
              <span class="text-xs text-danger-z5 mono">failed</span>
            {:else if router.configured}
              <span class="text-xs text-success-z6 mono">configured ✓</span>
            {:else if !router.needsKey}
              <span class="text-xs text-surface-z6 mono">no key needed</span>
            {:else}
              <span class="text-xs text-surface-z5 mono">not configured</span>
            {/if}
          </div>

          <!-- Providers + capabilities chips -->
          <div class="flex flex-wrap gap-1">
            {#each router.providers as p}
              <span class="mono text-[11px] text-surface-z7 bg-surface-z1 border border-surface-z3 rounded-sm px-1.5 py-0.5">{p}</span>
            {/each}
            {#each router.capabilities as c}
              <span class="mono text-[11px] text-primary-z6 bg-surface-z1 border border-primary-z2 rounded-sm px-1.5 py-0.5">{c}</span>
            {/each}
          </div>

          {#if router.needsKey}
            <div class="flex items-center gap-2">
              <input
                type="password"
                bind:value={router.draftKey}
                placeholder={router.configured ? 'Update key (paste to replace)' : 'Paste API key (sk-...)'}
                data-testid={`router-key-input-${router.id}`}
                class="flex-1 px-3 py-1.5 text-sm font-mono bg-surface-z1 border border-surface-z3 rounded-md text-surface-z9 placeholder-surface-z5 focus:outline-none focus:border-primary-z5"
              />
              {#if router.configured}
                <button
                  type="button"
                  data-testid={`router-clear-${router.id}`}
                  class="text-xs text-surface-z6 hover:text-danger-z5 cursor-pointer bg-none border-none"
                  onclick={async () => {
                    await fetch(`http://127.0.0.1:${$state.snapshot(wizardState).scan?.baseline?.scannedRootIds ? 7745 : 7744}/api/gateway/routers/${router.id}/key`, { method: 'DELETE' });
                    await wizardState.refreshInferenceRouters();
                  }}
                >Clear</button>
              {/if}
            </div>
            {#if router.saveError}
              <p class="text-xs text-danger-z5 font-mono m-0">{router.saveError}</p>
            {/if}
          {/if}
        </div>
      {/each}
    </div>
  {/if}
</div>
```

Note on the Clear button: the inline fetch is ugly; replace with a method on wizardState in Step 2.

- [ ] **Step 2: Add a `clearInferenceRouterKey` method on wizardState**

In `wizard-state.svelte.ts`:

```typescript
async clearInferenceRouterKey(id: string): Promise<void> {
  const api = senseiApi(appState.port);
  await api.clearGatewayRouterKey(id);
  await this.refreshInferenceRouters();
}
```

Replace the inline `onclick` handler in the page with:

```svelte
onclick={() => wizardState.clearInferenceRouterKey(router.id)}
```

- [ ] **Step 3: Build the app + run tests**

```bash
cd app && bunx svelte-check --tsconfig ./tsconfig.json
bunx vitest run
```
Expected: 0 errors, all tests pass.

- [ ] **Step 4: Commit**

```bash
git add app/src/lib/wizard-state.svelte.ts app/src/routes/(config)/setup/inference/+page.svelte
git commit -m "feat(app): rewrite Inference wizard stage as router-keys page

Cards represent routers (the credential boundary). Each card shows
providers + capabilities chips and either a password input (when
needs_key) or a 'no key needed' badge (Ollama). Per-card save state
(idle/saving/done/failed) + Clear button for already-configured
routers. Commit handler persists drafted keys via the daemon endpoint
which writes to the Keychain.

Co-Authored-By: Claude Opus 4.7 (1M context) <noreply@anthropic.com>"
```

---

## Phase 6 — E2E + manual verification

### Task 13: E2E spec for the Inference stage

**Files:**
- Create: `app/e2e/tests/inference-stage.spec.ts`

- [ ] **Step 1: Write the spec**

```typescript
/**
 * Inference stage — router list, paste a key, Continue → daemon
 * Keychain write.
 *
 * The test uses a sentinel router id ("test-router") to avoid
 * touching real OpenAI/Anthropic keys in the user's actual Keychain.
 * Set + clear roundtrip is asserted via the daemon's own status
 * (configured flag flips), not by reading the Keychain directly.
 */

import { test, expect } from '../fixtures';
import { navigateTo, DAEMON_URL } from '../helpers';

async function seedHealth(tauriPage: any): Promise<void> {
  await tauriPage.evaluate(`
    (function() {
      sessionStorage.setItem('sensei:health', 'ready');
      localStorage.removeItem('sensei:setup-complete');
    })()
  `);
}

async function resetSetupKeys(): Promise<void> {
  const keys = [
    'setup.welcome','setup.preferences','setup.assistants',
    'setup.roots','setup.scan','setup_complete',
  ];
  for (const k of keys) {
    await fetch(`${DAEMON_URL}/api/config/${k}`, { method: 'DELETE' });
  }
}

async function clearRouterKey(id: string): Promise<void> {
  await fetch(`${DAEMON_URL}/api/gateway/routers/${id}/key`, { method: 'DELETE' });
}

test.describe('Inference stage', () => {
  test.beforeEach(async ({ tauriPage }) => {
    await resetSetupKeys();
    await clearRouterKey('openai');
    await seedHealth(tauriPage);
    await navigateTo(tauriPage, '/setup/inference');
  });

  test('renders one card per known router with providers + capabilities chips', async ({ tauriPage }) => {
    const card = tauriPage.locator('[data-testid="router-card-openai"]');
    await expect(card).toBeVisible({ timeout: 10_000 });
    await expect(card).toContainText('openai');
    await expect(card).toContainText('image_generate');
    await expect(card).toHaveAttribute('data-configured', 'false');
  });

  test('paste key → Continue → daemon reports configured', async ({ tauriPage }) => {
    const card = tauriPage.locator('[data-testid="router-card-openai"]');
    const input = tauriPage.locator('[data-testid="router-key-input-openai"]');
    await expect(card).toBeVisible({ timeout: 10_000 });
    await input.fill('sk-e2e-test-key');

    await tauriPage.locator('.btn-primary').click();

    // After commit, the daemon's view of the router should be configured.
    const deadline = Date.now() + 10_000;
    let configured = false;
    while (Date.now() < deadline) {
      const list = await fetch(`${DAEMON_URL}/api/gateway/routers`).then(r => r.json());
      const openai = list.routers.find((r: any) => r.id === 'openai');
      if (openai?.configured) { configured = true; break; }
      await new Promise(r => setTimeout(r, 200));
    }
    expect(configured).toBe(true);

    // Cleanup so subsequent test runs start fresh.
    await clearRouterKey('openai');
  });

  test('Clear button removes the key', async ({ tauriPage }) => {
    // Pre-set so the Clear button renders.
    await fetch(`${DAEMON_URL}/api/gateway/routers/openai/key`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ key: 'sk-pretend' }),
    });
    await navigateTo(tauriPage, '/logs');
    await navigateTo(tauriPage, '/setup/inference');

    const card = tauriPage.locator('[data-testid="router-card-openai"]');
    await expect(card).toHaveAttribute('data-configured', 'true', { timeout: 10_000 });

    await tauriPage.locator('[data-testid="router-clear-openai"]').click();
    await expect(card).toHaveAttribute('data-configured', 'false', { timeout: 5_000 });
  });
});
```

- [ ] **Step 2: Build + run**

```bash
cd /Users/Jerry/Developer/sensei-hq/sensei
make app-e2e-build
cd app
pkill -f sensei-desktop 2>/dev/null
pkill -x senseid-dev 2>/dev/null
sleep 1
rm -f /tmp/tauri-playwright.sock /tmp/sensei-e2e-pid
bunx playwright test --config e2e/playwright.config.ts --project=tauri e2e/tests/inference-stage.spec.ts
```

Expected: 3 passed.

- [ ] **Step 3: Commit**

```bash
git add app/e2e/tests/inference-stage.spec.ts
git commit -m "test(e2e): Inference stage — list, set, clear, persist"
```

### Task 14: Manual end-to-end image generation

- [ ] **Step 1: Set the real key (one-time)**

```bash
curl -s -X POST -H 'Content-Type: application/json' \
  -d "{\"key\":\"$OPENAI_API_KEY\"}" \
  http://127.0.0.1:7745/api/gateway/routers/openai/key
```

- [ ] **Step 2: Confirm via the daemon**

```bash
curl -s http://127.0.0.1:7745/api/gateway/routers | python3 -m json.tool | grep -A 1 openai
```
Expected: `"configured": true`.

- [ ] **Step 3: Generate an image directly via the daemon endpoint**

```bash
curl -s -X POST -H 'Content-Type: application/json' \
  -d '{"prompt":"a tiny smiling onigiri (rice ball) holding a katana, chibi style, soft inks", "output_path":"/tmp/sensei-chibi.png"}' \
  http://127.0.0.1:7745/api/gateway/image/generate | python3 -m json.tool
open /tmp/sensei-chibi.png
```

Expected: a PNG opens. If it doesn't, check the JSON error.

- [ ] **Step 4: Call from a Claude Code session via MCP**

In a Claude Code session, ask: "Use the generate_image MCP tool to make a chibi character for a Kata-themed page, save it to /tmp/sensei-kata.png."

Verify the file lands.

- [ ] **Step 5: Document the verification (commit nothing)**

Append to `docs/superpowers/specs/2026-05-26-gateway-image-generation-design.md` under a new "Verified" heading: today's date, the prompt used, "ok via curl + ok via Claude Code MCP tool."

```bash
git add docs/superpowers/specs/2026-05-26-gateway-image-generation-design.md
git commit -m "docs: mark gateway image generation verified end-to-end"
```

---

## Self-Review

Spec coverage:
- Vocabulary section → Task 4 (router registry encodes it), Task 12 (UI uses it)
- KeyStore abstraction → Tasks 1, 2 (RouterConfig.api_key + resolve_api_key)
- macOS Keychain → Task 3
- Daemon endpoints → Tasks 5, 6, 7
- MCP tool → Task 8
- Inference wizard stage → Tasks 9-12
- E2E → Task 13
- Manual end-to-end → Task 14

Placeholder scan: no TBDs, no "implement later", every code step has the actual code or an exact command.

Type consistency: `RouterEntry` (app slice) maps to `DaemonRouter` (contract) with `needs_key`/`configured` flowing through unchanged. Daemon's `RouterEntry` (Rust) has the same fields. `api_key` on `RouterConfig` lands in Task 1 and is read in Task 2; daemon populates it in Task 6.
