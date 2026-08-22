//! Collective sharing preferences — the local user's "what leaves this machine,
//! toward whom, and how often" settings.
//!
//! Backs the Preferences → Sharing screen
//! (`docs/spec/screen/observatory-collective.md`, kanji 群) and gates the
//! Dōjō contribute loop (C6): destination + cadence + per-category filters + the
//! default attribution mode applied when an insight leaves the machine.
//!
//! One logical setting for the one local user, persisted as a single row in
//! `sensei.collective_preferences` (raw SQL in [`crate::db::pg_store`]). This
//! module owns the domain type, request parsing + enum validation, and the
//! conservative defaults returned when nothing is stored yet — mirroring the
//! discipline in [`crate::dojo::memberships`].

use std::collections::BTreeMap;

use dojo_protocol::AttributionMode;

use crate::api::util::parse_enum_field;
use crate::db::pg_store::{CollectivePrefsRow, PgStore};

/// Allowed `destination` values — where insights go (spec Signals: 4 states).
pub const DESTINATIONS: [&str; 4] = ["none", "global", "dojo", "both"];
/// Allowed `cadence` values — how often batches are prepared (spec Cadence chip).
pub const CADENCES: [&str; 3] = ["manual", "daily", "weekly"];
/// Shareable categories, in display order (spec Purpose list:
/// memory · pattern · rule · prompt · guard · skill · agent). A per-category
/// toggle gates whether that kind of insight may enter the next share batch.
pub const CATEGORIES: [&str; 7] = ["memory", "pattern", "rule", "prompt", "guard", "skill", "agent"];

/// Default destination — nothing leaves the machine until the user opts in.
const DEFAULT_DESTINATION: &str = "none";
/// Default cadence — batching is paused until the user triggers it.
const DEFAULT_CADENCE: &str = "manual";
/// Default attribution — the safest mode (source stripped). MUST equal
/// [`AttributionMode::Dereferenced`]`.as_db_str()` (asserted in tests). The
/// single source of truth for the conservative attribution default, shared with
/// membership creation (`api::handlers::dojo`) so a new membership never
/// defaults to the least-private `named`.
pub(crate) const DEFAULT_ATTRIBUTION: &str = "anonymous";
/// Default per-category toggle — every category is shareable once a destination
/// is enabled (`destination = none` already gates all sharing off, so an
/// all-on default just means "when I do share, share every supported kind").
const DEFAULT_CATEGORY_ON: bool = true;

/// The full [`CATEGORIES`] set at its default toggle. A `BTreeMap` keeps the JSON
/// key order stable across responses.
fn default_categories() -> BTreeMap<String, bool> {
    CATEGORIES.iter().map(|c| (c.to_string(), DEFAULT_CATEGORY_ON)).collect()
}

/// The attribution modes allowed for `attribution_default`, sourced from the
/// [`AttributionMode`] enum (the single source of truth) so this list can never
/// drift from the type.
fn attribution_modes() -> Vec<&'static str> {
    AttributionMode::ALL.iter().map(AttributionMode::as_db_str).collect()
}

/// The collective sharing preferences as returned by GET and accepted by PUT
/// `/api/preferences/collective`. `updated_at` is `None` in the not-yet-stored
/// defaults and RFC-3339 text once persisted.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct CollectivePreferences {
    /// none | global | dojo | both.
    pub destination: String,
    /// manual | daily | weekly.
    pub cadence: String,
    /// category → shareable. Always the full [`CATEGORIES`] set.
    pub categories: BTreeMap<String, bool>,
    /// named | anonymous | dereferenced (mirrors [`AttributionMode`]).
    pub attribution_default: String,
    /// RFC-3339 timestamp of the last write; `None` until first persisted.
    pub updated_at: Option<String>,
}

impl Default for CollectivePreferences {
    /// The conservative defaults returned when no row is stored yet: share
    /// nothing (`none` / `manual`), attribution `dereferenced` (source stripped),
    /// every category on (moot until a destination is enabled).
    fn default() -> Self {
        Self {
            destination: DEFAULT_DESTINATION.to_string(),
            cadence: DEFAULT_CADENCE.to_string(),
            categories: default_categories(),
            attribution_default: DEFAULT_ATTRIBUTION.to_string(),
            updated_at: None,
        }
    }
}

