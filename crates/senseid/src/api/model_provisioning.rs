//! On-demand model provisioning — a sensei-owned service (#79, gateway#5).
//!
//! The embedded chat model (`gemma2:2b`) is downloaded on demand so local chat
//! works without a pre-populated ollama/managed model. Downloads go through the
//! **Ollama registry over plain HTTP + sha256** ([`crate::model_provision`]),
//! NOT the gateway's Hugging Face puller: hf-hub 0.4.3 stalls on Xet-backed HF
//! repos (all modern GGUF repos — gateway#5), whereas the Ollama registry serves
//! the same model ids the rest of the gateway uses and its manifest digest
//! doubles as the sha256 to verify against.
//!
//! We own the provisioning service instead of the gateway's
//! [`ProvisioningSupervisor`] because the supervisor's `with_puller` takes a
//! concrete `HfHubPuller` (no seam to inject sensei's Ollama puller) and its
//! `ProvisionPlan` is `#[non_exhaustive]` (can't drive an Ollama download from
//! the daemon). [`ModelProvisioning`] mirrors the supervisor's proven shape — a
//! shared phase map, a concurrency cap, and the [`kernel::ReadinessProbe`] impl
//! — over sensei's [`Provisioner`](crate::model_provision::Provisioner).
//!
//! Wired as the gateway's readiness probe at startup (see
//! [`crate::api::gateway_init`]), so a chain leg whose model is still being
//! pulled degrades to a terminal `ModelNotReady { model, phase }` while
//! `gemma2:2b` downloads, then serves the instant the pull reaches `Ready`.
//! Once a model lands in the managed store, the already-registered
//! `embedded-llama` adapter lazy-loads it on the next inference request — a
//! completed download is servable with no explicit coldboot.
//!
//! Provisioning is **on-demand only**: nothing here or at startup auto-pulls; a
//! pull begins solely when the `POST /api/gateway/models/{id}/provision` handler
//! calls [`ModelProvisioning::ensure`].
//!
//! The service is only *constructed* in an `embedded-llama-cpp` build (it's
//! useful only when the embedded-llama adapter exists to serve the result — see
//! [`crate::api::gateway_init`]), but the [`ModelProvisioning`] type itself
//! compiles in every build: its deps ([`Provisioner`], `kernel::ReadinessProbe`,
//! [`gateway::ProvisionPhase`]) are all non-optional. So [`crate::api::state`]
//! can carry an `Option<Arc<ModelProvisioning>>` unconditionally and simply hold
//! `None` when built without the embedded engine — no per-call-site cfg on the
//! ~15 `SharedState` constructors.

// The type + its `ReadinessProbe` impl compile in every build so `SharedState`
// can carry `Option<Arc<ModelProvisioning>>` unconditionally, but the service is
// only CONSTRUCTED / driven in an `embedded-llama-cpp` build (`gateway_init` +
// the provision handlers, both feature-gated). In a non-embedded build the field
// is always `None`, so the constructor / catalog / status surface is genuinely
// unused there — suppress dead-code in THAT build only. The embedded build (the
// one that actually uses these) keeps the lint on, so real dead code still
// surfaces where it matters.
#![cfg_attr(not(feature = "embedded-llama-cpp"), allow(dead_code))]

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use gateway::ProvisionPhase;
use tokio::sync::Semaphore;

use crate::model_provision::Provisioner;

/// Sensei-owned on-demand model-provisioning service. Held as
/// `Arc<ModelProvisioning>` so the shared phase map is visible across the
/// spawned download job, the readiness probe, and the status endpoint.
///
/// The provisionable catalog is **config-derived** per instance (not a fixed
/// constant): [`crate::api::gateway_init`] scans the final [`GatewayConfig`]
/// for every chain leg routed through the `embedded-llama` adapter and hands
/// the resulting `(pullable_id, display_name)` pairs to [`Self::new`]. Because
/// the catalog is also what constrains a pull — [`Self::is_in_catalog`] gates
/// `ensure` at the handler — a `pull <id>` for an id that isn't a configured
/// local model is rejected rather than silently attempted.
///
/// [`GatewayConfig`]: gateway::types::config::GatewayConfig
pub struct ModelProvisioning {
    /// The Ollama-registry downloader, writing into the managed model dir.
    provisioner: Arc<Provisioner>,
    /// The provisionable catalog as flat `(id, display_name)` pairs — the models
    /// the configured `embedded-llama` legs run on. This is what the status
    /// endpoint (and the UI) list so a client can see every pullable model —
    /// with its current phase overlaid — before any pull has started, and what
    /// [`Self::is_in_catalog`] checks to constrain a pull to a configured model.
    catalog: Vec<(String, String)>,
    /// Live phase per model id. `std::sync::Mutex` (never held across `.await`).
    /// Shared (`Arc`) so the spawned job and the probe see the same map.
    phases: Arc<Mutex<HashMap<String, ProvisionPhase>>>,
    /// Caps concurrent pulls at 1 (concurrent large GGUF pulls would only
    /// contend for disk/RAM).
    sem: Arc<Semaphore>,
}

