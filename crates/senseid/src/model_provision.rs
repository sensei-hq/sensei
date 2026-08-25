//! Ollama-free model provisioning (#79).
//!
//! Pulls model GGUFs from the **Ollama registry over plain HTTP** — no Ollama
//! daemon or CLI required — into sensei's managed model dir, then registers
//! them so the `embedded-llama` adapter resolves them via the
//! [`ManagedResolver`]. This removes the Ollama *runtime* dependency for the
//! embedded path: Ollama becomes one optional model source among several, not
//! a prerequisite.
//!
//! Why the Ollama registry (vs HuggingFace): it serves the **same model ids**
//! the rest of the gateway already uses (`gemma2:2b`, `all-minilm`, …) and its
//! manifests carry the blob **digest**, which doubles as the sha256 to verify
//! against — so there are no hand-maintained URLs or hashes to drift.
//!
//! The pure parse helpers ([`split_id`], [`model_layer_from_manifest`],
//! [`managed_filename`]) are unit-tested; [`Provisioner::ensure_model`] does
//! the network + filesystem work and is covered by an `#[ignore]` integration
//! test against the live registry.

use std::path::{Path, PathBuf};

use local_engine::registry::{
    ManagedResolver, ModelEntry, ModelFormat, ModelResolver, ModelSource,
};
use sha2::{Digest, Sha256};
use tokio::io::AsyncWriteExt;

/// Base URL for the default (`library`) namespace of the Ollama registry.
const REGISTRY_LIBRARY: &str = "https://registry.ollama.ai/v2/library";
/// Media type of the layer that carries the model weights.
const MODEL_MEDIA_TYPE: &str = "application/vnd.ollama.image.model";
/// Manifest content type the registry expects in `Accept`.
const MANIFEST_ACCEPT: &str = "application/vnd.docker.distribution.manifest.v2+json";

/// Split an Ollama-style id `"name[:tag]"` into `(name, tag)`, defaulting the
/// tag to `latest`. Mirrors the id convention the [`OllamaResolver`] produces.
pub(crate) fn split_id(id: &str) -> (&str, &str) {
    match id.split_once(':') {
        Some((name, tag)) => (name, tag),
        None => (id, "latest"),
    }
}

/// Find the model-weights layer in a parsed Ollama manifest, returning its
/// `(digest, size_bytes)`. Returns `None` if there's no model layer.
pub(crate) fn model_layer_from_manifest(manifest: &serde_json::Value) -> Option<(String, u64)> {
    manifest
        .get("layers")?
        .as_array()?
        .iter()
        .find(|l| l.get("mediaType").and_then(|m| m.as_str()) == Some(MODEL_MEDIA_TYPE))
        .and_then(|l| {
            let digest = l.get("digest")?.as_str()?.to_string();
            let size = l.get("size").and_then(|s| s.as_u64()).unwrap_or(0);
            Some((digest, size))
        })
}

/// Derive a stable managed filename for a model id (`:` and `/` are not
/// path-safe across the id space). `gemma2:2b` → `gemma2_2b.gguf`.
pub(crate) fn managed_filename(id: &str) -> String {
    let safe: String = id.chars().map(|c| if c == ':' || c == '/' { '_' } else { c }).collect();
    format!("{safe}.gguf")
}

/// Provisions models from the Ollama registry into a managed root directory.
pub struct Provisioner {
    client: reqwest::Client,
    managed_root: PathBuf,
}

impl Provisioner {
    pub fn new(managed_root: impl Into<PathBuf>) -> Self {
        Self { client: reqwest::Client::new(), managed_root: managed_root.into() }
    }

    /// Ensure model `id` is present in the managed dir and registered in the
    /// managed index. No-op (returns the existing path) if it's already
    /// registered and on disk. Otherwise pulls it from the Ollama registry,
    /// verifying the blob against its manifest digest.
    ///
    /// Thin wrapper over [`Self::ensure_model_with_progress`] with a no-op
    /// progress callback — one download implementation, two entry points (DRY).
    pub async fn ensure_model(&self, id: &str) -> Result<PathBuf, String> {
        self.ensure_model_with_progress(id, |_done, _total| {}).await
    }

    /// The on-disk path of model `id` if it's already provisioned — registered
    /// in the managed index AND its file present. `None` otherwise. The shared
    /// presence check behind [`Self::ensure_model_with_progress`]'s idempotent
    /// early-return and [`Self::is_provisioned`].
    async fn present_path(&self, id: &str) -> Option<PathBuf> {
        let managed = ManagedResolver::new(&self.managed_root);
        match managed.resolve(id).await {
            Ok(Some(entry)) if entry.source.path().exists() => {
                Some(entry.source.path().to_path_buf())
            }
            _ => None,
        }
    }

    /// Whether model `id` is already provisioned (registered + on disk). Lets the
    /// provisioning service report `Ready` for a model pulled in a PREVIOUS run:
    /// the in-memory phase map is empty after a daemon restart, but the managed
    /// store persists, so readiness/status must fall back to on-disk truth.
    pub async fn is_provisioned(&self, id: &str) -> bool {
        self.present_path(id).await.is_some()
    }

