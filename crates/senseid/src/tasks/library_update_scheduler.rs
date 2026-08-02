//! Library-update scheduler (workstream F, v0 — detect + notify). Mirrors
//! [`crate::tasks::log_pruner`]. A daily tick resolves each referenced library's
//! latest published version, compares it to every project's pinned `version_used`
//! via the pure policy core, and on a real bump writes a `library_update`
//! recommendation — the EXISTING Insights/Observatory surface (no new channel).
//!
//! FAIL-CLOSED: any fetch/parse miss (or a range pin that can't be compared) skips +
//! logs and never fabricates a "newer version available". No apply, no DDL, no worker
//! task in v0 — detect+notify runs inline in the tick.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use crate::db::pg_store::PgStore;
use crate::libraries::advisory::{security_verdict, Advisory, OsvVulnSource, VulnSource};
use crate::libraries::registry::{HttpVersionSource, VersionSource};
use crate::libraries::version::{classify_bump, update_action, Bump, UpdateAction};
use super::queue::TaskQueue;
use super::{Task, TaskKind};

/// True iff the docs still need a re-index for `latest` — the applied marker is
/// absent or stale. Equal marker ⇒ `index_library` already stamped a confirmed,
/// non-empty re-index at `latest`, so the scheduler skips (and records the
/// auto-applied audit instead of re-enqueuing).
fn should_reindex(applied: Option<&str>, latest: &str) -> bool {
    applied != Some(latest)
}

/// One project × library update fact, shared by the notify writers.
struct PendingUpdate<'a> {
    project_id: &'a uuid::Uuid,
    lib_id: &'a uuid::Uuid,
    name: &'a str,
    ecosystem: &'a str,
    from: &'a str,
    to: &'a str,
    bump_str: &'a str,
    /// The advisory id driving a `security` flag (v2), for the payload. `None` otherwise.
    advisory: Option<&'a str>,
}

/// Write a `library_update` recommendation for `pu` at the given `mode`
/// (`notify` | `auto_applied` | `notify_no_source` | `security`), deduped per
/// (project, library, to_version) at the `is_security` tier — so a prior
/// non-security notify never suppresses a security flag. The single writer onto
/// the Insights surface — no new channel.
async fn write_update_rec(pg: &PgStore, pu: &PendingUpdate<'_>, mode: &str, urgency: &str, is_security: bool) {
    match pg.pending_library_update_exists(pu.project_id, pu.lib_id, pu.to, is_security).await {
        Ok(true) => return, // already flagged this exact update at this tier
        Ok(false) => {}
        Err(e) => {
            tracing::warn!(error = %e, "library_update_scheduler: dedup check failed — skip");
            return;
        }
    }
    let title = match mode {
        "security" => format!("Security: update {} {} → {}", pu.name, pu.from, pu.to),
        _ => format!("Update {} {} → {} ({})", pu.name, pu.from, pu.to, pu.bump_str),
    };
    let why = match mode {
        "security" => format!("A known {} vulnerability in {} {} is fixed by {} — docs/skills refreshed; update your dependency to apply the fix.", pu.ecosystem, pu.name, pu.from, pu.to),
        "auto_applied" => format!("Auto-refreshed {} docs/skills for {} to {}.", pu.ecosystem, pu.name, pu.to),
        "notify_no_source" => format!("A newer {} version of {} is available (no indexable source to auto-refresh).", pu.ecosystem, pu.name),
        _ => format!("A newer {} version of {} is available.", pu.ecosystem, pu.name),
    };
    let based_on = serde_json::json!({ "library_update": {
        "library_id": pu.lib_id.to_string(), "ecosystem": pu.ecosystem, "name": pu.name,
        "from_version": pu.from, "to_version": pu.to, "bump": pu.bump_str,
        "mode": mode, "is_security": is_security, "advisory": pu.advisory,
    }});
    if let Err(e) = pg.create_recommendation_full(pu.project_id, &title, &why, None, "library_update", urgency, &based_on, None, None).await {
        tracing::warn!(error = %e, lib = %pu.name, "library_update_scheduler: create recommendation failed");
    }
}

