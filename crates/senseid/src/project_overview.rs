//! Pure assembly helpers for the Project window · Overview pane (Slot 4).
//!
//! The `/api/projects/{id}/overview` handler composes its payload from several
//! DB sources; the small decisions that don't need a database live here so they
//! unit-test in isolation. Kept pure — no DB, no clock.
//!
//! Voice: sentence case, lowercase "sensei", no filler.

use serde_json::Value;

/// The default hero glyph when a project has no kanji icon — 場 (place), the
/// same fallback the Projects index uses.
pub const DEFAULT_KANJI: &str = "場";

/// Extract the hero kanji from a project's `icon` jsonb. Returns the glyph when
/// `icon = { "kind": "kanji", "value": "工" }`, otherwise [`DEFAULT_KANJI`]. An
/// image icon yields no kanji here — the pane still renders a kanji header, so a
/// repo-derived image never leaves the header blank.
pub fn kanji_from_icon(icon: &Value) -> String {
    if icon.get("kind").and_then(Value::as_str) == Some("kanji")
        && let Some(v) = icon.get("value").and_then(Value::as_str)
        && !v.trim().is_empty()
    {
        return v.to_string();
    }
    DEFAULT_KANJI.to_string()
}

/// Whether the project shows a warning dot. Mirrors the Projects-index rule
/// ([[screen/observatory-projects]]): a project only warns when it has recent
/// activity (`sessions_7d > 0`) AND a red quality signal (open drift, or 7-day
/// FTR below 0.6). A dormant project has no FTR signal, so the wire's zero-FTR
/// fallback must not light it amber.
pub fn is_warn(sessions_7d: i64, open_drift_count: i64, ftr_7d: f64) -> bool {
    if sessions_7d <= 0 {
        return false;
    }
    open_drift_count > 0 || ftr_7d < 0.6
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn kanji_from_a_kanji_icon() {
        assert_eq!(kanji_from_icon(&json!({ "kind": "kanji", "value": "工" })), "工");
    }

    #[test]
    fn kanji_falls_back_for_image_or_missing_or_blank() {
        assert_eq!(
            kanji_from_icon(&json!({ "kind": "image", "value": "logo.png" })),
            DEFAULT_KANJI
        );
        assert_eq!(kanji_from_icon(&json!({ "kind": "kanji", "value": "  " })), DEFAULT_KANJI);
        assert_eq!(kanji_from_icon(&json!({})), DEFAULT_KANJI);
        assert_eq!(kanji_from_icon(&Value::Null), DEFAULT_KANJI);
    }

    #[test]
    fn dormant_project_never_warns() {
        // The zero-FTR fallback (ftr_7d = 0) must NOT warn a project with no
        // recent sessions — the guard is the whole point.
        assert!(!is_warn(0, 0, 0.0));
        assert!(!is_warn(0, 9, 0.0));
    }

    #[test]
    fn active_project_warns_on_drift_or_low_ftr() {
        assert!(is_warn(3, 1, 1.0)); // open drift
        assert!(is_warn(3, 0, 0.59)); // low 7-day FTR
        assert!(!is_warn(3, 0, 0.6)); // healthy
        assert!(!is_warn(3, 0, 1.0));
    }
}