    /// Same as [`Self::ensure_model`] but reports download progress: `on_progress`
    /// is called with `(bytes_written, total)` as the blob streams, where `total`
    /// is the model layer's declared `size` from the manifest (`None` only if the
    /// manifest omits it). Called once per streamed chunk; not called at all when
    /// the model is already present (the early-return short-circuit).
    ///
    /// The callback is `FnMut` so a caller can drive a live phase/progress
    /// display; it must not block (it runs inline on the download task).
    pub async fn ensure_model_with_progress(
        &self,
        id: &str,
        mut on_progress: impl FnMut(u64, Option<u64>),
    ) -> Result<PathBuf, String> {
        if let Some(path) = self.present_path(id).await {
            return Ok(path);
        }

        let (name, tag) = split_id(id);
        let manifest_url = format!("{REGISTRY_LIBRARY}/{name}/manifests/{tag}");
        let manifest: serde_json::Value = self
            .client
            .get(&manifest_url)
            .header("Accept", MANIFEST_ACCEPT)
            .send()
            .await
            .map_err(|e| format!("manifest fetch {manifest_url}: {e}"))?
            .error_for_status()
            .map_err(|e| format!("manifest {manifest_url}: {e}"))?
            .json()
            .await
            .map_err(|e| format!("manifest parse {manifest_url}: {e}"))?;

        let (digest, size) = model_layer_from_manifest(&manifest)
            .ok_or_else(|| format!("no model layer in manifest for '{id}'"))?;
        let expected_hex = digest.trim_start_matches("sha256:").to_string();
        // The manifest layer carries the total size; `0` means "not declared",
        // in which case we report an unknown total rather than a bogus `Some(0)`.
        let total = (size > 0).then_some(size);

        tokio::fs::create_dir_all(&self.managed_root)
            .await
            .map_err(|e| format!("create managed dir: {e}"))?;
        let dest = self.managed_root.join(managed_filename(id));
        let blob_url = format!("{REGISTRY_LIBRARY}/{name}/blobs/{digest}");
        self.download_verify(&blob_url, &dest, &expected_hex, |done| on_progress(done, total))
            .await?;

        let managed = ManagedResolver::new(&self.managed_root);
        managed
            .add(ModelEntry {
                id: id.to_string(),
                name: id.to_string(),
                format: ModelFormat::Gguf,
                source: ModelSource::Managed { path: dest.clone() },
                sha256: Some(expected_hex),
                size_bytes: Some(size),
            })
            .await
            .map_err(|e| format!("register '{id}' in managed index: {e}"))?;

        Ok(dest)
    }

    /// Stream `url` to `dest`, hashing as we go, and fail (removing the partial
    /// file) if the sha256 doesn't match `expected_hex`. Downloads to a
    /// `.partial` sidecar first, then renames — so an interrupted download
    /// never leaves a truncated file at the final path.
    ///
    /// `on_progress(bytes_written_so_far)` is invoked after each streamed chunk
    /// with the running total of bytes written, so a caller can surface download
    /// progress.
    async fn download_verify(
        &self,
        url: &str,
        dest: &Path,
        expected_hex: &str,
        mut on_progress: impl FnMut(u64),
    ) -> Result<(), String> {
        let tmp = dest.with_extension("gguf.partial");
        let mut resp = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("blob fetch {url}: {e}"))?
            .error_for_status()
            .map_err(|e| format!("blob {url}: {e}"))?;

        let mut file =
            tokio::fs::File::create(&tmp).await.map_err(|e| format!("create {tmp:?}: {e}"))?;
        let mut hasher = Sha256::new();
        let mut written: u64 = 0;
        while let Some(chunk) = resp.chunk().await.map_err(|e| format!("download {url}: {e}"))? {
            hasher.update(&chunk);
            file.write_all(&chunk).await.map_err(|e| format!("write {tmp:?}: {e}"))?;
            written += chunk.len() as u64;
            on_progress(written);
        }
        file.flush().await.map_err(|e| format!("flush {tmp:?}: {e}"))?;
        drop(file);

        let got = hex::encode(hasher.finalize());
        if got != expected_hex {
            // Best-effort cleanup: the checksum-failed download is
            // corrupt, so leaving it on disk risks a next-run pick-up.
            // If cleanup itself fails we're not in a worse state than
            // pre-download, but log so an operator can see the temp
            // leaked.
            if let Err(e) = tokio::fs::remove_file(&tmp).await {
                tracing::debug!(tmp = ?tmp, error = %e, "model_provision: failed to remove checksum-mismatched temp file");
            }
            return Err(format!("sha256 mismatch for {url}: expected {expected_hex}, got {got}"));
        }
        tokio::fs::rename(&tmp, dest)
            .await
            .map_err(|e| format!("rename {tmp:?} → {dest:?}: {e}"))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn split_id_defaults_tag_to_latest() {
        assert_eq!(split_id("all-minilm"), ("all-minilm", "latest"));
        assert_eq!(split_id("gemma2:2b"), ("gemma2", "2b"));
        assert_eq!(split_id("gemma4:latest"), ("gemma4", "latest"));
    }