/// Parse the `categories` field of a PUT body into the full toggle map. Absent
/// keys default on; an unknown category key or a non-boolean value is rejected
/// (no silent bad writes).
fn parse_categories(body: &serde_json::Value) -> Result<BTreeMap<String, bool>, String> {
    let mut out = default_categories();
    match body.get("categories") {
        None | Some(serde_json::Value::Null) => Ok(out),
        Some(serde_json::Value::Object(map)) => {
            for (k, v) in map {
                if !CATEGORIES.contains(&k.as_str()) {
                    return Err(format!("unknown category: {k:?} (allowed: {})", CATEGORIES.join(", ")));
                }
                let b = v.as_bool().ok_or_else(|| format!("category {k:?} must be a boolean"))?;
                out.insert(k.clone(), b);
            }
            Ok(out)
        }
        Some(_) => Err("categories must be a JSON object of {category: boolean}".to_string()),
    }
}

impl CollectivePreferences {
    /// Parse + validate a PUT body into typed preferences. Full-replace
    /// semantics: an absent field takes its default; every present enum field is
    /// validated (`Err` → the handler answers 400, so no invalid value is ever
    /// written). `updated_at` is assigned by the store on write.
    pub fn from_request(body: &serde_json::Value) -> Result<Self, String> {
        if !body.is_object() {
            return Err("body must be a JSON object".to_string());
        }
        Ok(Self {
            destination: parse_enum_field(body, "destination", &DESTINATIONS, DEFAULT_DESTINATION)?,
            cadence: parse_enum_field(body, "cadence", &CADENCES, DEFAULT_CADENCE)?,
            attribution_default: parse_enum_field(
                body, "attribution_default", &attribution_modes(), DEFAULT_ATTRIBUTION,
            )?,
            categories: parse_categories(body)?,
            updated_at: None,
        })
    }

    /// Map a stored row into the wire shape, folding stored toggles over the full
    /// default category set so the response always carries every [`CATEGORIES`]
    /// key regardless of what the row happens to hold.
    fn from_row(row: CollectivePrefsRow) -> Self {
        let mut categories = default_categories();
        if let Some(map) = row.categories.as_object() {
            for cat in CATEGORIES {
                if let Some(b) = map.get(cat).and_then(serde_json::Value::as_bool) {
                    categories.insert(cat.to_string(), b);
                }
            }
        }
        Self {
            destination: row.destination,
            cadence: row.cadence,
            categories,
            attribution_default: row.attribution_default,
            updated_at: Some(row.updated_at),
        }
    }
}

/// Load the stored preferences, or the conservative defaults when none exist yet
/// (so the screen always has a shape).
pub async fn get(pg: &PgStore) -> Result<CollectivePreferences, String> {
    match pg.get_collective_preferences().await? {
        Some(row) => Ok(CollectivePreferences::from_row(row)),
        None => Ok(CollectivePreferences::default()),
    }
}

/// Upsert the (already-validated) preferences and return the saved shape with the
/// store-assigned `updated_at`.
pub async fn set(pg: &PgStore, prefs: CollectivePreferences) -> Result<CollectivePreferences, String> {
    let categories = serde_json::to_value(&prefs.categories).map_err(|e| e.to_string())?;
    let updated_at = pg
        .set_collective_preferences(&prefs.destination, &prefs.cadence, &categories, &prefs.attribution_default)
        .await?;
    Ok(CollectivePreferences { updated_at: Some(updated_at), ..prefs })
}

