//! Relay health sentinel (#444 §10.1) — disk and swap watermarks.
//!
//! A 15 s probe classifies Normal / Warning / Critical. Warning emits SSE
//! `health_warning` and triggers an async janitor run; Critical throttles
//! both claim endpoints with 503, runs the janitor synchronously, and
//! escalates to the person (desktop notification + error log) when cleanup
//! recovers less than 20 GB. A failed probe reads Normal: a blind sentinel
//! must never halt work.

/// Host health: Normal (work proceeds), Warning (warn + async cleanup),
/// Critical (throttle claims + sync cleanup + escalate when it is not
/// enough).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Health {
    Normal,
    Warning,
    Critical,
}

/// Watermarks (#444 §10.1): warning at ≤ 25 GB free or ≥ 60% swap, critical
/// below 15 GB or above 80%. Either reading missing degrades that half to
/// non-triggering — a blind sentinel never halts work.
pub fn classify(disk_free_gb: Option<u64>, swap_used_pct: Option<u8>) -> Health {
    let critical = disk_free_gb.is_some_and(|g| g < 15) || swap_used_pct.is_some_and(|p| p > 80);
    if critical {
        return Health::Critical;
    }
    let warning = disk_free_gb.is_some_and(|g| g <= 25) || swap_used_pct.is_some_and(|p| p >= 60);
    if warning { Health::Warning } else { Health::Normal }
}

/// Probe the host: free GiB on the home volume, used swap percent. Each
/// half is independently optional (see `classify`).
pub fn probe() -> (Option<u64>, Option<u8>) {
    (disk_free_gb(), swap_used_pct())
}

fn disk_free_gb() -> Option<u64> {
    let home = sensei_bootstrap::config::try_home_dir()?;
    sysinfo::Disks::new_with_refreshed_list()
        .iter()
        .filter(|d| home.starts_with(d.mount_point()))
        .max_by_key(|d| d.mount_point().components().count())
        .map(|d| d.available_space() / 1024 / 1024 / 1024)
}

#[cfg(target_os = "macos")]
fn swap_used_pct() -> Option<u8> {
    let mut usage = std::mem::MaybeUninit::<libc::xsw_usage>::uninit();
    let mut len = std::mem::size_of::<libc::xsw_usage>() as libc::size_t;
    // SAFETY: sysctlbyname fills a correctly-sized xsw_usage or fails.
    let r = unsafe {
        libc::sysctlbyname(
            c"vm.swapusage".as_ptr(),
            usage.as_mut_ptr() as *mut libc::c_void,
            &mut len,
            std::ptr::null_mut(),
            0,
        )
    };
    if r != 0 {
        return None;
    }
    // SAFETY: initialised by the successful sysctl above.
    let u = unsafe { usage.assume_init() };
    if u.xsu_total == 0 {
        return None;
    }
    Some((u.xsu_used as u128 * 100 / u.xsu_total as u128).min(100) as u8)
}

#[cfg(not(target_os = "macos"))]
fn swap_used_pct() -> Option<u8> {
    None
}

/// Claim gate: refuse new work while the host is critical (#444 §10.1).
/// Called by both claim endpoints after auth. A blind probe passes.
pub fn gate_claims() -> Result<(), crate::relay::RelayError> {
    gate_claims_inner(probe())
}

fn gate_claims_inner(
    (disk, swap): (Option<u64>, Option<u8>),
) -> Result<(), crate::relay::RelayError> {
    if classify(disk, swap) == Health::Critical {
        return Err(crate::relay::RelayError::ServiceUnavailable(
            "relay_host_disk_exhausted".into(),
        ));
    }
    Ok(())
}

/// One pass: probe, warn or clean, escalate when cleaning is not enough.
pub async fn tick(
    pg: std::sync::Arc<crate::db::pg_store::PgStore>,
    event_tx: &tokio::sync::broadcast::Sender<crate::api::events::StateEvent>,
    notifier: std::sync::Arc<dyn crate::notifications::Notifier>,
) {
    let (disk, swap) = probe();
    match classify(disk, swap) {
        Health::Normal => {}
        Health::Warning => {
            let _ = event_tx.send(crate::api::events::StateEvent::relay("health_warning"));
            let store = pg.clone();
            tokio::spawn(async move {
                crate::tasks::relay_janitor::run(&store).await;
            });
        }
        Health::Critical => {
            // Synchronous: claims are already throttling; recover first.
            crate::tasks::relay_janitor::run(&pg).await;
            let free = probe().0.unwrap_or(u64::MAX);
            if free < 20 {
                // The sentinel is host-global and crewless, so `open_asked`
                // (crew-scoped) cannot carry this: notify the desktop
                // directly and log loudly instead (#444 §10.1 deviation).
                tracing::error!(
                    disk_free_gb = free,
                    "relay_health: critical, cleanup recovered too little — free host storage"
                );
                notifier.notify(
                    crate::notifications::NotifyLevel::Critical,
                    "Sensei host storage critical",
                    "Cleanup recovered too little — free host storage before execution resumes.",
                );
            }
        }
    }
}

/// Spawn the sentinel for the daemon's lifetime (the `relay_health`
/// schedule owns the 15 s cadence).
pub fn spawn(
    pg: std::sync::Arc<crate::db::pg_store::PgStore>,
    event_tx: tokio::sync::broadcast::Sender<crate::api::events::StateEvent>,
    notifier: std::sync::Arc<dyn crate::notifications::Notifier>,
) {
    tokio::spawn(async move {
        let store = pg.clone();
        crate::tasks::ticker::run_scheduled(pg, "relay_health", notifier.clone(), move || {
            let store = store.clone();
            let event_tx = event_tx.clone();
            let notifier = notifier.clone();
            async move {
                tick(store, &event_tx, notifier).await;
                Ok(())
            }
        })
        .await;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watermarks() {
        assert_eq!(classify(Some(40), Some(10)), Health::Normal);
        assert_eq!(classify(Some(20), Some(50)), Health::Warning);
        assert_eq!(classify(Some(40), Some(85)), Health::Critical);
        assert_eq!(classify(Some(10), Some(10)), Health::Critical);
        // Edges: warning band is inclusive, critical strict.
        assert_eq!(classify(Some(25), Some(10)), Health::Warning);
        assert_eq!(classify(Some(15), Some(10)), Health::Warning);
        assert_eq!(classify(Some(40), Some(60)), Health::Warning);
        assert_eq!(classify(Some(40), Some(80)), Health::Warning);
    }

    #[test]
    fn a_blind_sentinel_never_halts_work() {
        assert_eq!(classify(None, None), Health::Normal);
        assert_eq!(classify(None, Some(99)), Health::Critical);
        assert_eq!(classify(Some(1), None), Health::Critical);
    }

    #[test]
    fn the_claim_gate_refuses_only_critical() {
        assert!(gate_claims_inner((Some(40), Some(10))).is_ok());
        assert!(gate_claims_inner((Some(20), Some(50))).is_ok());
        let err = gate_claims_inner((Some(10), Some(10))).unwrap_err();
        assert_eq!(err.status(), 503);
        assert_eq!(err.code(), "relay_host_disk_exhausted");
    }
}
