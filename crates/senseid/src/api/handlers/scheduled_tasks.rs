//! `/api/tasks/scheduled` — background-task visibility and editing (#96).
//!
//! Answers "what background workers exist, when did each last run, and how do I
//! change that?" — the question Jerry hit when `get_ftr_daily` came back empty
//! ("is the analyzer even running?").
//!
//! The listing is now the `sensei.schedules` TABLE. It used to be a static Rust
//! registry with each `last_run_at` read from a `sensei.config` watermark, and
//! that registry documented its own drift risk ("keep in step when a worker is
//! added"); the table is user-editable, a test asserts it agrees with
//! [`SCHEDULABLE`] in both directions, and `mark_schedule_run` records the
//! outcome of every pass — so `enabled`, the cadence, the window and the real
//! run health all come from one place. The config watermark reads are gone with
//! it. (`index_audit` still WRITES `audit.last_run`; nothing reads it any more,
//! so that write can be retired separately.)
//!
//! What stays code is the DESCRIPTION: prose about what a worker does is not
//! configuration. Spec: docs/spec/daemon/schedules.md.

use crate::api::handlers::err;
use crate::api::state::AppState;
use crate::db::pg_store::{SchedulePatch, StoredSchedule};
use crate::tasks::schedule::{SCHEDULABLE, window_label};
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::Json,
};
use chrono::NaiveTime;

/// What each schedulable worker does, keyed by its [`SCHEDULABLE`] name. Prose,
/// not configuration — the table owns WHEN a worker runs, this owns what it is.
/// A test asserts every worker has one.
const DESCRIPTIONS: &[(&str, &str)] = &[
    ("activity_prune", "Activity-data GC (after analysis derives insights)"),
    ("advance_run", "Relay run scheduler — auto-resume due pauses + tick active runs"),
    ("analyzer", "Session/log analyzer — findings, recommendations, learned memories"),
    ("capture_drain", "Capture-spool drain — re-imports hook events dead-lettered to disk"),
    ("contribute", "Dōjō upstream contribute cadence"),
    (
        "dojo_sync",
        "Dōjō sync — maps shared repositories to tenants, fetches the sync plan, and pushes allowed metric rows",
    ),
    ("index_audit", "Index integrity audit (read-only drift check)"),
    ("library_update", "Library-update detection — a new upstream version → a recommendation"),
    ("log_prune", "Structured-log TTL pruning"),
    ("metrics", "Metrics engine — one compute wave per project against its watermark"),
    ("reconcile", "Folder/index reconcile — self-healing scan-drift repair"),
    ("watchdog", "Run watchdog — stalled-run detection and recovery"),
];

/// The description for a worker, or `None` when it has none. `None` rather than
/// a placeholder: an invented description is worse than a blank cell, and the
/// coverage test means this cannot happen for a real worker.
fn description(name: &str) -> Option<&'static str> {
    DESCRIPTIONS.iter().find(|(n, _)| *n == name).map(|(_, d)| *d)
}

/// One schedule on the wire. Shared by the listing and the patch response so a
/// client sees the identical shape either way.
fn render(s: &StoredSchedule) -> serde_json::Value {
    serde_json::json!({
        "name": s.name,
        "description": description(&s.name),
        "enabled": s.enabled,
        "interval_secs": s.interval_secs,
        // `null` = any time of day, which is what a half-set window would also
        // mean — hence the patch validation that refuses to store one.
        "window": window_label(s.window_start, s.window_end),
        // Empty = every day. Never "never".
        "days": s.days,
        "last_run_at": s.last_run_at.map(|t| t.to_rfc3339()),
        "last_ok": s.last_ok,
        "last_error": s.last_error,
    })
}

/// GET /api/tasks/scheduled — every background worker with its schedule and the
/// outcome of its last pass.
pub(crate) async fn scheduled(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // Fail closed: a read error must not mask as an empty list ("no workers") —
    // that is the exact "is it even running?" ambiguity this endpoint removes.
    let rows = state.pg.list_schedules().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let tasks: Vec<_> = rows.iter().map(render).collect();
    Ok(Json(serde_json::json!({ "tasks": tasks })))
}

/// The fields a PATCH may carry. Anything else is refused, so a typo'd or
/// read-only key cannot be silently dropped.
const EDITABLE: &[&str] = &["enabled", "interval_secs", "window_start", "window_end", "days"];

