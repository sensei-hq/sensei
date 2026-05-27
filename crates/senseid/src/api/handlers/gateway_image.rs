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
    if path.starts_with(&home) { return true; }
    if path.starts_with("/tmp") { return true; }
    #[cfg(target_os = "macos")]
    if path.starts_with("/private/tmp") { return true; }
    false
}

/// Detect the image format from magic bytes and return the appropriate extension.
fn detect_extension(bytes: &[u8]) -> &'static str {
    match bytes {
        [0xFF, 0xD8, 0xFF, ..]                                                    => "jpg",
        [0x89, b'P', b'N', b'G', ..]                                              => "png",
        [b'R', b'I', b'F', b'F', _, _, _, _, b'W', b'E', b'B', b'P', ..]        => "webp",
        [b'G', b'I', b'F', b'8', ..]                                              => "gif",
        _                                                                          => "png",
    }
}

pub(crate) async fn image_generate(
    State(state): State<AppState>,
    Json(body): Json<ImageGenerateBody>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    fn err(status: StatusCode, msg: &str) -> (StatusCode, Json<serde_json::Value>) {
        (status, Json(serde_json::json!({ "error": msg })))
    }

    // Important 3: reject empty prompts before any further processing.
    if body.prompt.trim().is_empty() {
        return Err(err(StatusCode::BAD_REQUEST, "prompt must not be empty"));
    }

    let (model, router_override) = split_model_router(body.model, body.router);

    // If a router is named, verify it exists and is configured.
    if let Some(rid) = router_override.as_deref() {
        let Some(entry) = find_router(rid) else {
            return Err(err(StatusCode::NOT_FOUND, "unknown router"));
        };
        if entry.needs_key {
            let rid_owned = rid.to_string();
            let key_present = tokio::task::spawn_blocking(move || {
                crate::gateway_keys::has_key(&rid_owned)
            }).await.unwrap_or(false);
            if !key_present {
                return Err(err(StatusCode::BAD_REQUEST,
                    &format!("{} router key not configured", rid)));
            }
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
            // Some adapters return a URL instead of bytes. Fetch with a timeout.
            let resp = reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(60))
                .build()
                .map_err(|e| err(StatusCode::BAD_GATEWAY, &e.to_string()))?
                .get(url)
                .send()
                .await
                .map_err(|e| err(StatusCode::BAD_GATEWAY, &e.to_string()))?;
            resp.bytes().await
                .map_err(|e| err(StatusCode::BAD_GATEWAY, &e.to_string()))?
                .to_vec()
        } else {
            return Err(err(StatusCode::BAD_GATEWAY, "image had neither b64_json nor url"));
        };

        let path = if images.len() == 1 {
            if body.output_path.is_none() {
                // No explicit path — use detected extension from actual bytes.
                let ext = detect_extension(&bytes);
                let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
                base.with_file_name(format!("{stem}.{ext}"))
            } else {
                base.clone()
            }
        } else {
            let stem = base.file_stem().and_then(|s| s.to_str()).unwrap_or("image");
            let ext = if body.output_path.is_none() {
                detect_extension(&bytes)
            } else {
                base.extension().and_then(|s| s.to_str()).unwrap_or("png")
            };
            base.with_file_name(format!("{stem}-{}.{ext}", i + 1))
        };
        if let Some(parent) = path.parent() {
            tokio::fs::create_dir_all(parent).await
                .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()))?;
        }
        tokio::fs::write(&path, &bytes).await
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

    #[test]
    #[cfg(target_os = "macos")]
    fn path_safety_allows_private_tmp_on_macos() {
        assert!(path_is_safe(&PathBuf::from("/private/tmp/a.png"), false));
    }

    #[test]
    fn detect_extension_identifies_formats() {
        assert_eq!(detect_extension(&[0xFF, 0xD8, 0xFF, 0xE0]), "jpg");
        assert_eq!(detect_extension(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]), "png");
        assert_eq!(detect_extension(&[b'R', b'I', b'F', b'F', 0, 0, 0, 0, b'W', b'E', b'B', b'P']), "webp");
        assert_eq!(detect_extension(b"GIF89a"), "gif");
        assert_eq!(detect_extension(&[0x00, 0x01, 0x02]), "png"); // unknown → default
    }
}
