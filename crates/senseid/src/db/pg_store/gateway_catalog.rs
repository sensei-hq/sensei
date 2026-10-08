//! The gateway's router/model/chain catalogue, read at daemon startup (#227).
//!
//! These four statements used to live in `api/gateway_config_loader.rs`. The
//! split that survives the move is the right one: the LAYER reads rows, and
//! `gateway_config_loader` keeps the pure builders that turn them into a
//! `GatewayConfig` — `build_routers`, `build_models`, `build_chains`,
//! `assemble`, and the capability mapping. Those are unit-tested without a
//! database and stay that way.
//!
//! Unlike the metric reads, these are LITERAL statements with no `format!`, so
//! `scripts/check-sql-against-schema.py` plans every one of them against the
//! deployed schema in CI. Moving them did not change that, but it is worth
//! saying which half of the population each statement is in.
//!
//! The row tuples are named aliases rather than inline types because the model
//! row is seven columns wide and clippy is right that an inline tuple that size
//! is unreadable. The columns map 1:1 onto the `*Row` structs the builders take.

use super::PgStore;

/// `gateway.routers`: name, base url, key env var, active, headers, config.
pub type RouterTuple = (String, Option<String>, Option<String>, bool, String, String);
/// `gateway.models` + its default router: full name, family, capabilities,
/// context window, max output, router name, router model id.
pub type ModelTuple =
    (String, Option<String>, Vec<String>, Option<i32>, Option<i32>, Option<String>, Option<String>);
/// `gateway.fallback_chain_models`: chain, router, model, router model id, order.
pub type ChainModelTuple = (String, String, String, Option<String>, i32);

impl PgStore {
    /// Every router the catalogue holds, active or not.
    ///
    /// The jsonb columns are cast to TEXT and parsed in Rust rather than decoded
    /// as json, so this does not depend on sqlx's optional `json` feature.
    pub async fn gateway_routers(&self) -> Result<Vec<RouterTuple>, String> {
        sqlx_core::query_as::query_as(
            "SELECT name, api_base_url, api_key_env_var, is_active, \
                    default_headers::text, config::text \
             FROM gateway.routers",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("load_gateway_config routers: {e}"))
    }

    /// Active models, each with the router that serves it by default.
    ///
    /// The LATERAL picks ONE router per model — `is_default` first, then any
    /// active one. A model served by several routers would otherwise fan out
    /// into a row per router and be counted more than once.
    pub async fn gateway_models(&self) -> Result<Vec<ModelTuple>, String> {
        sqlx_core::query_as::query_as(
            "SELECT m.full_name, m.family, m.capabilities::text[], m.context_window, m.max_output_tokens, \
                    dr.router_name, dr.router_model_id \
             FROM gateway.models m \
             LEFT JOIN LATERAL ( \
                 SELECT r.name AS router_name, mir.router_model_id \
                 FROM gateway.models_in_router mir \
                 JOIN gateway.routers r ON r.id = mir.router_id \
                 WHERE mir.model_id = m.id AND mir.is_active \
                 ORDER BY mir.is_default DESC \
                 LIMIT 1 \
             ) dr ON true \
             WHERE m.is_active",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("load_gateway_config models: {e}"))
    }

    /// Active fallback chains: name and the capability they serve.
    pub async fn gateway_chains(&self) -> Result<Vec<(String, String)>, String> {
        sqlx_core::query_as::query_as(
            "SELECT name, capability::text FROM gateway.fallback_chains WHERE is_active",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("load_gateway_config chains: {e}"))
    }

    /// The members of each active chain, in sequence order.
    ///
    /// `models_in_router` is joined LEFT so a chain entry naming a (model,
    /// router) pair with no mapping still appears, carrying a NULL
    /// `router_model_id`. Dropping it would silently shorten a chain, which is
    /// the one failure a fallback chain must not have.
    pub async fn gateway_chain_models(&self) -> Result<Vec<ChainModelTuple>, String> {
        sqlx_core::query_as::query_as(
            "SELECT fc.name, r.name, m.full_name, mir.router_model_id, fcm.sequence_order \
             FROM gateway.fallback_chain_models fcm \
             JOIN gateway.fallback_chains fc ON fc.id = fcm.chain_id \
             JOIN gateway.routers r ON r.id = fcm.router_id \
             JOIN gateway.models m ON m.id = fcm.model_id \
             LEFT JOIN gateway.models_in_router mir \
                 ON mir.model_id = fcm.model_id AND mir.router_id = fcm.router_id \
             WHERE fcm.is_active AND fc.is_active",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("load_gateway_config chain_models: {e}"))
    }
}
