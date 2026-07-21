//! On-demand Hugging Face model provisioning wiring.
//!
//! The gateway v0.4.0 release ships a library-owned [`ProvisioningSupervisor`]
//! (`local_engine`) that can pull a GGUF chat model from the Hugging Face Hub
//! and coldboot it behind the embedded llama.cpp router — so local chat works
//! without a pre-populated ollama/managed model. This module owns senseid's
//! side of that wiring:
//!
//! - [`provisioning_plans`] — the fixed catalog of what senseid can pull
//!   (currently one entry: `gemma2:2b`).
//! - [`build_supervisor`] — assemble a [`ProvisioningSupervisor`] over those
//!   plans, sharing the SAME `AdapterRegistry` + managed-store resolver that the
//!   `embedded-llama` adapter uses (see [`crate::api::gateway_init`]).
//!
//! Everything that constructs a [`ProvisionPlan::HfGguf`] or calls the
//! coldboot builders (`with_registry`/`with_resolver`/`with_puller`) is gated
//! behind `embedded-llama-cpp` — only that build enables `local-engine`'s
//! `llama-cpp` (+ `coldboot`, +`hf-download`) features. The
//! [`ProvisioningSupervisor`] type itself is available in every build (the
//! `local-engine` dep is non-optional), so the daemon can carry an
//! `Option<Arc<ProvisioningSupervisor>>` unconditionally and simply hold `None`
//! when built without the embedded engine.
//!
//! Provisioning is **on-demand only**: this module never calls `ensure`. The
//! supervisor is wired as the gateway's readiness probe at startup, but a pull
//! begins solely when the `POST /api/gateway/models/{id}/provision` handler
//! asks for it.

/// HF repo the on-demand embedded chat model is pulled from. Isolated as a
/// named constant (with the file below) so the exact repo/filename is trivial
/// to adjust if the published GGUF asset name changes upstream.
#[cfg(feature = "embedded-llama-cpp")]
pub(crate) const GEMMA2_2B_HF_REPO: &str = "bartowski/gemma-2-2b-it-GGUF";

/// GGUF file within [`GEMMA2_2B_HF_REPO`] to download. Q4_K_M is the balanced
/// quant (~1.7 GB) that fits a laptop while keeping usable chat quality.
#[cfg(feature = "embedded-llama-cpp")]
pub(crate) const GEMMA2_2B_HF_FILE: &str = "gemma-2-2b-it-Q4_K_M.gguf";

/// Stable model id the plan is keyed by. MUST match the baseline config's
/// embedded chat model id + its `api_model_id` (see
/// [`crate::api::gateway_init::baseline_production_config`]) so the gateway's
/// readiness probe degrades the embedded chat leg to `ModelNotReady` while this
/// model is being pulled, then serves it once `Ready`.
#[cfg(feature = "embedded-llama-cpp")]
pub(crate) const EMBEDDED_CHAT_MODEL_ID: &str = "gemma2:2b";

/// The fixed catalog of models senseid can provision on demand, keyed by model
/// id. Currently one entry: the embedded chat model pulled as a GGUF from HF.
///
/// Pure over its constants — unit-testable without a runtime, network, or the
/// llama.cpp backend.
#[cfg(feature = "embedded-llama-cpp")]
pub(crate) fn provisioning_plans(
) -> std::collections::HashMap<String, local_engine::ProvisionPlan> {
    use local_engine::registry::{ModelFormat, PullSpec};
    use local_engine::ProvisionPlan;

    let mut plans = std::collections::HashMap::new();
    plans.insert(
        EMBEDDED_CHAT_MODEL_ID.to_string(),
        ProvisionPlan::HfGguf {
            spec: PullSpec {
                repo: GEMMA2_2B_HF_REPO.to_string(),
                revision: None,
                id: EMBEDDED_CHAT_MODEL_ID.to_string(),
                name: Some("Gemma 2 2B Instruct".to_string()),
                format: ModelFormat::Gguf,
                files: vec![GEMMA2_2B_HF_FILE.to_string()],
            },
        },
    );
    plans
}

/// Build the on-demand provisioning supervisor, sharing the given adapter
/// registry and managed-store resolver.
///
/// The supervisor coldboots a pulled GGUF into an `EmbeddedLlamaAdapter` and
/// registers it into `adapters` (the SAME registry the gateway dispatches
/// through), and resolves/verifies bytes through `resolver` (the SAME
/// managed→ollama chain the `embedded-llama` adapter already uses). The HF
/// puller writes into `managed_dir` — the managed store `resolver`'s
/// `ManagedResolver` leg reads from — so a freshly-pulled file resolves for the
/// coldboot verify step.
///
/// `max_concurrent` is 1: a single embedded chat model, and concurrent large
/// GGUF pulls would only contend for disk/RAM.
#[cfg(feature = "embedded-llama-cpp")]
pub(crate) fn build_supervisor(
    adapters: gateway::adapters::AdapterRegistry,
    resolver: std::sync::Arc<dyn local_engine::registry::ModelResolver>,
    managed_dir: std::path::PathBuf,
) -> local_engine::ProvisioningSupervisor {
    use local_engine::registry::{HfHubPuller, ManagedResolver};
    use std::sync::Arc;

    local_engine::ProvisioningSupervisor::new(provisioning_plans(), 1)
        .with_registry(adapters)
        .with_resolver(resolver)
        .with_puller(Arc::new(HfHubPuller::new(
            ManagedResolver::new(managed_dir),
            // No HF token: the embedded chat model is a public repo. A gated /
            // private repo would need a token threaded through here.
            None,
        )))
}

#[cfg(all(test, feature = "embedded-llama-cpp"))]
mod tests {
    use super::*;
    use local_engine::registry::ModelFormat;
    use local_engine::ProvisionPlan;

    /// The catalog must carry the embedded chat model keyed by its resolvable id
    /// with the expected HF repo, file, and format. This is the contract the
    /// baseline config + readiness probe rely on (config id == plan id ==
    /// resolvable id); a drift here silently breaks on-demand chat.
    #[test]
    fn provisioning_plans_contains_embedded_chat_model() {
        let plans = provisioning_plans();
        let plan = plans
            .get("gemma2:2b")
            .expect("catalog must contain the embedded chat model 'gemma2:2b'");

        match plan {
            ProvisionPlan::HfGguf { spec } => {
                assert_eq!(spec.repo, "bartowski/gemma-2-2b-it-GGUF");
                assert_eq!(spec.id, "gemma2:2b");
                assert_eq!(spec.format, ModelFormat::Gguf);
                assert_eq!(
                    spec.files,
                    vec!["gemma-2-2b-it-Q4_K_M.gguf".to_string()],
                    "single GGUF file to pull"
                );
                assert_eq!(spec.name.as_deref(), Some("Gemma 2 2B Instruct"));
                assert!(spec.revision.is_none(), "revision defaults to main");
            }
            // `ProvisionPlan` is #[non_exhaustive] and does not derive Debug, so
            // the wildcard arm can't print the variant — a bare panic is enough.
            _ => panic!("expected HfGguf plan, got a different variant"),
        }
    }

    /// The catalog holds exactly the one on-demand model today — a guard so an
    /// accidental extra entry (which would auto-appear in status/CLI output)
    /// is a deliberate, reviewed change.
    #[test]
    fn provisioning_plans_has_exactly_one_entry() {
        assert_eq!(provisioning_plans().len(), 1);
    }
}
