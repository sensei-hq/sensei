//! Observatory · Diagrams — read-only payloads for the diagram screens.
//!
//! One handler per diagram. Each assembles nothing: the shape it returns is the
//! shape its view already produces, so the screen and any other consumer cannot
//! disagree about what a span or a level means.

use crate::api::state::AppState;
use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde::Deserialize;

/// The levels the Structure diagram can be rolled up to.
///
/// Validated rather than interpolated: `level` reaches a `GROUP BY` expression,
/// and an unrecognised value must be a 400 the caller can see rather than a
/// silently narrower grouping.
const LEVELS: [&str; 3] = ["file", "module", "package"];

/// The edge kinds `sensei.structure_edges` carries. A request naming anything
/// else is a 400, not an empty graph — an empty result that means "you asked
/// for a kind that does not exist" is indistinguishable from "this project has
/// no edges", which is the distinction this screen exists to make.
const EDGE_KINDS: [&str; 5] = ["calls", "references", "imports", "implements", "extends"];

/// Defaults to `calls` alone. Measured on sensei: all five kinds at file level
/// is 3,215 edges over 1,751 nodes, which is the hairball #205 names as a wrong
/// gate. `calls` is the relationship the diagram is about; the rest are opt-in.
const DEFAULT_KINDS: &str = "calls";

#[derive(Deserialize)]
pub(crate) struct StructureQuery {
    level: Option<String>,
    kinds: Option<String>,
}

/// GET /api/projects/{id}/diagrams/structure?level=file|module|package&kinds=calls,references
///
/// Returns `{ nodes, edges, coverage, level, kinds }`.
///
/// `coverage.unplaced` is NOT decoration. Calls are under half placed on this
/// corpus, so the diagram necessarily omits more edges than it draws; a sparse
/// picture with no count beside it reads as a simple codebase rather than an
/// unresolved one.
///
/// It counts `edges.target_id IS NULL` directly, NOT `graph_placement`. An
/// earlier version asked that view for `outcome <> 'resolved'` — a predicate
/// that matched every row, because its outcomes are `placed` / `missed` /
/// `no verdict` and none of them is `resolved`. The screen therefore reported
/// the project's TOTAL edge count as its unplaced one, which is a precise-looking
/// number that says nothing. It was also 107 of the endpoint's 122 seconds.
///
/// A DB error is a 500. It is never an empty payload — that would be
/// indistinguishable from an unindexed project, which is the one thing this
/// screen must not do.
pub(crate) async fn structure(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Query(q): Query<StructureQuery>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let level = q.level.unwrap_or_else(|| "file".to_string());
    if !LEVELS.contains(&level.as_str()) {
        return Err(StatusCode::BAD_REQUEST);
    }
    let kinds: Vec<String> = q
        .kinds
        .unwrap_or_else(|| DEFAULT_KINDS.to_string())
        .split(',')
        .map(|k| k.trim().to_string())
        .filter(|k| !k.is_empty())
        .collect();
    if kinds.is_empty() || !kinds.iter().all(|k| EDGE_KINDS.contains(&k.as_str())) {
        return Err(StatusCode::BAD_REQUEST);
    }

    let project_id = resolve_project_id(&state, &id).await?;

    // CONCURRENTLY, because the three reads share nothing. Sequentially the
    // response costs their SUM; the edge query alone dominates, so waiting for
    // the other two in series adds latency for no ordering anyone needs.
    let (nodes, edges, unplaced) = tokio::try_join!(
        state.pg.structure_nodes(&project_id, &level),
        state.pg.structure_edges(&project_id, &level, &kinds),
        state.pg.structure_unplaced(&project_id, &kinds),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    // `drawn` is the payload's OWN edge count, not a second query. The line
    // sits directly beside the picture, so deriving it twice is how the two
    // come to disagree — and at a rolled-up level the number on screen is the
    // rolled-up one, which only this function knows.
    let coverage = serde_json::json!({ "drawn": edges.len() as i64, "unplaced": unplaced });

    Ok(Json(serde_json::json!({
        "level": level,
        "kinds": kinds,
        "nodes": nodes,
        "edges": edges,
        "coverage": coverage,
    })))
}

/// A project NAME or a UUID, matching the read-side pattern the other
/// project-scoped handlers use (`get_folder_commands`, the MCP tools).
///
/// A name that resolves to nothing, and a UUID with no row, are both 404 — the
/// caller asked about a project that is not here, which is not a server error.
async fn resolve_project_id(state: &AppState, id: &str) -> Result<uuid::Uuid, StatusCode> {
    if let Some(row) =
        state.pg.get_project_by_name(id).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        return crate::api::util::json_uuid(&row["id"]).ok_or(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let uid = uuid::Uuid::parse_str(id).map_err(|_| StatusCode::NOT_FOUND)?;
    state
        .pg
        .get_project(&uid)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::NOT_FOUND)?;
    Ok(uid)
}
