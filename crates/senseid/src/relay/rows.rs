//! Relay events carry their rows (#418): the screen applies an event in
//! place instead of re-reading Now. Each helper reads the entity's row the
//! way Now shapes it and attaches it to the event. A row that is gone, or a
//! read that fails, leaves the event id-only — the contract the screen
//! already answers with a re-read — and a failure is logged, never masked
//! by an invented row.

use uuid::Uuid;

use crate::api::events::StateEvent;
use crate::db::pg_store::PgStore;

/// `event` with the lane's Now row, when the lane can be read.
pub async fn with_lane_row(pg: &PgStore, event: StateEvent, lane_id: &Uuid) -> StateEvent {
    match pg.relay_lane_row(*lane_id).await {
        Ok(Some(row)) => event.with_row(row),
        Ok(None) => event,
        Err(e) => {
            tracing::warn!(lane_id = %lane_id, error = ?e, "relay stream: lane row unread; event goes id-only");
            event
        }
    }
}

/// `event` with the run's band row and its `id`, while the run is open.
/// A run that left the band stays id-only: the screen reads Now for it.
pub async fn with_run_row(pg: &PgStore, event: StateEvent, run_id: &Uuid) -> StateEvent {
    match pg.relay_run_row(*run_id).await {
        Ok(Some(mut row)) => {
            row["id"] = serde_json::Value::String(run_id.to_string());
            event.with_row(row)
        }
        Ok(None) => event,
        Err(e) => {
            tracing::warn!(run_id = %run_id, error = ?e, "relay stream: run row unread; event goes id-only");
            event
        }
    }
}

/// The decision event for an ask: its Needs-you row when it was asked of
/// the person; `None` when it was asked of a lane (Needs you never shows
/// it) or could not be read (logged) — the task event beside it re-reads.
pub async fn decision_asked(
    pg: &PgStore,
    decision_id: &Uuid,
    crew_id: &Uuid,
) -> Option<StateEvent> {
    match pg.relay_decision_row(*decision_id).await {
        Ok(Some(row)) => {
            Some(StateEvent::relay_decision(false, decision_id, crew_id).with_row(row))
        }
        Ok(None) => None,
        Err(e) => {
            tracing::warn!(decision_id = %decision_id, error = ?e, "relay stream: decision row unread; no decision event");
            None
        }
    }
}

/// The message event for a crew → person message: its Needs-you row; `None`
/// when it is not Needs you's or could not be read (logged) — the task or
/// relay event beside it re-reads.
pub async fn message_sent(pg: &PgStore, message_id: &Uuid, crew_id: &Uuid) -> Option<StateEvent> {
    match pg.relay_person_message_row(*message_id).await {
        Ok(Some(row)) => Some(StateEvent::relay_message(message_id, crew_id).with_row(row)),
        Ok(None) => None,
        Err(e) => {
            tracing::warn!(message_id = %message_id, error = ?e, "relay stream: message row unread; no message event");
            None
        }
    }
}
