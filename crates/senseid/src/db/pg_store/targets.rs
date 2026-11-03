//! Targets (docs/spec/dojo/targets-and-gates.md): the store side of the target
//! catalogue, resolution and evaluation. All target SQL lives here.

use super::PgStore;

/// One catalogue metric's gating eligibility (TG1): whether it may be a target,
/// how it is read, and when its reading is too thin. `comparator` comes from the
/// metric's own `direction` (`higher_better → ">="`, `lower_better → "<="`);
/// `None` for a `neutral` metric, which is never eligible. `unit` is the
/// catalogue's, never copied into the gating row.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct GatingRow {
    pub metric_key: String,
    pub eligible: bool,
    pub delta_scoped: bool,
    pub min_coverage: f64,
    pub comparator: Option<String>,
    pub unit: Option<String>,
}

impl PgStore {
    /// Every catalogue metric that has a gating row, with its comparator. A
    /// metric with no gating row is not in the list: it is not eligible, and no
    /// default row is invented for it. pending: tg1.
    pub async fn metric_gating(&self) -> Result<Vec<GatingRow>, String> {
        Err("pending: tg1".into())
    }
}