impl ModelProvisioning {
    /// Build the service over `managed_dir` — the same managed-store dir the
    /// `embedded-llama` adapter resolves from, so a completed download is
    /// immediately servable — with the config-derived `catalog` of
    /// `(pullable_id, display_name)` pairs it may provision.
    pub fn new(managed_dir: PathBuf, catalog: Vec<(String, String)>) -> Self {
        Self {
            provisioner: Arc::new(Provisioner::new(managed_dir)),
            catalog,
            phases: Arc::new(Mutex::new(HashMap::new())),
            sem: Arc::new(Semaphore::new(1)),
        }
    }

    /// The provisionable catalog as `(id, display_name)` pairs — the models the
    /// configured `embedded-llama` legs run on. The status handler reads this to
    /// build its rows (id + name) and overlays each entry's live phase.
    pub fn catalog(&self) -> &[(String, String)] {
        &self.catalog
    }

    /// Whether `id` is a provisionable model (present in the config-derived
    /// catalog). The provision handler gates `ensure` on this so a pull for an
    /// id that isn't a configured local model is rejected, not silently
    /// attempted against the Ollama registry.
    pub fn is_in_catalog(&self, id: &str) -> bool {
        self.catalog.iter().any(|(cid, _)| cid == id)
    }

    /// Current phase of `model`, or `Absent` if untracked. Locks only, no await
    /// held — shared body behind the inherent status and the readiness probe.
    fn phase_of(&self, model: &str) -> ProvisionPhase {
        self.phases
            .lock()
            .expect("provisioning phases mutex poisoned")
            .get(model)
            .cloned()
            .unwrap_or(ProvisionPhase::Absent)
    }

    /// Set `model`'s live phase.
    fn set_phase(&self, model: &str, phase: ProvisionPhase) {
        self.phases
            .lock()
            .expect("provisioning phases mutex poisoned")
            .insert(model.to_string(), phase);
    }

    /// Disk-aware phase of `model`: the in-memory tracked phase when a pull is
    /// live or finished THIS session; otherwise on-disk truth — `Ready` if the
    /// model is already provisioned in the managed store (pulled in a previous
    /// run, before a daemon restart cleared the in-memory map), else `Absent`.
    ///
    /// The map never stores `Absent` (`ensure` only sets Queued/Downloading/…/
    /// Ready/Failed), so a `phase_of` of `Absent` unambiguously means "untracked"
    /// → fall back to the managed store.
    async fn phase_for(&self, model: &str) -> ProvisionPhase {
        let tracked = self.phase_of(model);
        if !matches!(tracked, ProvisionPhase::Absent) {
            return tracked;
        }
        if self.provisioner.is_provisioned(model).await {
            ProvisionPhase::Ready
        } else {
            ProvisionPhase::Absent
        }
    }

    /// The provisionable catalog, each entry with its disk-aware phase. This is
    /// what the status endpoint lists — so a model pulled in a previous run shows
    /// `Ready` (not `Absent`) even before any `ensure` this session. The handler
    /// merges display names from [`Self::catalog`] on top.
    pub async fn status_all(&self) -> Vec<(String, ProvisionPhase)> {
        let mut out = Vec::new();
        for (id, _name) in &self.catalog {
            let phase = self.phase_for(id).await;
            out.push((id.clone(), phase));
        }
        out
    }

    /// Begin (or join) an on-demand pull of `model`, returning its current phase.
    /// Non-blocking, idempotent, deduped by id: if the model is `Ready` or a job
    /// is in flight, returns that phase unchanged; otherwise transitions to
    /// `Queued`, spawns a background download job, and returns `Queued`.
    ///
    /// The real download runs through the [`Provisioner`]; see [`Self::ensure_with`]
    /// for the injectable seam the unit tests drive without a network.
    pub fn ensure(self: &Arc<Self>, model: &str) -> ProvisionPhase {
        let provisioner = self.provisioner.clone();
        let model_owned = model.to_string();
        self.ensure_with(model, move |report| {
            let provisioner = provisioner.clone();
            let model_owned = model_owned.clone();
            async move {
                provisioner
                    .ensure_model_with_progress(&model_owned, |done, total| {
                        report(ProvisionPhase::Downloading { done, total });
                    })
                    .await
                    .map(|_| ())
            }
        })
    }

