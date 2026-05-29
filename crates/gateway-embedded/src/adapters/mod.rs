//! In-process inference adapters that implement
//! [`gateway::adapters::InferenceAdapter`].
//!
//! Each engine sits behind its own cargo feature so that callers only pay
//! the build cost (C++ toolchain for `llama-cpp-2`, ORT runtime download
//! for `ort`, etc.) for the engines they actually use.

#[cfg(feature = "llama-cpp")]
pub mod llama_cpp;

#[cfg(feature = "llama-cpp")]
pub use llama_cpp::LlamaCppAdapter;