/// `HH:MM` or `HH:MM:SS` — both, because clients differ: a time picker sends
/// `"22:00"` and a value copied out of Postgres carries its seconds.
fn parse_time(v: &serde_json::Value, field: &str) -> Result<NaiveTime, String> {
    let s = v.as_str().ok_or_else(|| format!("{field} must be a time string like \"22:00\""))?;
    NaiveTime::parse_from_str(s, "%H:%M:%S")
        .or_else(|_| NaiveTime::parse_from_str(s, "%H:%M"))
        .map_err(|_| format!("{field}: {s:?} is not a time of day (expected \"22:00\")"))
}

/// ISO weekdays, sorted and deduped. Sorting is not cosmetic: the table CHECKs
/// `array_length(days, 1) between 1 and 7`, so `[1,1,1,…]` would be rejected for
/// a length the user never asked for.
fn parse_days(v: &serde_json::Value) -> Result<Option<Vec<u8>>, String> {
    if v.is_null() {
        return Ok(None);
    }
    let items = v.as_array().ok_or("days must be a list of ISO weekdays (1 = Mon … 7 = Sun)")?;
    let mut days = Vec::with_capacity(items.len());
    for item in items {
        let d = item
            .as_u64()
            .and_then(|d| u8::try_from(d).ok())
            .filter(|d| (1..=7).contains(d))
            .ok_or_else(|| format!("days: {item} is not an ISO weekday (1 = Mon … 7 = Sun)"))?;
        days.push(d);
    }
    days.sort_unstable();
    days.dedup();
    // An empty list is the same request as `null`: no mask, i.e. every day.
    Ok((!days.is_empty()).then_some(days))
}

/// Read a PATCH body into a [`SchedulePatch`], or a message explaining the 400.
///
/// Pure, so every rule below is testable without a request or a database. The
/// rules exist because each one would otherwise fail SILENTLY — a rejected
/// interval as a raw CHECK violation, an unknown key as a no-op the user reads
/// as success, half a window as "any time" dressed up as a window.
fn parse_patch(body: &serde_json::Value) -> Result<SchedulePatch, String> {
    let obj = body.as_object().ok_or("body must be a JSON object")?;
    if let Some(unknown) = obj.keys().find(|k| !EDITABLE.contains(&k.as_str())) {
        return Err(format!("unknown field {unknown:?} (editable: {})", EDITABLE.join(", ")));
    }
    if obj.is_empty() {
        return Err(format!("nothing to change (editable: {})", EDITABLE.join(", ")));
    }

    let enabled = match obj.get("enabled") {
        None => None,
        Some(v) => Some(v.as_bool().ok_or("enabled must be true or false")?),
    };
    let interval_secs = match obj.get("interval_secs") {
        None => None,
        Some(v) => {
            let secs = v.as_i64().ok_or("interval_secs must be a whole number of seconds")?;
            // The CHECK rejects a zero too; saying so here means the user reads a
            // sentence rather than a constraint violation. The upper bound is the
            // `integer` column's, so an absurd cadence fails as a message rather
            // than as a silently clamped value.
            Some(
                u32::try_from(secs)
                    .ok()
                    .filter(|s| *s > 0 && i32::try_from(*s).is_ok())
                    .ok_or_else(|| {
                        format!("interval_secs must be between 1 and {}, got {secs}", i32::MAX)
                    })?,
            )
        }
    };

    // Both bounds or neither: `within_window` reads a lone bound as "any time",
    // so a half-set window is a setting the user believes they made and did not.
    let window = match (obj.get("window_start"), obj.get("window_end")) {
        (None, None) => None,
        (Some(s), Some(e)) if s.is_null() && e.is_null() => Some(None),
        (Some(s), Some(e)) if !s.is_null() && !e.is_null() => {
            Some(Some((parse_time(s, "window_start")?, parse_time(e, "window_end")?)))
        }
        _ => {
            return Err(
                "a window needs BOTH window_start and window_end (or both null to clear it) — \
                 one bound alone means any time"
                    .into(),
            );
        }
    };

    let days = match obj.get("days") {
        None => None,
        Some(v) => Some(parse_days(v)?),
    };

    Ok(SchedulePatch { enabled, interval_secs, window, days })
}

