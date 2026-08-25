//! Relay P5.3 — the fallback (coarse) backend.
//!
//! aider / Codex / plain CLIs have no hooks and no ACP, so the daemon cannot
//! drive them or intercept a live `PreToolUse` gate. Jerry's decision
//! (2026-07-18): such a fallback assistant already has sensei's MCP configured
//! everywhere, so as it works it calls sensei MCP tools — and the **recency of
//! that already-captured activity** is the coarse signal. No new capture path:
//! read the last MCP tool-call / `assistant_event` timestamp for the run and
//! collapse it to a **coarse** feed — running (recent) / idle (quiet past a
//! threshold) / done (session terminal). Observe-coarse only — there is **no**
//! live PreToolUse gate here (fallback gating is checkpoint/time-based, out of
//! scope for P5.3; relay-engine §7).
//!
//! ## What is done vs deferred
//!
//! - **Done + heavily tested:** the PURE [`coarse_status`] freshness collapse
//!   (recent→Running / stale→Idle / none→Idle / terminal→Done) and its
//!   [`coarse_to_run_status`] mapping onto the shared
//!   [`dojo_protocol::relay::RelayRunStatus`], plus the [`FallbackDriver`] behind
//!   the P5.1 [`RunDriver`] seam with `capability() == DriveCapability::Fallback`.
//!   The elapsed-time math is **reused** from
//!   [`crate::assistants::business_elapsed_hours`] (the same helper
//!   `capture_freshness` uses) — recency-of-captured-activity is the same idea,
//!   so it is imported, not re-derived.
//! - **Deferred (`// FOLLOW-UP`):** the thin impure DB read of the run's
//!   last-activity timestamp + terminal flag (the newest MCP `assistant_event`
//!   for the run) that feeds [`coarse_observe`]. It is kept **injectable** — the
//!   observe method takes the last-activity + terminal inputs — so the full
//!   coarse mapping is unit-tested without a DB; wiring the query is a later
//!   enrichment (like ACP's live loop in P5.2b). Nothing here is wired into
//!   `advance_run`'s drive tick — fallback is observe-coarse, never driven.

use super::trait_def::{DriveCapability, DriveStep, RunDriver};
use crate::agent_spawn::{AgentOutput, AgentSpawnError};
use crate::assistants::business_elapsed_hours;
use dojo_protocol::relay::RelayRunStatus;
use std::time::Duration;

/// The coarse feed a fallback backend can offer: exactly the three states
/// relay-engine §7 allows a no-hooks/no-ACP assistant — running / idle / done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoarseStatus {
    /// The assistant's sensei-MCP activity is recent (within the idle threshold).
    Running,
    /// The activity has gone quiet past the threshold, or there is none yet.
    Idle,
    /// The session has reached a terminal state.
    Done,
}

/// **The heavily-TDD'd core.** Collapse the recency of a fallback assistant's
/// captured sensei-MCP activity into a [`CoarseStatus`]. PURE — no DB, no clock,
/// no network; the caller injects `now_ms` + the run's `last_activity_ms` (newest
/// MCP `assistant_event` ts) + a `terminal` flag.
///
/// Rules (in order):
/// 1. `terminal` → [`CoarseStatus::Done`] regardless of any timestamp — a
///    finished session is done even if the last activity was seconds ago.
/// 2. else `last_activity_ms` within `idle_after` of `now_ms` → [`CoarseStatus::Running`].
/// 3. else (older than the threshold, or `None` — never seen) → [`CoarseStatus::Idle`].
///
/// Timestamp sanity: elapsed is computed via the SHARED
/// [`business_elapsed_hours`] helper (the same one `capture_freshness` uses),
/// which already returns `0.0` when `to <= from`. So a **future** `last_activity`
/// (clock skew) yields `0` elapsed → within any window → Running (never a panic,
/// never underflow), and an exactly-at-`now` timestamp is likewise `0` → Running.
/// Weekends are NOT excluded here: coarse liveness is a short wall-clock recency
/// signal (an idle assistant on Saturday IS idle), unlike the multi-day capture
/// window.
pub fn coarse_status(
    last_activity_ms: Option<i64>,
    now_ms: i64,
    idle_after: Duration,
    terminal: bool,
) -> CoarseStatus {
    if terminal {
        return CoarseStatus::Done;
    }
    match last_activity_ms {
        // Never seen any activity → treat as idle (nothing is happening yet),
        // matching capture_freshness's "never captured" being a non-Ok signal.
        None => CoarseStatus::Idle,
        Some(last) => {
            // Reuse the shared elapsed helper (full wall-clock — see doc above).
            // It clamps future/equal timestamps to 0.0, so no negative/panic.
            let elapsed_hours = business_elapsed_hours(last, now_ms, false);
            let idle_after_hours = idle_after.as_secs_f64() / 3_600.0;
            if elapsed_hours <= idle_after_hours {
                CoarseStatus::Running
            } else {
                CoarseStatus::Idle
            }
        }
    }
}