    /// Injectable core of [`Self::ensure`]. `download` is given a `report`
    /// callback (to push `Downloading{done,total}` progress into the shared phase
    /// map) and returns a future resolving to `Ok(())` on a completed download or
    /// `Err(msg)` on failure — no network dependency, so tests drive
    /// Downloading→Ready and Downloading→Failed deterministically.
    ///
    /// Dedup + spawn semantics (holding the phase lock only for the check-and-set,
    /// never across `.await`):
    /// - `Ready` or in-flight (`Queued`/`Downloading`/`Verifying`/`Loading`) →
    ///   return that phase, spawn nothing.
    /// - `Absent` or `Failed` → set `Queued`, spawn the job, return `Queued`.
    ///
    /// The job (holding a semaphore permit): set `Downloading{done:0,total:None}`,
    /// run `download` (progress → `Downloading{done,total}`), then `Ready` on
    /// success or `Failed{error}` on error. An `ensure_model` that finds the
    /// model already present returns fast, so `download` resolves `Ok` almost
    /// immediately → `Ready` (the idempotent short-circuit).
    pub(crate) fn ensure_with<F, Fut>(self: &Arc<Self>, model: &str, download: F) -> ProvisionPhase
    where
        F: FnOnce(ReportPhase) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = Result<(), String>> + Send + 'static,
    {
        {
            // Check-and-set under a single lock hold (no `.await` inside) so two
            // concurrent `ensure` calls for the same id can't both pass the dedup
            // check and spawn a job: the first flips the phase to `Queued`, the
            // second sees the in-flight phase and returns.
            let mut phases = self.phases.lock().expect("provisioning phases mutex poisoned");
            // `Ready` or in-flight → return that phase, spawn nothing (dedup).
            // `Absent`/`Failed` (or untracked) fall through to (re)start a job.
            if let Some(phase) = phases.get(model)
                && (matches!(phase, ProvisionPhase::Ready) || phase.is_in_flight())
            {
                return phase.clone();
            }
            phases.insert(model.to_string(), ProvisionPhase::Queued);
        }

        let this = self.clone();
        let model_owned = model.to_string();
        let sem = self.sem.clone();
        tokio::spawn(async move {
            // `acquire_owned` errs only if the semaphore is closed, which we
            // never do; the permit releases on return (and on panic).
            let _permit = sem.acquire_owned().await;
            this.set_phase(&model_owned, ProvisionPhase::Downloading { done: 0, total: None });

            // Progress reporter handed to `download`: pushes each transition into
            // the shared phase map so the readiness probe + status see live bytes.
            let report: ReportPhase = {
                let this = this.clone();
                let model_owned = model_owned.clone();
                Arc::new(move |phase: ProvisionPhase| this.set_phase(&model_owned, phase))
            };

            match download(report).await {
                Ok(()) => this.set_phase(&model_owned, ProvisionPhase::Ready),
                Err(error) => {
                    // Surface the failure on the phase map (a client polls status
                    // / the readiness probe for it); also log so an operator sees
                    // it without polling.
                    tracing::warn!(model = %model_owned, %error, "model provisioning failed");
                    this.set_phase(&model_owned, ProvisionPhase::Failed { error });
                }
            }
        });

        ProvisionPhase::Queued
    }
}

/// Callback the download job pushes phase transitions through (progress →
/// `Downloading{done,total}`). Shared (`Arc`) + `Send`/`Sync` so it can cross the
/// spawn boundary and be called from inside the download future.
pub(crate) type ReportPhase = Arc<dyn Fn(ProvisionPhase) + Send + Sync>;

/// The gateway consults readiness through this port; we answer from the live
/// phase map. A model still being pulled reports its in-flight phase, so a chain
/// leg keyed on it degrades to `ModelNotReady` rather than a generic failure.
///
/// `gateway::ReadinessProbe` is a re-export of `kernel::ReadinessProbe`, so this
/// impl satisfies the bound on [`gateway::Gateway::with_readiness`].
#[async_trait::async_trait]
impl gateway::ReadinessProbe for ModelProvisioning {
    async fn phase(&self, model: &str) -> ProvisionPhase {
        self.phase_for(model).await
    }

