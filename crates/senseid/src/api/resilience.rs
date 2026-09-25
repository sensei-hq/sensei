//! DB-connection resilience: bounded startup retry + runtime self-heal.
//!
//! On a cold boot the daemon and Postgres usually start together (both are
//! `brew services` / launchd agents), so `PgStore::connect` can lose the race
//! and hit Postgres before it accepts connections. Historically that latched
//! the daemon into degraded mode until a manual restart — `start_server` picks
//! the router exactly once and `axum::serve` consumes it for the process life,
//! so there was no path back. This module closes that gap two ways:
//!
//!   * [`connect_with_retry`] — a bounded retry-with-backoff used at startup, so
//!     the common cold-boot race is absorbed and the daemon reaches full mode
//!     without ever serving degraded.
//!   * [`RouterHandle`] + [`reconnect_and_upgrade`] — if the bounded window
//!     still expires, the daemon serves a degraded router through a swappable
//!     handle while a background task keeps probing; once Postgres is reachable
//!     it builds the full app and hot-swaps the served router degraded → full,
//!     with no restart.

use std::future::Future;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use axum::Router;
use axum::body::Body;
use axum::http::Request;
use tokio::time::{Instant, sleep};
use tower::ServiceExt;

/// Backoff policy for a retrying connect. `max_elapsed = None` retries forever
/// (used by the background self-heal); `Some(d)` bounds the total wait (used at
/// startup so the daemon doesn't block boot indefinitely).
#[derive(Clone, Debug)]
pub struct RetryPolicy {
    /// Total wall-clock budget across all attempts. `None` = retry forever.
    pub max_elapsed: Option<Duration>,
    /// First backoff; doubles each attempt, capped at `max_delay`.
    pub base_delay: Duration,
    /// Ceiling for a single backoff.
    pub max_delay: Duration,
}

impl RetryPolicy {
    /// Startup policy: absorb the cold-boot race, then give up so the daemon can
    /// serve degraded (and self-heal in the background). The window is bounded
    /// because `axum::serve` only starts after this resolves — a long wait would
    /// leave `/health` unresponsive at boot. 10s comfortably covers Postgres
    /// becoming ready after a co-scheduled service start; a longer outage falls
    /// through to degraded + background self-heal.
    pub fn startup() -> Self {
        Self {
            max_elapsed: Some(Duration::from_secs(10)),
            base_delay: Duration::from_millis(250),
            max_delay: Duration::from_secs(2),
        }
    }

    /// Background policy: never give up — probe until the DB comes back.
    pub fn background() -> Self {
        Self {
            max_elapsed: None,
            base_delay: Duration::from_millis(500),
            max_delay: Duration::from_secs(10),
        }
    }
}

/// Process-global state: which DB mode is the daemon serving in? Defaults to
/// full. `start_server` sets it when it falls back, and the self-heal clears it
/// after it hot-swaps back to the full router. The `/health` handler reads it
/// so a caller can distinguish "Postgres is reachable" (a component probe) from
/// "the daemon has a working pool" (this) — the exact gap that let a
/// stale-pool daemon report a green Postgres component.
///
/// THREE states, not two. `Provisioning` and `Degraded` share a router — both
/// mean "no pool" — but they are different situations with different advice,
/// and collapsing them made a first install report itself as a breakage.
/// An integer rather than a bool because the router swap keys on
/// [`is_degraded`] while `/health` needs the finer answer.
static MODE: AtomicU8 = AtomicU8::new(MODE_FULL);

const MODE_FULL: u8 = 0;
const MODE_PROVISIONING: u8 = 1;
const MODE_DEGRADED: u8 = 2;

/// Record that the daemon is serving degraded — the database is there and
/// unusable.
pub fn mark_degraded() {
    MODE.store(MODE_DEGRADED, Ordering::Relaxed);
}

/// Record that the database does not exist yet and is being built. Same router
/// as degraded; a different answer on `/health`.
pub fn mark_provisioning() {
    MODE.store(MODE_PROVISIONING, Ordering::Relaxed);
}