/// Enqueue an `IndexLibrary` re-index of the library's OWN docs/skills (never the
/// consuming project's code) when it's stale — guarded by `should_reindex` (the
/// props marker), a within-tick set, and a lib-id-keyed in-flight guard. Shared by
/// the patch (v1a) and security (v2) apply paths.
async fn maybe_enqueue_reindex(
    pg: &PgStore, queue: &TaskQueue, enqueued: &mut HashSet<uuid::Uuid>,
    lib_id: &uuid::Uuid, name: &str, latest: &str, url: &str,
) {
    let applied = pg.get_library_docs_applied(lib_id).await.ok().flatten();
    if !should_reindex(applied.as_deref(), latest) {
        return;
    }
    if enqueued.contains(lib_id)
        || queue.has_pending_kind_folder(TaskKind::IndexLibrary, &lib_id.to_string()).await
    {
        return;
    }
    queue.enqueue(Task::new(TaskKind::IndexLibrary, &lib_id.to_string(), name).with_url(url)).await;
    enqueued.insert(*lib_id);
    tracing::info!(lib = %name, %latest, "library_update_scheduler: enqueued docs re-index");
}

const DEFAULT_INTERVAL_SECS: u64 = 86_400; // daily
const DEFAULT_CHECK_TTL_SECS: i64 = 82_800; // ~23h — reuse a lib's cached latest if newer

fn parse_interval(cfg: Option<String>) -> u64 {
    cfg.and_then(|s| s.trim().parse::<u64>().ok()).filter(|n| *n > 0).unwrap_or(DEFAULT_INTERVAL_SECS)
}
fn parse_ttl(cfg: Option<String>) -> i64 {
    cfg.and_then(|s| s.trim().parse::<i64>().ok()).filter(|n| *n > 0).unwrap_or(DEFAULT_CHECK_TTL_SECS)
}

/// Spawn the scheduler for the daemon's lifetime. `queue` lets the apply arm
/// (F v1, step 3) enqueue an `IndexLibrary` re-index; threaded now like the
/// sibling schedulers (`reconcile_scheduler::spawn(queue, pg)`).
pub fn spawn(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    tokio::spawn(run(queue, pg));
}

async fn run(queue: Arc<TaskQueue>, pg: Arc<PgStore>) {
    let secs = parse_interval(pg.get_config("library.update_interval_secs").await.ok().flatten());
    let mut ticker = tokio::time::interval(Duration::from_secs(secs));
    let src = HttpVersionSource;
    let vuln = OsvVulnSource;
    loop {
        ticker.tick().await; // first tick fires immediately → a boot pass
        tick(&pg, &queue, &src, &vuln).await;
    }
}