/// Map a [`CoarseStatus`] onto the shared relay run status so the coarse feed
/// surfaces in the relay view consistently with every other backend (DRY — one
/// status vocabulary, [`RelayRunStatus`], for all backends).
///
/// - `Running` → [`RelayRunStatus::Running`] (advancing).
/// - `Done`    → [`RelayRunStatus::Done`] (terminal success).
/// - `Idle`    → [`RelayRunStatus::Stalled`] — the **honest** mapping. There is
///   no `Idle` variant, and `Paused` is reserved for an *intentional* pause that
///   carries a resume time (rate/weekly limit) — a silently-quiet fallback is
///   NOT that. `Stalled` is defined as "no progress within the expected window —
///   auto-diagnosed", which is exactly a coarse idle, and it drives the phone's
///   "needs you" band ([`RelayRunStatus::needs_attention`]) — the correct nudge
///   for a fallback whose gating is time/checkpoint-based rather than a live gate.
pub fn coarse_to_run_status(status: CoarseStatus) -> RelayRunStatus {
    match status {
        CoarseStatus::Running => RelayRunStatus::Running,
        CoarseStatus::Idle => RelayRunStatus::Stalled,
        CoarseStatus::Done => RelayRunStatus::Done,
    }
}

/// The fallback (coarse) backend behind the P5.1 [`RunDriver`] seam.
///
/// `capability()` is [`DriveCapability::Fallback`]. This is **observe-coarse**:
/// it does not spawn/drive an agent (`drive_step` returns unsupported — there is
/// no headless drive for a plain CLI), it derives a coarse [`CoarseStatus`] from
/// the recency of the run's captured sensei-MCP activity via [`coarse_observe`].
pub struct FallbackDriver {
    /// The underlying tool this fallback wraps (e.g. `"aider"`, `"codex"`).
    /// Empty → the bare `"fallback"` backend.
    pub tool: String,
    /// How long the assistant's sensei-MCP activity may be quiet before the
    /// coarse feed reports [`CoarseStatus::Idle`].
    pub idle_after: Duration,
    /// The composed backend id — `"fallback:<tool>"`, or `"fallback"` when
    /// `tool` is empty. Cached at construction ([`FallbackDriver::new`]) so
    /// [`id`](FallbackDriver::id) can return a borrow (the trait returns `&str`)
    /// without re-allocating per call.
    id_cache: String,
}

impl FallbackDriver {
    /// Construct a fallback driver for an underlying `tool` (e.g. `"aider"`).
    /// Pass an empty string for a bare `"fallback"` backend.
    pub fn new(tool: impl Into<String>, idle_after: Duration) -> Self {
        let tool = tool.into();
        let id_cache =
            if tool.is_empty() { "fallback".to_string() } else { format!("fallback:{tool}") };
        Self { tool, idle_after, id_cache }
    }

    /// Derive this run's coarse status from the recency of its captured
    /// sensei-MCP activity. Pure-ish: the DB read of `last_activity_ms` +
    /// `terminal` is the thin impure part and is **injected** here so the mapping
    /// is unit-tested without a DB. Returns both the [`CoarseStatus`] and its
    /// [`RelayRunStatus`] projection (the shape the relay view consumes).
    //
    // FOLLOW-UP (P5.3 wiring): a thin async method that reads the run's newest
    // MCP `assistant_event` ts + terminal flag from `PgStore` and calls this —
    // the impure adapter over the pure core, mirroring how ACP's live loop wraps
    // the pure `acp_update_to_segments`.
    pub fn coarse_observe(
        &self,
        last_activity_ms: Option<i64>,
        now_ms: i64,
        terminal: bool,
    ) -> (CoarseStatus, RelayRunStatus) {
        let coarse = coarse_status(last_activity_ms, now_ms, self.idle_after, terminal);
        (coarse, coarse_to_run_status(coarse))
    }
}