/// Record that the daemon has a working DB pool (full mode).
pub fn mark_full() {
    MODE.store(MODE_FULL, Ordering::Relaxed);
}

/// True while the daemon has no usable pool — degraded OR provisioning. A
/// database that does not exist yet is exactly as unusable as one that broke;
/// only the REPORTED state differs.
#[cfg(test)]
fn has_no_pool() -> bool {
    MODE.load(Ordering::Relaxed) != MODE_FULL
}

/// Current mode as the shared health enum, for the `/health` handler to report.
pub fn db_mode() -> sensei_bootstrap::DaemonDbMode {
    match MODE.load(Ordering::Relaxed) {
        MODE_PROVISIONING => sensei_bootstrap::DaemonDbMode::Provisioning,
        MODE_DEGRADED => sensei_bootstrap::DaemonDbMode::Degraded,
        _ => sensei_bootstrap::DaemonDbMode::Full,
    }
}

/// Which mode a FAILED connect means, given whether the target database exists.
///
/// Pure, so the decision is testable without psql — the caller supplies the
/// answer from `sensei_bootstrap::database::database_exists`. The connect error
/// itself cannot decide this: it arrives as a formatted `String`, so reading it
/// would make the daemon's state machine depend on a Display format.
///
/// `None` is "the existence check itself failed", and it maps to `Degraded`:
/// not knowing whether a database exists is not evidence that one is being
/// built, and reporting progress nobody observed is the optimistic fabrication
/// this codebase refuses everywhere else.
pub fn mode_after_failed_connect(db_exists: Option<bool>) -> sensei_bootstrap::DaemonDbMode {
    match db_exists {
        Some(false) => sensei_bootstrap::DaemonDbMode::Provisioning,
        Some(true) | None => sensei_bootstrap::DaemonDbMode::Degraded,
    }
}

/// Call `connect` until it succeeds or `policy.max_elapsed` is exceeded, sleeping
/// with exponential backoff (capped at `policy.max_delay`) between attempts.
/// Returns the last error if a bounded budget is exhausted.
pub async fn connect_with_retry<F, Fut, T, E>(mut connect: F, policy: &RetryPolicy) -> Result<T, E>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = Result<T, E>>,
{
    let start = Instant::now();
    let mut attempt: u32 = 0;
    loop {
        match connect().await {
            Ok(v) => return Ok(v),
            Err(e) => {
                attempt += 1;
                // Exponential backoff: base * 2^(attempt-1), capped at max_delay.
                // `checked_shl` guards the shift; a saturating multiply caps the
                // Duration so a long-lived unbounded loop can't overflow.
                let factor = 1u32.checked_shl(attempt - 1).unwrap_or(u32::MAX);
                let backoff = policy.base_delay.saturating_mul(factor).min(policy.max_delay);
                // Bounded budget: give up if the next wait would run past it.
                if let Some(max) = policy.max_elapsed
                    && start.elapsed() + backoff >= max
                {
                    return Err(e);
                }
                sleep(backoff).await;
            }
        }
    }
}

/// A hot-swappable axum router. The served router ([`RouterHandle::serving_router`])
/// forwards every request to whatever inner router is current, so a background
/// task can atomically replace it (degraded → full) without restarting the
/// server.
#[derive(Clone)]
pub struct RouterHandle {
    inner: Arc<RwLock<Router>>,
}

impl RouterHandle {
    /// Wrap an initial router.
    pub fn new(router: Router) -> Self {
        Self { inner: Arc::new(RwLock::new(router)) }
    }

    /// Atomically replace the inner router.
    pub fn swap(&self, router: Router) {
        *self.inner.write().expect("RouterHandle lock poisoned") = router;
    }

    /// Clone the current inner router. Cheap — axum routers are `Arc`-backed.
    fn current(&self) -> Router {
        self.inner.read().expect("RouterHandle lock poisoned").clone()
    }

