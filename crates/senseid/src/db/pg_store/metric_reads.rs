//! The metric groups' READS, inside the persistence layer (#227).
//!
//! These 25 statements used to live in `tasks/handlers/metrics/*`, which made
//! them the only production SQL outside `PgStore`. An ADR briefly defended that
//! as a deliberate exception and was superseded the same day, for a reason that
//! is about enforcement rather than taste: a documented exception has none.
//! Nothing failed when the next metric added the thirty-first query, and nothing
//! distinguished "deliberate" from "nobody noticed".
//!
//! ## One file per group, which is the ADR's one surviving point
//!
//! The argument for leaving them out was that a metric's SQL belongs beside the
//! prose explaining what it measures. That is true, and it is a question of FILE
//! ORGANISATION rather than of which layer owns the statement. Each group keeps
//! its own module here, so `session_outcomes.rs` holds the session-outcome
//! queries and the paragraphs about keying and exclusions that go with them. The
//! computer keeps the arithmetic, the thresholds and the props it writes.
//!
//! ## The window is a PARAMETER, never a spliced fragment
//!
//! [`day_filter`] and [`bind_day`] moved here from `metrics/mod.rs` with the
//! queries. A `PgStore` method that accepted a pre-built predicate string would
//! be the same problem wearing a different hat — the SQL would still be assembled
//! outside the layer. Every method here takes `window_days` and `as_of` and
//! builds the predicate itself.
//!
//! ## What this does NOT buy, so nobody over-reads a green CI run
//!
//! These statements are still `format!`-assembled, so
//! `scripts/check-sql-against-schema.py` still reaches them as fragments and
//! counts them unverifiable. Moving them did not make them statically checkable.
//! What catches a break is EXECUTING them, which every group's tests do —
//! verified by mutation 2026-10-05, when `cost` turned out to be the one group
//! whose query no test reached.

use super::PgStore;

pub(crate) mod architecture;
pub(crate) mod autonomy;
pub(crate) mod churn;
pub(crate) mod cost;
pub(crate) mod knowledge;
pub(crate) mod planner;
pub(crate) mod quality;
pub(crate) mod session_outcomes;
pub(crate) mod session_process;
pub(crate) mod usage;

/// The `$2`-anchored single-day-or-window SQL filter shared by the per-day
/// reads.
///
/// `anchor` is the group's occurrence-time expression — the timestamptz it
/// buckets on (`s.started_at`, `r.started_at`, `to_timestamp(ae.ts / 1000.0)`).
/// `as_of = None` is the rolling window (`$2 = window_days::int`, the
/// incremental path); `Some(_)` is a single historical day (`$2 = D::date`, the
/// backfill path). In ONE place so the window/day SQL — and its `$2` contract
/// with [`bind_day`] — cannot drift between groups.
///
/// The `day` SELECT column each query emits must use the SAME anchor
/// (`date_trunc('day', <anchor>)::date`) so `computed_on` matches the filter.
pub(crate) fn day_filter(anchor: &str, as_of: Option<chrono::NaiveDate>) -> String {
    match as_of {
        Some(_) => format!("date_trunc('day', {anchor})::date = $2::date"),
        None => format!("{anchor} >= now() - make_interval(days => $2::int)"),
    }
}

/// Bind `$2` for [`day_filter`]: the target day on the `Some` path, else the
/// window length. Consumes and returns the query so call sites stay one-liners.
/// Anchor-agnostic — identical for every per-day read, so the `$2` binding lives
/// in ONE place.
pub(crate) fn bind_day<'q, O>(
    q: sqlx_core::query_as::QueryAs<'q, sqlx_postgres::Postgres, O, sqlx_postgres::PgArguments>,
    window_days: u32,
    as_of: Option<chrono::NaiveDate>,
) -> sqlx_core::query_as::QueryAs<'q, sqlx_postgres::Postgres, O, sqlx_postgres::PgArguments> {
    match as_of {
        Some(d) => q.bind(d),
        None => q.bind(window_days as i32),
    }
}

impl PgStore {
    /// Today's date, from the DATABASE rather than the host clock.
    ///
    /// The `computed_on` for the snapshot metrics that store a point-in-time
    /// value rather than a windowed series. Read from the DB so the day boundary
    /// matches the `date_trunc('day', …)::date` the windowed reads use — same
    /// session timezone. Shared so the day source cannot drift between groups.
    pub(crate) async fn metric_today(&self) -> Result<chrono::NaiveDate, String> {
        let (d,): (chrono::NaiveDate,) = sqlx_core::query_as::query_as("SELECT current_date")
            .fetch_one(&self.pool)
            .await
            .map_err(|e| e.to_string())?;
        Ok(d)
    }
}
