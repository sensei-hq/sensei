//! The build/test command catalogue (`sensei.command_patterns`,
//! `sensei.command_markers`; spec tracker-sdlc-metrics S3), read whole and
//! compiled for [`crate::sdlc::classify`].

use super::*;
use crate::sdlc::classify::{Catalogue, MarkerRow, PatternRow};

impl PgStore {
    /// The whole catalogue, compiled, with the hash a session's derivation is
    /// stamped with. The hash covers every column the classifier reads (not
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
        let (hash,): (String,) = sqlx_core::query_as::query_as(
            "SELECT md5(
                 coalesce((SELECT string_agg(concat_ws(E'\\x1f', toolchain, pattern, coalesce(category, ''),
                                                       wrapper, compiles, coalesce(array_to_string(owns, ','), '*'), priority),
                                             E'\\x1e' ORDER BY toolchain, pattern)
                             FROM sensei.command_patterns), '')
                 || E'\\x1d' ||
                 coalesce((SELECT string_agg(concat_ws(E'\\x1f', toolchain, category, kind, marker),
                                             E'\\x1e' ORDER BY toolchain, category, kind, marker)
                             FROM sensei.command_markers), ''))",
        )
        .fetch_one(&self.pool)
        .await
        .map_err(|e| format!("command catalogue hash: {e}"))?;

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
