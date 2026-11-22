//! SSE endpoint for scan events — uniform frames over the state channel.

use axum::{
    extract::State,
    http::HeaderMap,
    response::sse::{Event, Sse},
};

use crate::api::state::AppState;
use crate::api::stream::{StreamKind, allows_entity, framed_stream, heartbeat, parse_cursor};

/// `GET /api/scan/events` — uniform frames for scan entities
/// (workspace/folder/activity/assistant) with cursor replay. Relay traffic
/// stays on the relay socket.
pub(crate) async fn scan_events_sse(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.streams.state_tx.subscribe();
    let cursor = parse_cursor(&headers);
    let stream = framed_stream(rx, state.streams.state_log.clone(), cursor, move |f| {
        allows_entity(StreamKind::Scan, f.entity())
    });
    heartbeat(stream)
}
