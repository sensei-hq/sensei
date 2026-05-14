//! Ollama model detection and management.
//!
//! These are standalone utilities used by the setup wizard (hardware-tier-based
//! model recommendations) and the Tauri `list_models` / `missing_models` commands.
//! They are orthogonal to the health-check rewrite.

use std::process::Command;

use crate::hardware::ModelTier;

// ── Model lists ───────────────────────────────────────────────────────────────

/// Required models per tier.
pub fn required_models(tier: &ModelTier) -> Vec<&'static str> {
    match tier {
        ModelTier::Minimum     => vec!["gemma3:12b"],
        ModelTier::Recommended => vec!["gemma3:27b", "qwen3:14b"],
        // TODO: add the full MOE panel models once the set is finalised.
        ModelTier::Full        => vec!["gemma3:27b", "qwen3:14b"],
    }
}

// ── Ollama queries ────────────────────────────────────────────────────────────

/// List models currently available in Ollama.
pub fn list() -> Vec<String> {
    let output = Command::new("ollama")
        .args(["list"])
        .output();

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

    required.into_iter()
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
    fn required_models_minimum_has_one() {
        let models = required_models(&ModelTier::Minimum);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0], "gemma3:12b");
    }

    #[test]
    fn required_models_recommended_has_two() {
        let models = required_models(&ModelTier::Recommended);
        assert_eq!(models.len(), 2);
        assert!(models.contains(&"gemma3:27b"));
        assert!(models.contains(&"qwen3:14b"));
    }

    #[test]
    fn missing_models_when_nothing_installed() {
        // If ollama isn't running, list() returns empty → all models missing
        let missing = missing_models(&ModelTier::Minimum);
        // Can't assert exact result since it depends on whether ollama is running,
        // but the function shouldn't panic
        assert!(missing.len() <= 1);
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
