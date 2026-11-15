//! A workspace's SDLC metric settings (`sensei.workspace_metric_settings`, spec S1).

use super::*;
use crate::sdlc::settings::MetricSettings;

impl PgStore {
    /// The workspace's settings, or the defaults when it has no row.
    pub async fn metric_settings(
        &self,
        workspace_id: uuid::Uuid,
    ) -> Result<MetricSettings, String> {
        let row: Option<(Option<f64>, Option<f64>, i32)> = sqlx_core::query_as::query_as(
            "SELECT hourly_rate::float8, minutes_per_point::float8, idle_threshold_minutes
               FROM sensei.workspace_metric_settings
              WHERE workspace_id = $1",
        )
        .bind(workspace_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| format!("metric_settings: {e}"))?;
        Ok(row.map_or_else(MetricSettings::default, |(hourly_rate, minutes_per_point, t)| {
            MetricSettings { hourly_rate, minutes_per_point, idle_threshold_minutes: t }
        }))
    }

    /// Store the workspace's settings whole, creating the row on first write.
    pub async fn put_metric_settings(
        &self,
        workspace_id: uuid::Uuid,
        s: &MetricSettings,
    ) -> Result<(), String> {
        sqlx_core::query::query(
            "INSERT INTO sensei.workspace_metric_settings
                 (workspace_id, hourly_rate, minutes_per_point, idle_threshold_minutes, modified_at)
             VALUES ($1, $2::numeric, $3::numeric, $4, now())
             ON CONFLICT (workspace_id) DO UPDATE SET
                 hourly_rate            = excluded.hourly_rate
               , minutes_per_point      = excluded.minutes_per_point
               , idle_threshold_minutes = excluded.idle_threshold_minutes
               , modified_at            = now()",
        )
        .bind(workspace_id)
        .bind(s.hourly_rate)
        .bind(s.minutes_per_point)
        .bind(s.idle_threshold_minutes)
        .execute(&self.pool)
        .await
        .map_err(|e| format!("put_metric_settings: {e}"))?;
        Ok(())
    }
}