    #[test]
    fn model_layer_from_manifest_picks_the_model_media_type() {
        let manifest = json!({
            "layers": [
                {"mediaType": "application/vnd.ollama.image.license", "digest": "sha256:lic", "size": 11},
                {"mediaType": MODEL_MEDIA_TYPE, "digest": "sha256:797b70c4", "size": 45949216},
                {"mediaType": "application/vnd.ollama.image.params", "digest": "sha256:par", "size": 16},
            ]
        });
        let (digest, size) = model_layer_from_manifest(&manifest).expect("model layer");
        assert_eq!(digest, "sha256:797b70c4");
        assert_eq!(size, 45949216);
    }

    #[test]
    fn model_layer_from_manifest_none_when_absent() {
        let manifest = json!({"layers": [
            {"mediaType": "application/vnd.ollama.image.license", "digest": "sha256:x", "size": 1}
        ]});
        assert!(model_layer_from_manifest(&manifest).is_none());
        assert!(model_layer_from_manifest(&json!({})).is_none());
    }

    #[test]
    fn managed_filename_is_path_safe() {
        assert_eq!(managed_filename("all-minilm"), "all-minilm.gguf");
        assert_eq!(managed_filename("gemma2:2b"), "gemma2_2b.gguf");
        assert_eq!(managed_filename("library/foo:bar"), "library_foo_bar.gguf");
    }

    /// `is_provisioned` is true only when the model is BOTH registered in the
    /// managed index AND its file is on disk — the on-disk truth the provisioning
    /// service falls back to when its in-memory phase map is empty (post-restart).
    #[tokio::test]
    async fn is_provisioned_reflects_managed_store() {
        let dir = tempfile::tempdir().unwrap();
        let prov = Provisioner::new(dir.path());

        // Nothing registered → not provisioned.
        assert!(!prov.is_provisioned("gemma2:2b").await);

        // Register the model + create its file → provisioned.
        let path = dir.path().join(managed_filename("gemma2:2b"));
        tokio::fs::write(&path, b"stub gguf").await.unwrap();
        ManagedResolver::new(dir.path())
            .add(ModelEntry {
                id: "gemma2:2b".to_string(),
                name: "Gemma 2 2B".to_string(),
                format: ModelFormat::Gguf,
                source: ModelSource::Managed { path: path.clone() },
                sha256: None,
                size_bytes: None,
            })
            .await
            .unwrap();
        assert!(prov.is_provisioned("gemma2:2b").await);

        // Registered but the file was removed → not provisioned (the presence
        // check verifies the file exists, not just the index entry).
        tokio::fs::remove_file(&path).await.unwrap();
        assert!(!prov.is_provisioned("gemma2:2b").await);
    }

    /// End-to-end against the live Ollama registry: pull the tiny all-minilm
    /// model (~45MB) into a temp managed dir, verify the digest, and confirm
    /// it's registered + resolvable. Ignored by default (network + 45MB).
    ///   cargo test -p senseid model_provision::tests::ensure_model -- --ignored
    #[tokio::test]
    #[ignore = "network: pulls all-minilm (~45MB) from registry.ollama.ai"]
    async fn ensure_model_pulls_and_registers_from_registry() {
        let dir = tempfile::tempdir().unwrap();
        let prov = Provisioner::new(dir.path());

        // Pull with progress: the callback must observe a monotonic byte count
        // and a stable `Some(total)` from the manifest layer size.
        let mut last_done = 0u64;
        let mut last_total: Option<u64> = None;
        let mut called = false;
        let path = prov
            .ensure_model_with_progress("all-minilm", |done, total| {
                assert!(done >= last_done, "progress bytes must be monotonic");
                last_done = done;
                last_total = total;
                called = true;
            })
            .await
            .expect("ensure_model_with_progress");
        assert!(called, "progress callback fired at least once");
        assert!(last_done > 0, "reported non-zero bytes downloaded");
        assert!(last_total.is_some(), "manifest declared a total size");
        assert_eq!(last_done, last_total.unwrap(), "final bytes == declared total");
        assert!(path.exists(), "gguf downloaded to {path:?}");
        assert!(path.starts_with(dir.path()));

        // Registered + resolvable via the managed index.
        let managed = ManagedResolver::new(dir.path());
        let entry = managed.resolve("all-minilm").await.unwrap().expect("registered");
        assert!(entry.sha256.is_some());
        assert_eq!(entry.source.path(), path);

        // Idempotent: a second call returns the same path without re-downloading
        // — and, being an early-return, fires no progress callback.
        let mut progressed = false;
        let again = prov
            .ensure_model_with_progress("all-minilm", |_, _| progressed = true)
            .await
            .expect("idempotent");
        assert_eq!(again, path);
        assert!(!progressed, "already-present model short-circuits without downloading");

        // The plain `ensure_model` wrapper still works (no-op progress).
        let again2 = prov.ensure_model("all-minilm").await.expect("idempotent wrapper");
        assert_eq!(again2, path);
    }
}
