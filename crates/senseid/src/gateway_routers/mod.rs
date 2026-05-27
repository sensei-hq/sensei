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
        providers: &["ollama"],
        capabilities: &["text_chat", "text_complete", "text_embed"],
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
    fn every_router_has_at_least_one_provider() {
        for r in REGISTRY {
            assert!(!r.providers.is_empty(), "{} has no providers", r.id);
        }
    }
}
