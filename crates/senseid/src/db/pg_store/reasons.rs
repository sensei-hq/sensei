//! The `sensei.reason_codes` registry — one vocabulary answering "why didn't this
//! happen?", scoped by domain (see `docs/architecture/reason-codes.md`).
//!
//! Its own module rather than living beside its first caller: the registry is
//! deliberately cross-domain (`repository_sharing`, `metric_computation`,
//! `schedule`, `governance_pull`, `rule_pack_adoption`), and the whole reason it
//! is one table instead of five is that nobody should write a second reader. The
//! metrics status endpoint is the first consumer; the sharing and schedule
//! surfaces are next, and they resolve here.

use super::*;

impl PgStore {
    /// Every reason code in one domain, ordered by `precedence` — lower first,
    /// which is "fix this one first" within the domain.
    ///
    /// Returned as a list, not a map, because the ORDER is data: precedence is
    /// only meaningful within a domain and callers that rank (a worst-first
    /// summary) need it. Callers that only resolve a code index it themselves.
    ///
    /// An unknown domain yields an empty list — honest-empty, since a domain with
    /// no codes seeded genuinely has none. A read FAILURE propagates: a caller
    /// serving reasons with a silently-empty registry would render every row's
    /// code as a bare slug, which is the exact failure the registry prevents.
    pub async fn reason_codes(&self, domain: &str) -> Result<Vec<ReasonCode>, String> {
        /// `sensei.reason_codes` in [`ReasonCode`] field order. The two enums
        /// (`kind`, `actor`) are projected as text, so this decodes without pulling
        /// the Postgres types into Rust.
        type Row = (String, String, i16, String, String, Option<String>, Option<String>);

        let rows: Vec<Row> = sqlx_core::query_as::query_as(
            "SELECT code, kind::text, precedence, summary, detail, remedy, actor::text
               FROM sensei.reason_codes
              WHERE domain = $1
              ORDER BY precedence",
        )
        .bind(domain)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("reason_codes({domain}): {e}"))?;

        Ok(rows
            .into_iter()
            .map(|(code, kind, precedence, summary, detail, remedy, actor)| ReasonCode {
                code,
                kind,
                precedence,
                summary,
                detail,
                remedy,
                actor,
            })
            .collect())
    }
}
