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
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, RwLock};
use std::time::Duration;

use axum::body::Body;
use axum::http::Request;
use axum::Router;
use tokio::time::{sleep, Instant};
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

/// Process-global flag: is the daemon currently serving the degraded router
/// (DB pool unavailable)? Defaults to `false` (full mode). `start_server` sets
/// it when it falls back to degraded, and the self-heal clears it after it
/// hot-swaps back to the full router. The `/health` handler reads it so a
/// caller can distinguish "Postgres is reachable" (a component probe) from
/// "the daemon has a working pool" (this flag) — the exact gap that let a
/// stale-pool daemon report a green Postgres component.
static DEGRADED: AtomicBool = AtomicBool::new(false);

/// Record that the daemon is serving in degraded mode (no DB pool).
pub fn mark_degraded() {
    DEGRADED.store(true, Ordering::Relaxed);
}

/// Record that the daemon has a working DB pool (full mode).
pub fn mark_full() {
    DEGRADED.store(false, Ordering::Relaxed);
}

/// True while the daemon is serving degraded (DB pool unavailable).
pub fn is_degraded() -> bool {
    DEGRADED.load(Ordering::Relaxed)
}

/// Current mode as the shared health enum, for the `/health` handler to report.
pub fn db_mode() -> sensei_bootstrap::DaemonDbMode {
    if is_degraded() {
        sensei_bootstrap::DaemonDbMode::Degraded
    } else {
        sensei_bootstrap::DaemonDbMode::Full
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
    use std::sync::atomic::{AtomicU32, Ordering};
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use axum::routing::get;
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
        assert!(is_degraded());
        assert_eq!(db_mode(), DaemonDbMode::Degraded);
        mark_full();
        assert!(!is_degraded());
        assert_eq!(db_mode(), DaemonDbMode::Full);
    }
}