/// One pass. Testable with a stub [`VersionSource`]. Never panics; every failure is
/// skip + log (fail-closed).
pub(crate) async fn tick(pg: &PgStore, queue: &TaskQueue, src: &impl VersionSource, vuln: &impl VulnSource) {
    let ttl = parse_ttl(pg.get_config("library.check_ttl_secs").await.ok().flatten());
    let now = chrono::Utc::now().timestamp();

    let pins = match pg.list_library_project_pins().await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "library_update_scheduler: list pins failed");
            return;
        }
    };

    // Resolve the latest version once per DISTINCT library, TTL-gated + props-cached.
    let mut latest_by_lib: HashMap<uuid::Uuid, Option<String>> = HashMap::new();
    for (lib_id, name, ecosystem, local_path, _pid, _vu, _burl, _stype) in &pins {
        if latest_by_lib.contains_key(lib_id) {
            continue;
        }
        let cached = pg.get_library_latest_cache(lib_id).await.ok().flatten();
        let fresh = cached.as_ref().filter(|(_, checked)| now - checked < ttl).map(|(v, _)| v.clone());
        let latest = match fresh {
            Some(v) => Some(v), // within TTL — reuse cache, no network
            None => match src.latest(ecosystem, name, local_path.as_deref()).await {
                Some(v) => {
                    if let Err(e) = pg.set_library_latest_cache(lib_id, &v, now).await {
                        tracing::warn!(error = %e, lib = %name, "library_update_scheduler: cache write failed");
                    }
                    Some(v)
                }
                None => {
                    tracing::debug!(lib = %name, "library_update_scheduler: no latest resolved — skip (fail-closed)");
                    None
                }
            },
        };
        latest_by_lib.insert(*lib_id, latest);
    }

    // Dispatch each project pin by the policy action. PATCH auto-applies (re-index
    // sensei's OWN docs/skills — never the user's code); a KNOWN high-severity
    // advisory fixed by the upgrade escalates ANY real bump to a security refresh +
    // high-urgency flag; other MINOR/MAJOR bumps notify.
    let mut enqueued: HashSet<uuid::Uuid> = HashSet::new();
    let mut adv_by_lib: HashMap<uuid::Uuid, Vec<Advisory>> = HashMap::new();
    for (lib_id, name, ecosystem, local_path, project_id, version_used, base_url, _stype) in &pins {
        let Some(Some(latest)) = latest_by_lib.get(lib_id) else { continue };
        let bump = classify_bump(version_used, latest);
        if bump == Bump::None || bump == Bump::Unknown {
            continue; // no real, comparable bump — nothing to act on (fail-closed)
        }

        // Resolve the advisory set once per library-with-a-real-bump (fail-closed:
        // None → empty → no security flag), then recompute the verdict PER PIN.
        if !adv_by_lib.contains_key(lib_id) {
            let advs = vuln.advisories(ecosystem, name).await.unwrap_or_default();
            adv_by_lib.insert(*lib_id, advs);
        }
        let verdict = security_verdict(&adv_by_lib[lib_id], version_used, latest);
        let is_security = verdict.is_security;
        let action = update_action(bump, is_security);

        let bump_str = match bump {
            Bump::Patch => "patch",
            Bump::Minor => "minor",
            Bump::Major => "major",
            _ => "update",
        };
        let pu = PendingUpdate {
            project_id, lib_id, name, ecosystem, from: version_used, to: latest, bump_str,
            advisory: verdict.top.as_deref(),
        };

        // Security (v2): ALWAYS surface a high-urgency flag (even with no source —
        // the flag is the user-critical output), and best-effort refresh docs/skills.
        // NEVER touches the consuming project's code/pins.
        if is_security {
            write_update_rec(pg, &pu, "security", "high", true).await;
            if let Some(url) = base_url.clone().or_else(|| local_path.clone()) {
                maybe_enqueue_reindex(pg, queue, &mut enqueued, lib_id, name, latest, &url).await;
            }
            continue;
        }

        // PATCH auto-apply (v1a, non-security).
        if action == UpdateAction::AutoApply {
            let applied = pg.get_library_docs_applied(lib_id).await.ok().flatten();
            if !should_reindex(applied.as_deref(), latest) {
                // The marker has caught up ⇒ index_library confirmed the re-index.
                write_update_rec(pg, &pu, "auto_applied", "low", false).await;
                continue;
            }
            match base_url.clone().or_else(|| local_path.clone()) {
                // Needs a refresh; the audit is written on a LATER tick once the
                // marker flips — never at enqueue.
                Some(url) => maybe_enqueue_reindex(pg, queue, &mut enqueued, lib_id, name, latest, &url).await,
                // No indexable source → surface a notify (never fabricate a url).
                None => write_update_rec(pg, &pu, "notify_no_source", "low", false).await,
            }
            continue;
        }

        // MINOR / MAJOR → notify (v1 conservative; the compat gate is unbuilt).
        let urgency = if bump == Bump::Major { "medium" } else { "low" };
        write_update_rec(pg, &pu, "notify", urgency, false).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::pg_store::PgStore;

    struct Stub(Option<String>);
    #[async_trait::async_trait]
    impl VersionSource for Stub {
        async fn latest(&self, _e: &str, _n: &str, _l: Option<&str>) -> Option<String> {
            self.0.clone()
        }
    }

    struct StubVuln(Vec<Advisory>);
    #[async_trait::async_trait]
    impl VulnSource for StubVuln {
        async fn advisories(&self, _e: &str, _n: &str) -> Option<Vec<Advisory>> {
            Some(self.0.clone())
        }
    }
    /// No advisories — the default for the non-security paths.
    fn no_vulns() -> StubVuln {
        StubVuln(vec![])
    }
    /// One HIGH advisory affecting from the beginning, fixed at `fixed`.
    fn high_fixed_at(fixed: &str) -> StubVuln {
        StubVuln(vec![Advisory {
            id: "GHSA-test".into(),
            high: true,
            ranges: vec![crate::libraries::advisory::AffectedRange {
                introduced: None,
                fixed: crate::libraries::version::parse_semver(fixed),
            }],
        }])
    }

    #[test]
    fn parse_interval_falls_back() {
        assert_eq!(parse_interval(None), DEFAULT_INTERVAL_SECS);
        assert_eq!(parse_interval(Some("0".into())), DEFAULT_INTERVAL_SECS);
        assert_eq!(parse_interval(Some("3600".into())), 3600);
    }

    async fn seed_pin(s: &PgStore, version_used: &str) -> (uuid::Uuid, uuid::Uuid) {
        let u = uuid::Uuid::new_v4();
        let pid = s.create_project(&format!("_fsched_{u}"), None, None).await.unwrap();
        let lib = format!("_flib_{u}");
        let lid = s.upsert_library(&lib, "npm", Some("1.0.0"), None, None, None).await.unwrap();
        // A folder owned by the project, referencing the library at `version_used`.
        s.execute_raw("INSERT INTO sensei.folders_to_watch(id, path, name, status) VALUES('00000000-0000-0000-0000-000000000001','/_test','_test','watching'::sensei.watch_status) ON CONFLICT DO NOTHING").await.unwrap();
        let fid = uuid::Uuid::new_v4();
        s.execute_raw(&format!(
            "INSERT INTO sensei.folders(id, root_id, kind, name, path, abs_path, project_id) VALUES('{fid}','00000000-0000-0000-0000-000000000001','git'::sensei.folder_kind,'{lib}','{lib}','/_test/{u}','{pid}')"
        )).await.unwrap();
        s.execute_raw(&format!(
            "INSERT INTO sensei.referenced_libraries(folder_id, library_id, version_used) VALUES('{fid}','{lid}','{version_used}') ON CONFLICT DO NOTHING"
        )).await.unwrap();
        (pid, lid)
    }

    #[tokio::test]
    async fn tick_notifies_a_real_bump_and_dedupes() {
        let Ok(s) = PgStore::connect_test().await else { return };
        let q = TaskQueue::new();
        let (pid, lid) = seed_pin(&s, "1.0.0").await;
        tick(&s, &q, &Stub(Some("1.2.0".into())), &no_vulns()).await; // minor bump
        assert!(s.pending_library_update_exists(&pid, &lid, "1.2.0", false).await.unwrap(),
            "a real bump writes a library_update recommendation");
        // Second pass is idempotent — no duplicate rec for the same to_version.
        tick(&s, &q, &Stub(Some("1.2.0".into())), &no_vulns()).await;
        let n: (i64,) = sqlx_core::query_as::query_as(
            "SELECT count(*) FROM inference.recommendations WHERE project_id=$1 AND action_type='library_update' AND based_on->'library_update'->>'to_version'='1.2.0'")
            .bind(pid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(n.0, 1, "no duplicate on re-tick");
    }

    #[tokio::test]
    async fn tick_is_fail_closed_on_range_pin_and_no_latest() {
        let Ok(s) = PgStore::connect_test().await else { return };
        let q = TaskQueue::new();
        // A range pin already accepts the latest → Unknown → no notice.
        let (pid, lid) = seed_pin(&s, "^1.0.0").await;
        tick(&s, &q, &Stub(Some("1.9.0".into())), &no_vulns()).await;
        assert!(!s.pending_library_update_exists(&pid, &lid, "1.9.0", false).await.unwrap(),
            "a range pin must NOT produce a spurious update recommendation");
        // No latest resolved → no notice.
        let (pid2, lid2) = seed_pin(&s, "1.0.0").await;
        tick(&s, &q, &Stub(None), &no_vulns()).await;
        assert!(!s.pending_library_update_exists(&pid2, &lid2, "9.9.9", false).await.unwrap());
    }

    // ── Step-2 plumbing: props apply-marker, mode-aware dedup, pins source ──

    #[tokio::test]
    async fn docs_applied_marker_round_trips() {
        let Ok(s) = PgStore::connect_test().await else { return };
        let lib = format!("_fmark_{}", uuid::Uuid::new_v4());
        let lid = s.upsert_library(&lib, "cargo", Some("1.0.0"), None, None, None).await.unwrap();
        assert_eq!(s.get_library_docs_applied(&lid).await.unwrap(), None, "unset → None");
        s.set_library_docs_applied(&lid, "1.2.3", 111).await.unwrap();
        assert_eq!(
            s.get_library_docs_applied(&lid).await.unwrap().as_deref(),
            Some("1.2.3"),
            "marker round-trips the applied version"
        );
    }

    #[tokio::test]
    async fn pending_update_dedup_is_security_aware() {
        let Ok(s) = PgStore::connect_test().await else { return };
        let (pid, lid) = seed_pin(&s, "1.0.0").await;
        // A NON-security notify for to_version 2.0.0.
        let based_on = serde_json::json!({ "library_update": {
            "library_id": lid.to_string(), "to_version": "2.0.0", "is_security": false } });
        s.create_recommendation_full(&pid, "t", "w", None, "library_update", "low", &based_on, None, None).await.unwrap();
        // Same-tier query sees it; the security-tier query does NOT — so a prior
        // non-security notify can't suppress a later security flag.
        assert!(s.pending_library_update_exists(&pid, &lid, "2.0.0", false).await.unwrap(),
            "non-security query matches the non-security row");
        assert!(!s.pending_library_update_exists(&pid, &lid, "2.0.0", true).await.unwrap(),
            "security query must NOT be suppressed by a non-security row");
    }

    #[tokio::test]
    async fn pins_include_base_url_and_source_type() {
        let Ok(s) = PgStore::connect_test().await else { return };
        let (_pid, lid) = seed_pin(&s, "1.0.0").await;
        // Give the seeded library a resolvable source pointer.
        s.update_library_source(&lid, "llms.txt", Some("https://x/llms.txt")).await.unwrap();
        let pins = s.list_library_project_pins().await.unwrap();
        let row = pins.iter().find(|p| p.0 == lid).expect("seeded pin present");
        assert_eq!(row.6.as_deref(), Some("https://x/llms.txt"), "base_url in pins");
        assert_eq!(row.7.as_deref(), Some("llms.txt"), "source_type in pins");
    }

    // ── Step-3: F-v1a auto-apply PATCH ────────────────────────────────

    #[test]
    fn should_reindex_gate() {
        assert!(should_reindex(None, "1.2.4"), "no marker → needs reindex");
        assert!(should_reindex(Some("1.2.3"), "1.2.4"), "stale marker → needs reindex");
        assert!(!should_reindex(Some("1.2.4"), "1.2.4"), "marker == latest → already applied");
    }

    async fn set_pin_source(s: &PgStore, lid: &uuid::Uuid) {
        s.update_library_source(lid, "llms.txt", Some("https://x/llms.txt")).await.unwrap();
    }

    /// The `mode` of the (single) library_update rec for `pid`/`to`, if any.
    async fn rec_mode(s: &PgStore, pid: &uuid::Uuid, to: &str) -> Option<String> {
        let row: Option<(Option<String>,)> = sqlx_core::query_as::query_as(
            "SELECT based_on->'library_update'->>'mode' FROM inference.recommendations \
               WHERE project_id=$1 AND action_type='library_update' AND based_on->'library_update'->>'to_version'=$2 LIMIT 1")
            .bind(pid).bind(to).fetch_optional(s.pool()).await.unwrap();
        row.and_then(|(m,)| m)
    }

    #[tokio::test]
    async fn patch_bump_enqueues_reindex_not_a_rec() {
        let Ok(s) = PgStore::connect_test().await else { return };
        let q = TaskQueue::new();
        let (pid, lid) = seed_pin(&s, "1.0.0").await;
        set_pin_source(&s, &lid).await;
        tick(&s, &q, &Stub(Some("1.0.5".into())), &no_vulns()).await; // patch bump
        assert!(q.has_pending_kind_folder(TaskKind::IndexLibrary, &lid.to_string()).await,
            "a patch auto-applies: an IndexLibrary re-index is enqueued for the lib");
        assert!(!s.pending_library_update_exists(&pid, &lid, "1.0.5", false).await.unwrap(),
            "no recommendation is written at enqueue time (audit waits for confirmed success)");
        // Re-tick while the task is still in-flight → no duplicate enqueue.
        tick(&s, &q, &Stub(Some("1.0.5".into())), &no_vulns()).await;
        let n = q.snapshot().await.into_iter()
            .filter(|(k, f, _)| *k == TaskKind::IndexLibrary && f == &lid.to_string()).count();
        assert_eq!(n, 1, "the in-flight guard prevents a duplicate re-index");
    }

    #[tokio::test]
    async fn patch_already_applied_writes_auto_applied_audit() {
        let Ok(s) = PgStore::connect_test().await else { return };
        let q = TaskQueue::new();
        let (pid, lid) = seed_pin(&s, "1.0.0").await;
        set_pin_source(&s, &lid).await;
        // Marker already at latest ⇒ index_library confirmed the re-index.
        s.set_library_docs_applied(&lid, "1.0.5", 1).await.unwrap();
        tick(&s, &q, &Stub(Some("1.0.5".into())), &no_vulns()).await;
        assert!(!q.has_pending_kind_folder(TaskKind::IndexLibrary, &lid.to_string()).await,
            "already applied → no re-index enqueued");
        assert_eq!(rec_mode(&s, &pid, "1.0.5").await.as_deref(), Some("auto_applied"),
            "an auto_applied audit is recorded once the marker has caught up");
    }

    #[tokio::test]
    async fn patch_without_source_falls_back_to_notify() {
        let Ok(s) = PgStore::connect_test().await else { return };
        let q = TaskQueue::new();
        let (pid, lid) = seed_pin(&s, "1.0.0").await; // no source set
        tick(&s, &q, &Stub(Some("1.0.5".into())), &no_vulns()).await;
        assert!(!q.has_pending_kind_folder(TaskKind::IndexLibrary, &lid.to_string()).await,
            "no indexable source → nothing enqueued (never fabricate a url)");
        assert_eq!(rec_mode(&s, &pid, "1.0.5").await.as_deref(), Some("notify_no_source"),
            "a no-source patch surfaces a notify instead");
    }

    // ── Step-5: F-v2 security scan ────────────────────────────────────

    #[tokio::test]
    async fn security_bump_flags_high_urgency_refreshes_and_never_touches_pin() {
        let Ok(s) = PgStore::connect_test().await else { return };
        let q = TaskQueue::new();
        let (pid, lid) = seed_pin(&s, "1.0.0").await; // pin at 1.0.0
        set_pin_source(&s, &lid).await;
        // HIGH advisory fixed at 1.1.0; latest 1.5.0 is a MINOR bump that would only
        // NOTIFY — the advisory escalates it to a security refresh + high-urgency flag.
        tick(&s, &q, &Stub(Some("1.5.0".into())), &high_fixed_at("1.1.0")).await;
        assert_eq!(rec_mode(&s, &pid, "1.5.0").await.as_deref(), Some("security"),
            "a high-severity advisory escalates a minor bump to a security flag");
        let urgency: (String,) = sqlx_core::query_as::query_as(
            "SELECT urgency::text FROM inference.recommendations \
               WHERE project_id=$1 AND based_on->'library_update'->>'to_version'='1.5.0' LIMIT 1")
            .bind(pid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(urgency.0, "high", "security flags are high urgency");
        assert!(q.has_pending_kind_folder(TaskKind::IndexLibrary, &lid.to_string()).await,
            "security auto-apply refreshes docs/skills (same path as v1a)");
        // The consuming project's dependency pin is NEVER modified.
        let vu: (String,) = sqlx_core::query_as::query_as(
            "SELECT version_used FROM sensei.referenced_libraries WHERE library_id=$1 LIMIT 1")
            .bind(lid).fetch_one(s.pool()).await.unwrap();
        assert_eq!(vu.0, "1.0.0", "the dependency pin is NEVER changed — docs/skills only");
    }

    #[tokio::test]
    async fn security_flag_not_suppressed_by_prior_notify() {
        let Ok(s) = PgStore::connect_test().await else { return };
        let q = TaskQueue::new();
        let (pid, lid) = seed_pin(&s, "1.0.0").await;
        set_pin_source(&s, &lid).await;
        // A prior NON-security notify for the same to_version.
        let based_on = serde_json::json!({ "library_update": {
            "library_id": lid.to_string(), "to_version": "1.5.0", "is_security": false } });
        s.create_recommendation_full(&pid, "t", "w", None, "library_update", "low", &based_on, None, None).await.unwrap();
        // The security tick still writes a security-tier flag (mode-aware dedup).
        tick(&s, &q, &Stub(Some("1.5.0".into())), &high_fixed_at("1.1.0")).await;
        assert!(s.pending_library_update_exists(&pid, &lid, "1.5.0", true).await.unwrap(),
            "a security-tier flag is written despite a prior non-security notify");
    }

    #[tokio::test]
    async fn low_severity_advisory_does_not_escalate() {
        let Ok(s) = PgStore::connect_test().await else { return };
        let q = TaskQueue::new();
        let (pid, lid) = seed_pin(&s, "1.0.0").await;
        set_pin_source(&s, &lid).await;
        // A non-high advisory (high=false) must NOT flip is_security → stays a minor notify.
        let low = StubVuln(vec![Advisory { high: false, ..high_fixed_at("1.1.0").0.pop().unwrap() }]);
        tick(&s, &q, &Stub(Some("1.5.0".into())), &low).await;
        assert_eq!(rec_mode(&s, &pid, "1.5.0").await.as_deref(), Some("notify"),
            "a low/indeterminate advisory does not escalate — normal minor notify");
        assert!(!s.pending_library_update_exists(&pid, &lid, "1.5.0", true).await.unwrap(),
            "no security-tier flag");
    }
}