impl RunDriver for FallbackDriver {
    fn id(&self) -> &str {
        // Carry the underlying tool so a mixed run list distinguishes backends
        // (`"fallback:aider"`); the bare `"fallback"` when no tool is set. The
        // composed id is cached at construction (see `new`).
        &self.id_cache
    }

    fn capability(&self) -> DriveCapability {
        DriveCapability::Fallback
    }

    async fn drive_step(&self, _step: DriveStep<'_>) -> Result<AgentOutput, AgentSpawnError> {
        // Observe-coarse: a plain CLI / aider / Codex has no headless drive the
        // daemon owns. Fail loudly (like AcpObserveDriver) rather than silently
        // no-op, so a caller that wires fallback into a drive tick by mistake
        // breaks immediately instead of hanging a run that never advances.
        Err(AgentSpawnError::Spawn(
            "fallback backend is observe-coarse only — no headless drive".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const IDLE_AFTER: Duration = Duration::from_secs(300); // 5 min

    // A fixed "now" so tests are deterministic. 2026-06-15T10:00:00Z (a Monday,
    // matching the health.rs fixtures) in epoch ms.
    fn now() -> i64 {
        chrono::DateTime::parse_from_rfc3339("2026-06-15T10:00:00Z").unwrap().timestamp_millis()
    }

    // ---- the pure coarse_status collapse ----

    #[test]
    fn recent_activity_is_running() {
        // 1 minute ago, threshold 5 min → within window → Running.
        let one_min_ago = now() - 60_000;
        assert_eq!(
            coarse_status(Some(one_min_ago), now(), IDLE_AFTER, false),
            CoarseStatus::Running
        );
    }

    #[test]
    fn stale_activity_is_idle() {
        // 10 minutes ago, threshold 5 min → past window → Idle.
        let ten_min_ago = now() - 600_000;
        assert_eq!(coarse_status(Some(ten_min_ago), now(), IDLE_AFTER, false), CoarseStatus::Idle);
    }

    #[test]
    fn no_activity_is_idle() {
        // Never seen any MCP activity for the run → Idle (nothing happening yet).
        assert_eq!(coarse_status(None, now(), IDLE_AFTER, false), CoarseStatus::Idle);
    }

    #[test]
    fn terminal_is_done_regardless_of_recency() {
        // Terminal wins over any timestamp: recent, stale, or none → Done.
        let one_sec_ago = now() - 1_000;
        assert_eq!(coarse_status(Some(one_sec_ago), now(), IDLE_AFTER, true), CoarseStatus::Done);
        let ten_min_ago = now() - 600_000;
        assert_eq!(coarse_status(Some(ten_min_ago), now(), IDLE_AFTER, true), CoarseStatus::Done);
        assert_eq!(coarse_status(None, now(), IDLE_AFTER, true), CoarseStatus::Done);
    }

    #[test]
    fn exact_boundary_is_running() {
        // Exactly `idle_after` ago → `elapsed <= threshold` → Running (inclusive
        // boundary, matching capture_freshness's `elapsed <= window_hours`).
        let exactly_threshold_ago = now() - IDLE_AFTER.as_millis() as i64;
        assert_eq!(
            coarse_status(Some(exactly_threshold_ago), now(), IDLE_AFTER, false),
            CoarseStatus::Running
        );
    }

    #[test]
    fn just_past_boundary_is_idle() {
        // One millisecond past the threshold → Idle (the exclusive far side).
        let just_past = now() - IDLE_AFTER.as_millis() as i64 - 1;
        assert_eq!(coarse_status(Some(just_past), now(), IDLE_AFTER, false), CoarseStatus::Idle);
    }

    #[test]
    fn future_timestamp_is_running_not_panic() {
        // Clock skew: last_activity is AFTER now. Must clamp to 0 elapsed →
        // within any window → Running, never a panic or underflow.
        let one_hour_future = now() + 3_600_000;
        assert_eq!(
            coarse_status(Some(one_hour_future), now(), IDLE_AFTER, false),
            CoarseStatus::Running
        );
    }

    #[test]
    fn now_exactly_is_running() {
        // last_activity == now → 0 elapsed → Running.
        assert_eq!(coarse_status(Some(now()), now(), IDLE_AFTER, false), CoarseStatus::Running);
    }

    #[test]
    fn huge_gap_is_idle() {
        // A very old timestamp (days ago) → far past any minutes-scale threshold
        // → Idle. Guards against any overflow in the elapsed math at scale.
        let days_ago = now() - 10 * 24 * 3_600_000;
        assert_eq!(coarse_status(Some(days_ago), now(), IDLE_AFTER, false), CoarseStatus::Idle);
    }

    #[test]
    fn zero_idle_after_only_now_is_running() {
        // A zero threshold: only an exactly-now (or future) activity is Running;
        // anything in the past is Idle. Degenerate but must not panic.
        let zero = Duration::from_secs(0);
        assert_eq!(coarse_status(Some(now()), now(), zero, false), CoarseStatus::Running);
        assert_eq!(coarse_status(Some(now() - 1), now(), zero, false), CoarseStatus::Idle);
    }

    // ---- the CoarseStatus → RelayRunStatus mapping (the honesty call) ----

    #[test]
    fn coarse_maps_to_honest_relay_status() {
        assert_eq!(coarse_to_run_status(CoarseStatus::Running), RelayRunStatus::Running);
        // Idle → Stalled is the honest mapping (no Idle variant; Paused is for an
        // intentional resume-timed pause, not a silent quiet).
        assert_eq!(coarse_to_run_status(CoarseStatus::Idle), RelayRunStatus::Stalled);
        assert_eq!(coarse_to_run_status(CoarseStatus::Done), RelayRunStatus::Done);
    }

    #[test]
    fn idle_relay_status_needs_attention() {
        // The honesty check: a coarse-idle fallback drives the phone's "needs
        // you" band (its gating is time/checkpoint-based, so it must surface),
        // and is NOT terminal.
        let idle = coarse_to_run_status(CoarseStatus::Idle);
        assert!(idle.needs_attention());
        assert!(!idle.is_terminal());
        // Running is neither.
        assert!(!coarse_to_run_status(CoarseStatus::Running).needs_attention());
        // Done is terminal.
        assert!(coarse_to_run_status(CoarseStatus::Done).is_terminal());
    }

    // ---- the driver behind the P5.1 seam ----

    #[test]
    fn id_carries_underlying_tool() {
        let d = FallbackDriver::new("aider", IDLE_AFTER);
        assert_eq!(d.id(), "fallback:aider");
        assert_eq!(d.capability(), DriveCapability::Fallback);
    }

    #[test]
    fn bare_fallback_id_when_no_tool() {
        let d = FallbackDriver::new("", IDLE_AFTER);
        assert_eq!(d.id(), "fallback");
        assert_eq!(d.capability(), DriveCapability::Fallback);
    }

    #[tokio::test]
    async fn drive_step_is_unsupported_observe_coarse() {
        use std::path::Path;
        // Observe-coarse: drive_step must fail loudly (no headless drive for a
        // plain CLI), never silently succeed.
        let d = FallbackDriver::new("codex", IDLE_AFTER);
        let err = d
            .drive_step(DriveStep {
                cwd: Path::new("."),
                prompt: "would drive",
                timeout: Duration::from_secs(1),
            })
            .await
            .expect_err("fallback is observe-coarse only");
        match err {
            AgentSpawnError::Spawn(msg) => {
                assert!(msg.contains("observe-coarse"), "msg was {msg:?}")
            }
            other => panic!("expected a Spawn error, got {other:?}"),
        }
    }

    #[test]
    fn coarse_observe_runs_the_pure_core() {
        // The (injectable) observe method delegates to the pure core: recent →
        // Running/running; stale → Idle/stalled; terminal → Done/done.
        let d = FallbackDriver::new("aider", IDLE_AFTER);

        let (c, r) = d.coarse_observe(Some(now() - 60_000), now(), false);
        assert_eq!(c, CoarseStatus::Running);
        assert_eq!(r, RelayRunStatus::Running);

        let (c, r) = d.coarse_observe(Some(now() - 600_000), now(), false);
        assert_eq!(c, CoarseStatus::Idle);
        assert_eq!(r, RelayRunStatus::Stalled);

        let (c, r) = d.coarse_observe(None, now(), true);
        assert_eq!(c, CoarseStatus::Done);
        assert_eq!(r, RelayRunStatus::Done);
    }

    #[test]
    fn observe_update_is_none_not_an_acp_observer() {
        // Fallback contributes a status-level (not segment-level) signal, so the
        // ACP segment observe seam stays the trait default (None) — backward
        // compatible, and it does not masquerade as an ACP outline observer.
        use agent_client_protocol::schema::v1::{Plan, SessionUpdate};
        let d = FallbackDriver::new("aider", IDLE_AFTER);
        assert!(d.observe_update(&SessionUpdate::Plan(Plan::new(vec![]))).is_none());
    }
}
