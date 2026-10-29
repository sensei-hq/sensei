//! The build/test command catalogue (`sensei.command_patterns`,
//! `sensei.command_markers`; spec tracker-sdlc-metrics S3), read whole and
//! compiled for [`crate::sdlc::classify`].

use super::*;
use crate::sdlc::classify::{Catalogue, MarkerRow, PatternRow};

impl PgStore {
    /// The whole catalogue, compiled, with the hash a session's derivation is
    /// stamped with: SHA-256 over every column the classifier reads (not
    /// `modified_at`), so a re-seed that changes nothing re-derives nothing.
    ///
    /// A read failure or a row that does not compile is an `Err` naming it: a
    /// catalogue missing a row would classify that toolchain's runs as nothing.
    pub async fn command_catalogue(&self) -> Result<Catalogue, String> {
        type P = (String, String, Option<String>, bool, bool, Option<Vec<String>>, i32);
        let patterns: Vec<P> = sqlx_core::query_as::query_as(
            "SELECT toolchain, pattern, category, wrapper, compiles, owns, priority
               FROM sensei.command_patterns
              ORDER BY toolchain, pattern",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("command_patterns: {e}"))?;
        let markers: Vec<(String, String, String, String)> = sqlx_core::query_as::query_as(
            "SELECT toolchain, category, kind, marker
               FROM sensei.command_markers
              ORDER BY toolchain, category, kind, marker",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| format!("command_markers: {e}"))?;
        // The hash describes exactly the rows read (spec S3, A3): computed here, not
        // in a separate statement that could see a different catalogue.
        let hash = {
            use sha2::Digest;
            let mut h = sha2::Sha256::new();
            for (toolchain, pattern, category, wrapper, compiles, owns, priority) in &patterns {
                let owns = owns.as_ref().map_or("*".to_string(), |o| o.join(","));
                let category = category.as_deref().unwrap_or("");
                let line = format!(
                    "{toolchain}\x1f{pattern}\x1f{category}\x1f{wrapper}\x1f{compiles}\x1f{owns}\x1f{priority}\x1e"
                );
                h.update(line.as_bytes());
            }
            h.update(b"\x1d");
            for (toolchain, category, kind, marker) in &markers {
                h.update(format!("{toolchain}\x1f{category}\x1f{kind}\x1f{marker}\x1e").as_bytes());
            }
            hex::encode(h.finalize())
        };

        Catalogue::compile(
            patterns
                .into_iter()
                .map(|(toolchain, pattern, category, wrapper, compiles, owns, priority)| {
                    PatternRow { toolchain, pattern, category, wrapper, compiles, owns, priority }
                })
                .collect(),
            markers
                .into_iter()
                .map(|(toolchain, category, kind, marker)| MarkerRow {
                    toolchain,
                    category,
                    kind,
                    marker,
                })
                .collect(),
            hash,
        )
    }
}