/// Serializes the DB-gated tests that mutate the single `collective_preferences`
/// row (HTTP round-trip + pg_store round-trip) so parallel test threads don't
/// clobber each other's expected state on the shared singleton.
#[cfg(test)]
pub(crate) fn test_lock() -> &'static crate::tasks::test_support::TestGate {
    static LOCK: crate::tasks::test_support::TestGate =
        crate::tasks::test_support::TestGate::new();
    &LOCK
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowed_sets_match_ddl_and_the_attribution_enum() {
        // Spec Signals: destination has 4 states; cadence has 3 chips.
        assert_eq!(DESTINATIONS, ["none", "global", "dojo", "both"]);
        assert_eq!(CADENCES, ["manual", "daily", "weekly"]);
        // attribution_default reuses the dojo_protocol AttributionMode values.
        assert_eq!(attribution_modes(), vec!["named", "anonymous"]);
        // The defaults are themselves valid members of their sets.
        assert!(DESTINATIONS.contains(&DEFAULT_DESTINATION));
        assert!(CADENCES.contains(&DEFAULT_CADENCE));
        assert_eq!(DEFAULT_ATTRIBUTION, AttributionMode::Anonymous.as_db_str());
    }

    #[test]
    fn default_is_the_conservative_share_nothing_shape() {
        let d = CollectivePreferences::default();
        assert_eq!(d.destination, "none");
        assert_eq!(d.cadence, "manual");
        assert_eq!(d.attribution_default, "anonymous");
        assert_eq!(d.updated_at, None);
        // Every category is present and on.
        assert_eq!(d.categories.len(), CATEGORIES.len());
        for cat in CATEGORIES {
            assert_eq!(d.categories.get(cat), Some(&true), "category {cat} should default on");
        }
    }

    #[test]
    fn from_request_accepts_a_full_valid_body() {
        let body = serde_json::json!({
            "destination": "both",
            "cadence": "weekly",
            "attribution_default": "named",
            "categories": { "memory": false, "agent": true }
        });
        let p = CollectivePreferences::from_request(&body).unwrap();
        assert_eq!(p.destination, "both");
        assert_eq!(p.cadence, "weekly");
        assert_eq!(p.attribution_default, "named");
        // Named toggles applied; the rest fall back to the on default.
        assert_eq!(p.categories.get("memory"), Some(&false));
        assert_eq!(p.categories.get("agent"), Some(&true));
        assert_eq!(p.categories.get("pattern"), Some(&true));
        assert_eq!(p.categories.len(), CATEGORIES.len());
    }

    #[test]
    fn from_request_defaults_absent_fields() {
        let p = CollectivePreferences::from_request(&serde_json::json!({})).unwrap();
        assert_eq!(p.destination, "none");
        assert_eq!(p.cadence, "manual");
        assert_eq!(p.attribution_default, "anonymous");
        assert_eq!(p.categories.len(), CATEGORIES.len());
    }

    #[test]
    fn from_request_rejects_invalid_destination() {
        let body = serde_json::json!({ "destination": "everyone" });
        let e = CollectivePreferences::from_request(&body).unwrap_err();
        assert!(e.contains("destination"), "error should name the field: {e}");
    }

    #[test]
    fn from_request_rejects_invalid_cadence() {
        let body = serde_json::json!({ "cadence": "hourly" });
        let e = CollectivePreferences::from_request(&body).unwrap_err();
        assert!(e.contains("cadence"), "error should name the field: {e}");
    }

    #[test]
    fn from_request_rejects_invalid_attribution() {
        let body = serde_json::json!({ "attribution_default": "public" });
        let e = CollectivePreferences::from_request(&body).unwrap_err();
        assert!(e.contains("attribution_default"), "error should name the field: {e}");
    }

    #[test]
    fn from_request_rejects_unknown_category_and_non_bool_value() {
        let unknown = serde_json::json!({ "categories": { "secrets": true } });
        assert!(CollectivePreferences::from_request(&unknown).unwrap_err().contains("secrets"));

        let non_bool = serde_json::json!({ "categories": { "memory": "yes" } });
        assert!(CollectivePreferences::from_request(&non_bool).unwrap_err().contains("memory"));
    }

    #[test]
    fn from_request_rejects_non_object_body_and_wrong_typed_fields() {
        assert!(CollectivePreferences::from_request(&serde_json::json!("nope")).is_err());
        assert!(CollectivePreferences::from_request(&serde_json::json!({ "destination": 7 })).is_err());
        assert!(CollectivePreferences::from_request(&serde_json::json!({ "categories": [] })).is_err());
    }

    #[test]
    fn from_row_folds_partial_stored_toggles_over_full_defaults() {
        let row = CollectivePrefsRow {
            destination: "global".into(),
            cadence: "daily".into(),
            categories: serde_json::json!({ "memory": false }),
            attribution_default: "anonymous".into(),
            updated_at: "2026-07-08T00:00:00Z".into(),
        };
        let p = CollectivePreferences::from_row(row);
        assert_eq!(p.destination, "global");
        assert_eq!(p.updated_at.as_deref(), Some("2026-07-08T00:00:00Z"));
        assert_eq!(p.categories.get("memory"), Some(&false));
        // Absent-in-storage keys still surface (default on).
        assert_eq!(p.categories.get("skill"), Some(&true));
        assert_eq!(p.categories.len(), CATEGORIES.len());
    }
}
