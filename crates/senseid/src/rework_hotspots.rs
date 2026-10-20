//! Rework hotspots — how the Patterns view shows re-edit churn (#287).
//!
//! The analyzer writes one `rework: <file>` anti-pattern row per file a session
//! re-edited heavily. Those rows are the signal: `rework_density` counts them
//! and each feeds a per-file "High rework" recommendation. As patterns they
//! flooded the Patterns view — one row per path — so the view folds the
//! unreviewed ones into a single anti-pattern, its files grouped by top-level
//! directory.

/// The name prefix the analyzer gives a rework row: `rework: <file>`.
pub const REWORK_PREFIX: &str = "rework: ";

/// The file a rework row names, or `None` when the row is not a rework row.
pub fn rework_file(name: &str) -> Option<&str> {
    name.strip_prefix(REWORK_PREFIX).map(str::trim)
}

/// One reworked file: its path, the row's confidence, and its busiest
/// session's re-edit count (`instances[0].max_session_edits`).
pub struct Hotspot {
    pub file: String,
    pub confidence: Option<f64>,
    pub peak_edits: Option<i64>,
}

/// The single "rework hotspots" anti-pattern for these files, or `None` for no
/// files.
pub fn hotspots(files: &[Hotspot]) -> Option<serde_json::Value> {
    if files.is_empty() {
        return None;
    }
    // Busiest first: by the busiest session's re-edits — confidence caps at 10
    // re-edits, so it ties for many files — then confidence, then path.
    let mut ranked: Vec<&Hotspot> = files.iter().collect();
    ranked.sort_by(|a, b| {
        b.peak_edits
            .unwrap_or(0)
            .cmp(&a.peak_edits.unwrap_or(0))
            .then_with(|| b.confidence.unwrap_or(0.0).total_cmp(&a.confidence.unwrap_or(0.0)))
            .then_with(|| a.file.cmp(&b.file))
    });

    let mut groups: Vec<(String, Vec<String>)> = Vec::new();
    for Hotspot { file: path, .. } in &ranked {
        let dir = match path.split_once('/') {
            Some((top, _)) => top.to_string(),
            None => ROOT.to_string(),
        };
        match groups.iter_mut().find(|(d, _)| *d == dir) {
            Some((_, list)) => list.push(path.clone()),
            None => groups.push((dir, vec![path.clone()])),
        }
    }
    groups.sort_by(|a, b| b.1.len().cmp(&a.1.len()).then_with(|| a.0.cmp(&b.0)));

    let tally: Vec<String> = groups.iter().map(|(d, list)| format!("{d} {}", list.len())).collect();
    let example: Vec<&str> = ranked.iter().take(EXAMPLE_FILES).map(|h| h.file.as_str()).collect();
    let confidence = files.iter().filter_map(|h| h.confidence).reduce(f64::max);
    Some(serde_json::json!({
        "id":            HOTSPOTS_ID,
        "name":          "rework hotspots",
        "family":        null,
        "isAntiPattern": true,
        "lifecycle":     "suggested",
        "confidence":    confidence,
        "instanceCount": files.len(),
        "kind":          crate::pattern_effectiveness::pattern_kind(true, "suggested"),
        "ftrDelta":      null,
        "description":   format!(
            "{} files were re-edited heavily within a single session — by directory: {}.",
            files.len(),
            tally.join(", "),
        ),
        "example":       example.join(", "),
        "enforcement":   null,
        "groups":        groups
            .into_iter()
            .map(|(directory, files)| serde_json::json!({ "directory": directory, "files": files }))
            .collect::<Vec<_>>(),
    }))
}

/// The group of files at the repository root (no directory).
const ROOT: &str = "(root)";
/// How many of the busiest files the hotspots' example names.
const EXAMPLE_FILES: usize = 5;
/// The folded entry's id: it stands for many rows, so it has no row id.
const HOTSPOTS_ID: &str = "rework-hotspots";

#[cfg(test)]
mod tests {
    use super::*;

    fn f(path: &str, c: f64) -> Hotspot {
        Hotspot { file: path.to_string(), confidence: Some(c), peak_edits: None }
    }

    #[test]
    fn rework_file_reads_the_prefix() {
        assert_eq!(rework_file("rework: src/a.rs"), Some("src/a.rs"));
        assert_eq!(rework_file("correction-prone"), None);
    }

    #[test]
    fn no_files_no_hotspots() {
        assert_eq!(hotspots(&[]), None);
    }

    #[test]
    fn one_anti_pattern_grouping_the_files_by_top_level_directory() {
        let h = hotspots(&[
            f("crates/a.rs", 0.5),
            f("crates/b/c.rs", 1.0),
            f("docs/x.md", 0.7),
            f("Makefile", 0.6),
        ])
        .unwrap();
        assert_eq!(h["name"], "rework hotspots");
        assert_eq!(h["isAntiPattern"], true);
        assert_eq!(h["instanceCount"], 4);
        assert_eq!(h["confidence"], 1.0);
        // Largest directory first; a root-level file is its own group, "(root)".
        assert_eq!(
            h["groups"],
            serde_json::json!([
                { "directory": "crates", "files": ["crates/b/c.rs", "crates/a.rs"] },
                { "directory": "(root)", "files": ["Makefile"] },
                { "directory": "docs", "files": ["docs/x.md"] },
            ])
        );
        assert_eq!(
            h["description"],
            "4 files were re-edited heavily within a single session — by directory: crates 2, (root) 1, docs 1."
        );
        // The busiest files first.
        assert_eq!(h["example"], "crates/b/c.rs, docs/x.md, Makefile, crates/a.rs");
    }

    /// Confidence caps at 10 re-edits, so many files tie at 1.0 (134 of 371
    /// live); the busiest-session re-edit count decides between them.
    #[test]
    fn busiest_by_peak_re_edits_when_confidence_ties() {
        let h = hotspots(&[
            Hotspot { file: "a.rs".into(), confidence: Some(1.0), peak_edits: Some(12) },
            Hotspot { file: "b.rs".into(), confidence: Some(1.0), peak_edits: Some(54) },
            Hotspot { file: "c.rs".into(), confidence: Some(0.6), peak_edits: Some(6) },
        ])
        .unwrap();
        assert_eq!(h["example"], "b.rs, a.rs, c.rs");
        assert_eq!(h["confidence"], 1.0);
    }
}