/// PATCH /api/tasks/scheduled/{name} — edit one worker's schedule.
///
/// The name is validated against the CODE registry, so an unknown one is a 404
/// rather than a row that schedules a worker nobody runs.
pub(crate) async fn patch_scheduled(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(body): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    if !SCHEDULABLE.contains(&name.as_str()) {
        return Err(err(StatusCode::NOT_FOUND, format!("no such scheduled task: {name:?}")));
    }
    let patch = parse_patch(&body).map_err(|m| err(StatusCode::BAD_REQUEST, m))?;
    let updated = state
        .pg
        .update_schedule(&name, &patch)
        .await
        .map_err(|e| err(StatusCode::INTERNAL_SERVER_ERROR, e))?
        // Schedulable but unscheduled: the row is missing, which the code↔table
        // agreement test makes a build failure. Not something to paper over by
        // inserting one here.
        .ok_or_else(|| err(StatusCode::NOT_FOUND, format!("{name} has no sensei.schedules row")))?;
    Ok(Json(render(&updated)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tasks::schedule::SCHEDULABLE;

    fn patch(body: serde_json::Value) -> Result<SchedulePatch, String> {
        parse_patch(&body)
    }
    fn hm(h: u32, m: u32) -> chrono::NaiveTime {
        chrono::NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn every_schedulable_worker_has_a_description() {
        // The description is the one thing that stays code — it is prose about
        // what a worker does, not configuration. A worker with none would list
        // with a blank cell, which is the drift the old static registry warned
        // about, so it fails here instead.
        for name in SCHEDULABLE {
            assert!(description(name).is_some(), "{name} is schedulable but has no description");
        }
        assert_eq!(
            DESCRIPTIONS.len(),
            SCHEDULABLE.len(),
            "a description for a worker that does not exist"
        );
    }

    #[test]
    fn a_zero_or_negative_interval_is_rejected_with_a_clear_message() {
        // The DB CHECK also rejects it; failing here means the user reads "must
        // be greater than 0" instead of a constraint-violation dump.
        for bad in [serde_json::json!(0), serde_json::json!(-5)] {
            let e = patch(serde_json::json!({ "interval_secs": bad })).unwrap_err();
            assert!(e.contains("interval_secs"), "the message must name the field, got {e}");
        }
        assert_eq!(
            patch(serde_json::json!({ "interval_secs": 900 })).unwrap().interval_secs,
            Some(900)
        );
    }

    #[test]
    fn a_day_outside_iso_1_to_7_is_rejected_and_the_rest_are_normalised() {
        for bad in [serde_json::json!([0]), serde_json::json!([8]), serde_json::json!(["mon"])] {
            assert!(patch(serde_json::json!({ "days": bad })).is_err(), "days {bad} must be 1..7");
        }
        // Sorted and deduped: the CHECK bounds the array length at 7, so a
        // repeated day would otherwise fail the write for the wrong reason.
        assert_eq!(
            patch(serde_json::json!({ "days": [5, 1, 1] })).unwrap().days,
            Some(Some(vec![1, 5]))
        );
        // Both spellings of "no mask" clear it — an unset mask means every day,
        // never "never".
        assert_eq!(patch(serde_json::json!({ "days": [] })).unwrap().days, Some(None));
        assert_eq!(patch(serde_json::json!({ "days": null })).unwrap().days, Some(None));
    }

    #[test]
    fn a_half_specified_window_is_rejected() {
        // `within_window` reads one bound alone as "any time", so accepting this
        // would leave the user believing they had set a window that does not
        // exist. Both bounds, or neither.
        for half in [
            serde_json::json!({ "window_start": "22:00" }),
            serde_json::json!({ "window_end": "05:00" }),
            serde_json::json!({ "window_start": "22:00", "window_end": null }),
        ] {
            let e = patch(half.clone()).unwrap_err();
            assert!(e.contains("window"), "{half} must be refused as half a window, got {e}");
        }
    }

    #[test]
    fn a_window_is_set_and_cleared_as_a_pair() {
        assert_eq!(
            patch(serde_json::json!({ "window_start": "22:00", "window_end": "05:30:00" }))
                .unwrap()
                .window,
            Some(Some((hm(22, 0), hm(5, 30)))),
            "both HH:MM and HH:MM:SS parse"
        );
        assert_eq!(
            patch(serde_json::json!({ "window_start": null, "window_end": null })).unwrap().window,
            Some(None),
            "two explicit nulls clear the window back to any time"
        );
        assert!(
            patch(serde_json::json!({ "window_start": "25:00", "window_end": "05:00" })).is_err(),
            "an impossible time is a 400, not a silently dropped window"
        );
    }

    #[test]
    fn a_field_nobody_can_act_on_is_rejected_rather_than_ignored() {
        // A typo'd or read-only key silently ignored is the same failure as the
        // half window: the user believes they changed something.
        for body in [
            serde_json::json!({ "interval": 900 }),
            serde_json::json!({ "last_ok": true }),
            serde_json::json!({}),
            serde_json::json!("enabled"),
        ] {
            assert!(patch(body.clone()).is_err(), "{body} must not be accepted as a patch");
        }
        assert_eq!(patch(serde_json::json!({ "enabled": false })).unwrap().enabled, Some(false));
        assert!(patch(serde_json::json!({ "enabled": "no" })).is_err(), "enabled is a boolean");
    }

    /// The shared `AppState` fixture. DB-backed: `make_ctx` connects to
    /// `sensei_test` and panics if it cannot, so every test below needs the
    /// daemon database running — there is no graceful skip.
    async fn state() -> crate::api::state::AppState {
        crate::tasks::test_support::make_ctx().await.app_state.clone()
    }

    #[tokio::test]
    async fn the_listing_comes_from_the_table_not_a_static_registry() {
        let state = state().await;
        let body = scheduled(State(state)).await.expect("the listing reads the schedules table");
        let rows = body.0["tasks"].as_array().expect("tasks array").clone();
        for name in SCHEDULABLE {
            let row = rows
                .iter()
                .find(|r| r["name"] == *name)
                .unwrap_or_else(|| panic!("{name} missing from the listing"));
            assert!(row["description"].is_string(), "{name} needs its code-side description");
            assert!(
                row["interval_secs"].as_u64().unwrap_or(0) > 0,
                "{name} must report the cadence it actually runs on"
            );
            assert!(row["enabled"].is_boolean(), "{name} must report whether it is scheduled");
            for key in ["window", "days", "last_run_at", "last_ok", "last_error"] {
                assert!(row.get(key).is_some(), "{name} is missing {key}");
            }
        }
    }

    #[tokio::test]
    async fn patching_a_worker_that_does_not_exist_is_a_404() {
        // Validated against the CODE registry, so a PATCH cannot invent a row
        // that names a worker nobody runs.
        let state = state().await;
        let (status, body) = patch_scheduled(
            State(state),
            axum::extract::Path("no_such_worker".to_string()),
            Json(serde_json::json!({ "enabled": false })),
        )
        .await
        .unwrap_err();
        assert_eq!(status, StatusCode::NOT_FOUND);
        // The MESSAGE too: a schedulable worker whose row is missing 404s as
        // well, so asserting the status alone would let the registry check be
        // deleted without a single test noticing.
        let msg = body.0["error"].as_str().expect("a 404 carries an error message").to_string();
        assert!(msg.contains("no such scheduled task"), "the registry refused it, got {msg}");
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn a_rejected_patch_is_a_400_and_writes_nothing() {
        // `log_prune` is a SEEDED row, shared with every other test that edits
        // one. "Nothing changed between these two reads" is only a statement
        // about THIS patch while no sibling is writing the same row.
        let _gate = crate::tasks::test_support::SCHEDULE_EDIT_GATE.enter();
        let state = state().await;
        let before = state.pg.load_schedule("log_prune").await.unwrap().unwrap();
        let (status, _) = patch_scheduled(
            State(state.clone()),
            axum::extract::Path("log_prune".to_string()),
            Json(serde_json::json!({ "interval_secs": 0 })),
        )
        .await
        .unwrap_err();
        assert_eq!(status, StatusCode::BAD_REQUEST);
        let after = state.pg.load_schedule("log_prune").await.unwrap().unwrap();
        assert_eq!(after.interval_secs, before.interval_secs, "a 400 must not have written");
    }

    #[tokio::test]
    #[allow(clippy::await_holding_lock)]
    async fn patching_a_worker_persists_the_edit_and_returns_the_new_state() {
        // Edits the same SEEDED row as the test above, so it takes the gate too.
        let _gate = crate::tasks::test_support::SCHEDULE_EDIT_GATE.enter();
        let state = state().await;

        let body = patch_scheduled(
            State(state.clone()),
            axum::extract::Path("log_prune".to_string()),
            Json(serde_json::json!({ "enabled": false, "interval_secs": 4242,
                                     "window_start": "22:00", "window_end": "05:00" })),
        )
        .await
        .expect("a valid patch applies")
        .0;
        let stored = state.pg.load_schedule("log_prune").await.unwrap().unwrap();

        // Restore BEFORE asserting, not after: a failing assertion below would
        // otherwise leave the seeded row on a 4242s cadence with a fresh
        // `modified_at`, which `staging.import_schedules` then refuses to
        // overwrite — one red test would edit this machine's schedule for good.
        crate::tasks::test_support::restore_seeded_schedule(&state.pg, "log_prune").await;

        assert_eq!(body["enabled"], serde_json::json!(false));
        assert_eq!(body["interval_secs"], serde_json::json!(4242));
        assert_eq!(body["window"], serde_json::json!("22:00–05:00"), "rendered by window_label");
        assert!(!stored.enabled, "the response must describe what was actually stored");
        assert_eq!(stored.interval_secs, 4242);
    }
}
