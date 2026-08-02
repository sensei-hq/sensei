//! Library-update scheduler (workstream F, v0 — detect + notify). Mirrors
//! [`crate::tasks::log_pruner`]. A daily tick resolves each referenced library's
//! latest published version, compares it to every project's pinned `version_used`
//! via the pure policy core, and on a real bump writes a `library_update`
//! recommendation — the EXISTING Insights/Observatory surface (no new channel).
//!
//! FAIL-CLOSED: any fetch/parse miss (or a range pin that can't be compared) skips +
//! logs and never fabricates a "newer version available". No apply, no DDL, no worker
//! task in v0 — detect+notify runs inline in the tick.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::db::pg_store::PgStore;
use crate::libraries::registry::{HttpVersionSource, VersionSource};
use crate::libraries::version::{classify_bump, update_action, Bump, UpdateAction};
use super::queue::TaskQueue;

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
    loop {
        ticker.tick().await; // first tick fires immediately → a boot pass
        tick(&pg, &queue, &src).await;
    }
}

/// One pass. Testable with a stub [`VersionSource`]. Never panics; every failure is
/// skip + log (fail-closed).
pub(crate) async fn tick(pg: &PgStore, _queue: &TaskQueue, src: &impl VersionSource) {
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

    // Compare each project pin to the latest and notify on a real, actionable bump.
    for (lib_id, name, ecosystem, _lp, project_id, version_used, _burl, _stype) in &pins {
        let Some(Some(latest)) = latest_by_lib.get(lib_id) else { continue };
        let bump = classify_bump(version_used, latest);
        if update_action(bump, false) == UpdateAction::Ignore {
            continue; // None/Unknown (incl. range pins) — no notice
        }
        match pg.pending_library_update_exists(project_id, lib_id, latest, false).await {
            Ok(true) => continue, // already flagged this exact update
            Ok(false) => {}
            Err(e) => {
                tracing::warn!(error = %e, "library_update_scheduler: dedup check failed — skip");
                continue;
            }
        }
        let bump_str = match bump {
            Bump::Patch => "patch",
            Bump::Minor => "minor",
            Bump::Major => "major",
            _ => "update",
        };
        let urgency = if bump == Bump::Major { "medium" } else { "low" };
        let title = format!("Update {name} {version_used} → {latest} ({bump_str})");
        let why = format!("A newer {ecosystem} version of {name} is available.");
        let based_on = serde_json::json!({ "library_update": {
            "library_id": lib_id.to_string(), "ecosystem": ecosystem, "name": name,
            "from_version": version_used, "to_version": latest, "bump": bump_str,
        }});
        if let Err(e) = pg.create_recommendation_full(project_id, &title, &why, None, "library_update", urgency, &based_on, None, None).await {
            tracing::warn!(error = %e, lib = %name, "library_update_scheduler: create recommendation failed");
        }
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
        tick(&s, &q, &Stub(Some("1.2.0".into()))).await; // minor bump
        assert!(s.pending_library_update_exists(&pid, &lid, "1.2.0", false).await.unwrap(),
            "a real bump writes a library_update recommendation");
        // Second pass is idempotent — no duplicate rec for the same to_version.
        tick(&s, &q, &Stub(Some("1.2.0".into()))).await;
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
        tick(&s, &q, &Stub(Some("1.9.0".into()))).await;
        assert!(!s.pending_library_update_exists(&pid, &lid, "1.9.0", false).await.unwrap(),
            "a range pin must NOT produce a spurious update recommendation");
        // No latest resolved → no notice.
        let (pid2, lid2) = seed_pin(&s, "1.0.0").await;
        tick(&s, &q, &Stub(None)).await;
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
}