    async fn status_all(&self) -> Vec<(String, ProvisionPhase)> {
        // Fully-qualified to call the inherent (disk-aware catalog) method, not
        // recurse into this trait method.
        ModelProvisioning::status_all(self).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use tokio::sync::Notify;

    /// A one-entry test catalog matching the shipped embedded chat model, so the
    /// service tests exercise the same `(id, display_name)` shape the status
    /// endpoint + UI render without depending on the (config-derived) production
    /// catalog.
    fn test_catalog() -> Vec<(String, String)> {
        vec![("gemma2:2b".to_string(), "gemma2:2b".to_string())]
    }

    /// A service built with an explicit catalog exposes it verbatim through
    /// [`ModelProvisioning::catalog`] (what the status handler reads for id +
    /// display name).
    #[test]
    fn catalog_returns_the_configured_entries() {
        let svc = ModelProvisioning::new(std::env::temp_dir(), test_catalog());
        assert_eq!(
            svc.catalog(),
            &[("gemma2:2b".to_string(), "gemma2:2b".to_string())][..],
            "catalog is the config-derived list handed to `new`",
        );
    }

    /// `is_in_catalog` gates a pull: only ids in the config-derived catalog are
    /// provisionable; anything else is rejected by the handler with a 404.
    #[test]
    fn is_in_catalog_only_accepts_configured_ids() {
        let svc = ModelProvisioning::new(std::env::temp_dir(), test_catalog());
        assert!(svc.is_in_catalog("gemma2:2b"), "configured model is provisionable");
        assert!(!svc.is_in_catalog("llama3:70b"), "non-catalog id is rejected");
        assert!(!svc.is_in_catalog(""), "empty id is rejected");
    }

    /// A fresh service over an empty managed dir: `status_all` lists the catalog
    /// with each model `Absent` (nothing tracked, nothing on disk).
    #[tokio::test]
    async fn fresh_service_lists_catalog_as_absent() {
        let dir = tempfile::tempdir().unwrap();
        let svc = Arc::new(ModelProvisioning::new(dir.path().to_path_buf(), test_catalog()));
        assert_eq!(
            svc.status_all().await,
            vec![("gemma2:2b".to_string(), ProvisionPhase::Absent)],
            "status lists the catalog; fresh service has nothing on disk → Absent",
        );
        assert_eq!(svc.phase_of("gemma2:2b"), ProvisionPhase::Absent);
    }

    /// `phase_for` prefers the in-memory tracked phase, then falls back to on-disk
    /// truth so a model pulled in a PREVIOUS run (empty phase map after a restart)
    /// reports `Ready` rather than `Absent`.
    #[tokio::test]
    async fn phase_for_prefers_tracked_then_falls_back_to_disk() {
        use local_engine::registry::{ManagedResolver, ModelEntry, ModelFormat, ModelSource};

        let dir = tempfile::tempdir().unwrap();
        let svc = Arc::new(ModelProvisioning::new(dir.path().to_path_buf(), test_catalog()));

        // Untracked + empty managed store → Absent.
        assert_eq!(svc.phase_for("gemma2:2b").await, ProvisionPhase::Absent);

        // A tracked (in-flight) phase wins over disk.
        svc.set_phase("gemma2:2b", ProvisionPhase::Downloading { done: 1, total: Some(2) });
        assert_eq!(
            svc.phase_for("gemma2:2b").await,
            ProvisionPhase::Downloading { done: 1, total: Some(2) },
        );

        // Untracked but present on disk (registered + file) → Ready.
        let other = tempfile::tempdir().unwrap();
        let svc2 = Arc::new(ModelProvisioning::new(other.path().to_path_buf(), test_catalog()));
        let file = other.path().join("gemma2_2b.gguf");
        tokio::fs::write(&file, b"stub").await.unwrap();
        ManagedResolver::new(other.path())
            .add(ModelEntry {
                id: "gemma2:2b".into(),
                name: "Gemma 2 2B".into(),
                format: ModelFormat::Gguf,
                source: ModelSource::Managed { path: file },
                sha256: None,
                size_bytes: None,
            })
            .await
            .unwrap();
        assert_eq!(svc2.phase_for("gemma2:2b").await, ProvisionPhase::Ready);
    }

    /// `ensure` (via the injectable seam) transitions a fresh model
    /// Queued → Downloading{progress} → Ready, driving progress through the
    /// `report` callback without any network. Uses a gate so the assertions
    /// observe the intermediate `Downloading` state before completion.
    #[tokio::test]
    async fn ensure_with_drives_downloading_to_ready() {
        let svc = Arc::new(ModelProvisioning::new(std::env::temp_dir(), test_catalog()));
        let release = Arc::new(Notify::new());
        let release_job = release.clone();

        // First call: fresh (Absent) → returns Queued and spawns the job.
        let phase = svc.ensure_with("gemma2:2b", move |report| async move {
            // Report mid-download progress, then block until the test releases us.
            report(ProvisionPhase::Downloading { done: 42, total: Some(100) });
            release_job.notified().await;
            Ok(())
        });
        assert_eq!(phase, ProvisionPhase::Queued, "fresh ensure returns Queued");

        // Wait for the job to publish Downloading progress.
        wait_until(&svc, "gemma2:2b", |p| {
            matches!(p, ProvisionPhase::Downloading { done: 42, total: Some(100) })
        })
        .await;

        // A concurrent ensure while downloading joins the in-flight job (no new
        // job) and returns the live Downloading phase unchanged.
        let joined = svc.ensure_with("gemma2:2b", |_report| async {
            panic!("in-flight ensure must NOT spawn a second download job");
        });
        assert_eq!(
            joined,
            ProvisionPhase::Downloading { done: 42, total: Some(100) },
            "in-flight ensure returns the live phase, deduped",
        );

        // Let the job finish → Ready.
        release.notify_one();
        wait_until(&svc, "gemma2:2b", |p| matches!(p, ProvisionPhase::Ready)).await;

        // An ensure on a Ready model is a no-op that returns Ready.
        let already = svc.ensure_with("gemma2:2b", |_report| async {
            panic!("ensure on a Ready model must NOT spawn a download job");
        });
        assert_eq!(already, ProvisionPhase::Ready);
    }

    /// A failing download transitions Queued → Downloading → Failed{error}, and a
    /// subsequent `ensure` RESTARTS the job (Failed is not terminal for retry).
    #[tokio::test]
    async fn ensure_with_drives_downloading_to_failed_then_retries() {
        let svc = Arc::new(ModelProvisioning::new(std::env::temp_dir(), test_catalog()));

        let phase = svc
            .ensure_with("gemma2:2b", |_report| async { Err("registry unreachable".to_string()) });
        assert_eq!(phase, ProvisionPhase::Queued);

        wait_until(&svc, "gemma2:2b", |p| matches!(p, ProvisionPhase::Failed { .. })).await;
        assert_eq!(
            svc.phase_of("gemma2:2b"),
            ProvisionPhase::Failed { error: "registry unreachable".to_string() },
        );

        // Failed → a fresh ensure restarts provisioning (returns Queued, spawns
        // a new job) rather than being stuck on the old failure.
        let ran = Arc::new(AtomicBool::new(false));
        let ran_job = ran.clone();
        let retry = svc.ensure_with("gemma2:2b", move |_report| async move {
            ran_job.store(true, Ordering::SeqCst);
            Ok(())
        });
        assert_eq!(retry, ProvisionPhase::Queued, "Failed model re-queues on ensure");
        wait_until(&svc, "gemma2:2b", |p| matches!(p, ProvisionPhase::Ready)).await;
        assert!(ran.load(Ordering::SeqCst), "retry job actually ran");
    }

    /// The readiness probe answers from the same live phase map: a model mid-pull
    /// reports its in-flight phase (so a chain leg degrades to ModelNotReady), an
    /// untracked model reports Absent, and `status_all` mirrors the map.
    #[tokio::test]
    async fn readiness_probe_reflects_live_phases() {
        use gateway::ReadinessProbe;
        let dir = tempfile::tempdir().unwrap();
        let svc = Arc::new(ModelProvisioning::new(dir.path().to_path_buf(), test_catalog()));
        let probe: Arc<dyn ReadinessProbe> = svc.clone();

        assert_eq!(probe.phase("gemma2:2b").await, ProvisionPhase::Absent);

        let release = Arc::new(Notify::new());
        let release_job = release.clone();
        svc.ensure_with("gemma2:2b", move |report| async move {
            report(ProvisionPhase::Downloading { done: 1, total: Some(10) });
            release_job.notified().await;
            Ok(())
        });
        wait_until(&svc, "gemma2:2b", |p| matches!(p, ProvisionPhase::Downloading { .. })).await;

        assert_eq!(
            probe.phase("gemma2:2b").await,
            ProvisionPhase::Downloading { done: 1, total: Some(10) },
            "probe reflects the in-flight download phase",
        );
        let all = probe.status_all().await;
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].0, "gemma2:2b");

        release.notify_one();
    }

    /// Poll the live phase map until `pred` holds, with a bounded timeout so a
    /// hung job fails the test instead of hanging forever.
    async fn wait_until(
        svc: &Arc<ModelProvisioning>,
        model: &str,
        pred: impl Fn(&ProvisionPhase) -> bool,
    ) {
        for _ in 0..200 {
            if pred(&svc.phase_of(model)) {
                return;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        panic!(
            "phase for '{model}' never satisfied the predicate (last: {:?})",
            svc.phase_of(model)
        );
    }
}
