//! Ollama model detection and management.
//!
//! These are standalone utilities used by the setup wizard (hardware-tier-based
//! model recommendations) and the Tauri `list_models` / `missing_models` commands.
//! They are orthogonal to the health-check rewrite.

use std::process::Command;

use crate::hardware::ModelTier;

// ── Model lists ───────────────────────────────────────────────────────────────

/// Required models per tier.
///
/// Every tier provisions the embedded defaults the gateway's seed chains lead
/// with — `gemma2:2b` (in-process chat for the classify/summarize chains, 1.6GB)
/// and `all-minilm` (384-dim embeddings, 45MB). Both are pulled via Ollama; the
/// embedded `embedded-llama` adapter then reads the same blobs in-process (#79),
/// so no separate download or shipped GGUF is needed. The remaining per-tier
/// models back the reasoning chain + MOE panel as local fallbacks.
///
/// NOTE: the reasoning chain *leads* with `gemma4` (9.6GB), which is intentionally
/// NOT auto-pulled here — whether to provision it per hardware tier is a product
/// decision. Absent, the reasoning chain falls through to `gemma3:27b`.
pub fn required_models(tier: &ModelTier) -> Vec<&'static str> {
    match tier {
        ModelTier::Minimum => vec!["gemma2:2b", "all-minilm", "gemma3:12b"],
        ModelTier::Recommended => vec!["gemma2:2b", "all-minilm", "gemma3:27b", "qwen3:14b"],
        // TODO: add the full MOE panel models (phi-4:14b, llama4-scout:17b) once finalised.
        ModelTier::Full => vec!["gemma2:2b", "all-minilm", "gemma3:27b", "qwen3:14b"],
    }
}

// ── Ollama queries ────────────────────────────────────────────────────────────

/// List models currently available in Ollama.
pub fn list() -> Vec<String> {
    let output = Command::new("ollama").args(["list"]).output();

    match output {
        Ok(o) if o.status.success() => {
            String::from_utf8_lossy(&o.stdout)
                .lines()
                .skip(1) // skip header
                .filter_map(|line| {
                    let name = line.split_whitespace().next()?;
                    Some(name.to_string())
                })
                .collect()
        }
        _ => vec![],
    }
}

/// Check which required models are missing for the given tier.
pub fn missing_models(tier: &ModelTier) -> Vec<String> {
    let installed = list();
    let required = required_models(tier);

    required
        .into_iter()
        .filter(|model| {
            !installed.iter().any(|installed_name| {
                // Match "gemma3:27b" against "gemma3:27b" or "gemma3:27b-q4_0" etc.
                installed_name.starts_with(model)
            })
        })
        .map(|s| s.to_string())
        .collect()
}

/// Pull a model via ollama. Returns immediately — caller should poll for completion.
pub fn pull(model: &str) -> Result<std::process::Child, String> {
    Command::new("ollama")
        .args(["pull", model])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .map_err(|e| format!("failed to start ollama pull: {e}"))
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_models_minimum_includes_embedded_defaults() {
        let models = required_models(&ModelTier::Minimum);
        // Embedded chat + embed defaults are provisioned on every tier.
        assert!(models.contains(&"gemma2:2b"), "embedded chat default");
        assert!(models.contains(&"all-minilm"), "embed default");
        assert!(models.contains(&"gemma3:12b"), "minimum reasoning fallback");
    }

    #[test]
    fn required_models_recommended_includes_embedded_defaults_and_reasoning() {
        let models = required_models(&ModelTier::Recommended);
        assert!(models.contains(&"gemma2:2b"));
        assert!(models.contains(&"all-minilm"));
        assert!(models.contains(&"gemma3:27b"));
        assert!(models.contains(&"qwen3:14b"));
    }

    #[test]
    fn every_tier_provisions_the_embedded_defaults() {
        for tier in [ModelTier::Minimum, ModelTier::Recommended, ModelTier::Full] {
            let models = required_models(&tier);
            assert!(models.contains(&"gemma2:2b"), "{tier:?} must pull embedded chat");
            assert!(models.contains(&"all-minilm"), "{tier:?} must pull embed model");
        }
    }

    #[test]
    fn missing_models_when_nothing_installed() {
        // If ollama isn't running, list() returns empty → all required missing.
        let missing = missing_models(&ModelTier::Minimum);
        // Can't assert exact result (depends on whether ollama is running), but
        // the function shouldn't panic and can't exceed the required count.
        assert!(missing.len() <= required_models(&ModelTier::Minimum).len());
    }

    #[test]
    fn model_name_matching() {
        let installed = ["gemma3:27b".to_string(), "llama3:8b".to_string()];
        let model = "gemma3:27b";
        let found = installed.iter().any(|name| name.starts_with(model));
        assert!(found);
    }

    #[test]
    fn model_name_matching_not_found() {
        let installed = ["llama3:8b".to_string()];
        let model = "gemma3:27b";
        let found = installed.iter().any(|name| name.starts_with(model));
        assert!(!found);
    }
}
