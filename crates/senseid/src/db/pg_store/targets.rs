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

/// A gating row as the read returns it, before it becomes a [`GatingRow`].
type GatingTuple = (String, bool, bool, f64, Option<String>, Option<String>);

impl PgStore {
    /// Every catalogue metric that has a gating row, with its comparator. A
    /// metric with no gating row is not in the list: it is not eligible, and no
    /// default row is invented for it. A failed read is an error, never an empty
    /// list (TG4 reads this as "which targets exist"). A neutral metric is never
    /// eligible here, even if a row written outside the import says so.
    pub async fn metric_gating(&self) -> Result<Vec<GatingRow>, String> {
        let rows: Vec<GatingTuple> = sqlx_core::query_as::query_as(
            "SELECT m.key,
                        g.eligible AND m.direction <> 'neutral',
                        g.delta_scoped,
                        g.min_coverage::float8,
                        CASE m.direction WHEN 'higher_better' THEN '>='
                                         WHEN 'lower_better'  THEN '<=' END,
                        m.unit
                   FROM sensei.metric_gating g
                   JOIN sensei.metrics m ON m.id = g.metric_id
                  ORDER BY m.key",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("metric_gating read failed: {e}"))?;
        Ok(rows
            .into_iter()
            .map(|(metric_key, eligible, delta_scoped, min_coverage, comparator, unit)| GatingRow {
                metric_key,
                eligible,
                delta_scoped,
                min_coverage,
                comparator,
                unit,
            })
            .collect())
    }
}
