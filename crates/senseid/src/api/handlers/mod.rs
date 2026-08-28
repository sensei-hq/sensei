pub(crate) mod auth;
pub(crate) mod checker;
pub(crate) mod codebase;
pub(crate) mod config;
pub(crate) mod corrections;
pub(crate) mod dojo;
pub(crate) mod gateway;
pub(crate) mod gateway_chains;
pub(crate) mod gateway_image;
pub(crate) mod gateway_routers;
pub(crate) mod health;
pub(crate) mod identity;
pub(crate) mod instruments;
pub(crate) mod knowledge;
pub(crate) mod libraries;
pub(crate) mod logs;
pub(crate) mod mcp;
pub(crate) mod mcp_manifests;
pub(crate) mod mcp_servers;
pub(crate) mod metrics;
pub(crate) mod model_provisioning;
pub(crate) mod observatory;
pub(crate) mod planner;
pub(crate) mod playbook;
pub(crate) mod preferences;
pub(crate) mod project_detail;
pub(crate) mod query;
pub(crate) mod repositories;
pub(crate) mod review;
pub(crate) mod runs;
pub(crate) mod scan_events;
pub(crate) mod scheduled_tasks;
pub(crate) mod sessions;
pub(crate) mod share_review;
pub(crate) mod stance;
pub(crate) mod tasks;
pub(crate) mod tool_signals;
pub(crate) mod tools_health;
pub(crate) mod upgrades;
pub(crate) mod verdicts;
pub(crate) mod workspace;

/// The daemon's error envelope: a status and `{"error": "…"}` beside it.
///
/// One definition because there were nine byte-identical ones — every handler
/// module that returns a 4xx had grown its own, so the SHAPE of an API error
/// (adding a code, a request id, structured validation detail) was nine edits
/// with no compiler help if one were missed. `Display` rather than `&str`: the
/// messages are as often a `format!` as a literal.
pub(crate) fn err(
    status: axum::http::StatusCode,
    msg: impl std::fmt::Display,
) -> (axum::http::StatusCode, axum::response::Json<serde_json::Value>) {
    (status, axum::response::Json(serde_json::json!({ "error": msg.to_string() })))
}
