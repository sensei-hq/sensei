//! SSE endpoint for scan events — emits StateEvent format.

use axum::{
    extract::State,
    response::sse::{Event, Sse},
};
use tokio_stream::wrappers::BroadcastStream;
use tokio_stream::StreamExt;

use crate::api::state::AppState;

/// `GET /api/scan/events` — SSE stream of StateEvent (project + activity).
pub(crate) async fn scan_events_sse(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = Result<Event, std::convert::Infallible>>> {
    let rx = state.event_tx.subscribe();
    let stream = BroadcastStream::new(rx)
        .filter_map(|result| {
            match result {
                Ok(event) => {
                    let data = serde_json::to_string(&event).unwrap_or_default();
                    Some(Ok(Event::default().data(data)))
                }
                Err(_) => None,
            }
        });
    Sse::new(stream)
}