    /// Build the stable outer router to hand to `axum::serve`. It owns no routes
    /// itself; every request is forwarded to the current inner router, so a
    /// [`swap`](Self::swap) is observed by all subsequent requests.
    pub fn serving_router(&self) -> Router {
        let handle = self.clone();
        Router::new().fallback_service(tower::service_fn(move |req: Request<Body>| {
            let inner = handle.current();
            async move { inner.oneshot(req).await }
        }))
    }
}

/// Background self-heal: retry `connect` (with an unbounded policy) until it
/// succeeds, then `build` the full router from the connection and hot-swap it
/// into `handle`, taking the daemon from degraded → full mode with no restart.
/// Returns `true` if the upgrade happened. Does not touch the [`is_degraded`]
/// flag itself — the caller flips it after this returns, so the function stays
/// free of process-global state and testable in isolation.
pub async fn reconnect_and_upgrade<T, C, CFut, B, BFut>(
    handle: RouterHandle,
    connect: C,
    build: B,
    policy: &RetryPolicy,
) -> bool
where
    C: FnMut() -> CFut,
    CFut: Future<Output = Result<T, String>>,
    B: FnOnce(T) -> BFut,
    BFut: Future<Output = Router>,
{
    match connect_with_retry(connect, policy).await {
        Ok(conn) => {
            let full = build(conn).await;
            handle.swap(full);
            tracing::info!("daemon self-heal: DB reachable — upgraded degraded to full mode");
            true
        }
        Err(e) => {
            // Only reachable with a bounded policy; the caller passes an
            // unbounded one so recovery keeps trying until the DB returns.
            tracing::error!(error = %e, "daemon self-heal: reconnect loop gave up before the DB returned");
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
    use std::sync::atomic::{AtomicU32, Ordering};
    use tower::ServiceExt;

    /// A tiny router with a single `/x` route that returns the given marker text.
    fn marker_router(text: &'static str) -> Router {
        Router::new().route("/x", get(move || async move { text }))
    }

    /// Drive one GET through a router; return `(status, body_text)`.
    async fn oneshot_text(app: Router, uri: &str) -> (StatusCode, String) {
        let req = Request::builder().uri(uri).body(Body::empty()).unwrap();
        let resp = app.oneshot(req).await.unwrap();
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        (status, String::from_utf8(bytes.to_vec()).unwrap())
    }

    fn fast_policy() -> RetryPolicy {
        RetryPolicy {
            max_elapsed: Some(Duration::from_secs(30)),
            base_delay: Duration::from_millis(1),
            max_delay: Duration::from_millis(10),
        }
    }

    #[tokio::test(start_paused = true)]
    async fn connect_with_retry_succeeds_after_transient_failures() {
        let attempts = Arc::new(AtomicU32::new(0));
        let a = attempts.clone();
        let connect = move || {
            let a = a.clone();
            async move {
                let n = a.fetch_add(1, Ordering::SeqCst);
                if n < 2 { Err("cold") } else { Ok(42u32) }
            }
        };
        let out = connect_with_retry(connect, &fast_policy()).await;
        assert_eq!(out, Ok(42));
        assert_eq!(attempts.load(Ordering::SeqCst), 3, "should try 3 times (2 fail, 1 ok)");
    }

    #[tokio::test(start_paused = true)]
    async fn connect_with_retry_bounded_gives_up() {
        let attempts = Arc::new(AtomicU32::new(0));
        let a = attempts.clone();
        let connect = move || {
            let a = a.clone();
            async move {
                a.fetch_add(1, Ordering::SeqCst);
                Err::<u32, &str>("down")
            }
        };
        let out = connect_with_retry(
            connect,
            &RetryPolicy {
                max_elapsed: Some(Duration::from_millis(50)),
                base_delay: Duration::from_millis(10),
                max_delay: Duration::from_secs(1),
            },
        )
        .await;
        assert_eq!(out, Err("down"));
        assert!(attempts.load(Ordering::SeqCst) >= 2, "should retry before giving up");
    }

    #[tokio::test(start_paused = true)]
    async fn connect_with_retry_unbounded_never_gives_up() {
        let attempts = Arc::new(AtomicU32::new(0));
        let a = attempts.clone();
        let connect = move || {
            let a = a.clone();
            async move {
                let n = a.fetch_add(1, Ordering::SeqCst);
                if n < 5 { Err("x") } else { Ok(7u32) }
            }
        };
        let out = connect_with_retry(
            connect,
            &RetryPolicy {
                max_elapsed: None,
                base_delay: Duration::from_millis(10),
                max_delay: Duration::from_millis(50),
            },
        )
        .await;
        assert_eq!(out, Ok(7));
    }

    #[tokio::test]
    async fn router_handle_dispatches_to_current_then_swapped_router() {
        let handle = RouterHandle::new(marker_router("first"));
        let serving = handle.serving_router();

        let (status, body) = oneshot_text(serving.clone(), "/x").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "first");

        handle.swap(marker_router("second"));
        let (status, body) = oneshot_text(serving, "/x").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "second", "serving router must reflect the swapped inner router");
    }

    #[tokio::test(start_paused = true)]
    async fn reconnect_and_upgrade_swaps_degraded_to_full() {
        let handle = RouterHandle::new(marker_router("degraded"));
        let serving = handle.serving_router();

        // Sanity: starts degraded.
        let (_, body) = oneshot_text(serving.clone(), "/x").await;
        assert_eq!(body, "degraded");

        let attempts = Arc::new(AtomicU32::new(0));
        let a = attempts.clone();
        let connect = move || {
            let a = a.clone();
            async move {
                let n = a.fetch_add(1, Ordering::SeqCst);
                if n < 2 { Err("nope".to_string()) } else { Ok(()) }
            }
        };
        let build = |_conn: ()| async move { marker_router("full") };

        let upgraded = reconnect_and_upgrade(handle.clone(), connect, build, &fast_policy()).await;
        assert!(upgraded, "should report a successful upgrade");

        let (status, body) = oneshot_text(serving, "/x").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, "full", "after DB recovers the served router must upgrade to full");
    }

    #[test]
    fn db_mode_indicator_toggles_and_maps_to_enum() {
        // Set-then-assert (not asserting the default) so this stays deterministic
        // regardless of ordering — it is the only test that touches the flag.
        use sensei_bootstrap::DaemonDbMode;
        mark_degraded();
        assert!(has_no_pool());
        assert_eq!(db_mode(), DaemonDbMode::Degraded);
        mark_full();
        assert!(!has_no_pool());
        assert_eq!(db_mode(), DaemonDbMode::Full);
        // The third state is still "no usable pool" — a database that does not
        // exist yet is just as unusable as one that broke. Only the REPORTED
        // state differs, which is the whole point of adding it.
        mark_provisioning();
        assert!(has_no_pool(), "provisioning still has no pool");
        assert_eq!(db_mode(), DaemonDbMode::Provisioning);
        mark_full();
    }

    /// **WHICH FAILURE IS IT? ASK WHETHER THE DATABASE IS THERE.**
    ///
    /// The connect error cannot answer this: it arrives as a formatted String,
    /// so deciding on its text would be a Display-format dependency. The
    /// project already owns the real question —
    /// `sensei_bootstrap::database::database_exists` — and this is the pure
    /// half, so the decision is testable without psql.
    ///
    /// `None` means the check itself failed, and that maps to `Degraded`: not
    /// knowing whether a database exists is not evidence that one is being
    /// built, and claiming progress we cannot see would be the optimistic
    /// fabrication the no-fallback rule exists to prevent.
    #[test]
    fn a_missing_database_is_provisioning_and_anything_else_is_degraded() {
        use sensei_bootstrap::DaemonDbMode;
        assert_eq!(
            mode_after_failed_connect(Some(false)),
            DaemonDbMode::Provisioning,
            "no database yet — it is being built, so a client should wait"
        );
        assert_eq!(
            mode_after_failed_connect(Some(true)),
            DaemonDbMode::Degraded,
            "the database is there and unusable — that is a fault"
        );
        assert_eq!(
            mode_after_failed_connect(None),
            DaemonDbMode::Degraded,
            "could not tell — never claim provisioning we have not observed"
        );
    }
}
