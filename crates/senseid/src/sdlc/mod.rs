//! SDLC metrics (spec `docs/spec/daemon/tracker-sdlc-metrics.md`): what the AI
//! sessions linked to a ticket did — builds, tests, reviews, holds — derived
//! from captured hook events and read alongside the tracker's own history.

pub mod classify;
pub mod settings;
pub mod signals;
pub mod sprint_replay;
pub mod ticket;

/// Whether `tracker_id`'s adapter reads ticket history (§G8). A missing tracker or an
/// unregistered kind is an error.
pub(crate) async fn reads_history(
    pg: &crate::db::pg_store::PgStore,
    tracker_id: uuid::Uuid,
) -> Result<bool, String> {
    let row = pg
        .tracker_by_id(tracker_id)
        .await?
        .ok_or_else(|| format!("tracker {tracker_id} not found"))?;
    crate::tracker::capabilities_of_kind(&row.kind).map(|c| c.history)
}

/// The `sdlc_metrics` reason codes the datafile registers, for tests that check
/// every reason a figure gives is one of them.
#[cfg(test)]
pub(crate) fn registered_reasons() -> std::collections::HashSet<String> {
    include_str!("../../../../database/import/staging/reason_codes.jsonl")
        .lines()
        .map(|l| serde_json::from_str::<serde_json::Value>(l).expect("a JSON line"))
        .filter(|v| v["domain"] == "sdlc_metrics")
        .map(|v| v["code"].as_str().unwrap().to_string())
        .collect()
}
