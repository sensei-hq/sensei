//! The facet records on disk, beside the report: one `<session>.json` each.

use crate::facets::Facet;
use std::path::{Path, PathBuf};

/// Whether a session's record is already on disk AND reads as one. A file that
/// does not parse — a write cut short — is not a record, so it is derived again.
pub fn already_derived(target: &Path) -> bool {
    std::fs::read_to_string(target).ok().is_some_and(|t| serde_json::from_str::<Facet>(&t).is_ok())
}

/// Write `text` to `target` through a temporary file and a rename, so a write
/// cut short never leaves a truncated record in its place.
pub fn write_atomically(target: &Path, text: &str) -> std::io::Result<()> {
    let tmp = target.with_extension("json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, target)
}

/// The records in `dir`, and the files that would not read as one. The
/// `_`-prefixed files beside them (the insights) are not records.
pub fn read_facets(dir: &Path) -> (Vec<Facet>, Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return (Vec::new(), Vec::new()) };
    let mut paths: Vec<PathBuf> = rd
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|x| x == "json"))
        .filter(|p| !p.file_name().and_then(|n| n.to_str()).is_some_and(|n| n.starts_with('_')))
        .collect();
    paths.sort();
    let (mut found, mut rejected) = (Vec::new(), Vec::new());
    for p in paths {
        match std::fs::read_to_string(&p).ok().and_then(|t| serde_json::from_str::<Facet>(&t).ok())
        {
            Some(f) => found.push(f),
            None => rejected.push(p),
        }
    }
    found.sort_by(|a, b| a.session_id.cmp(&b.session_id));
    (found, rejected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(name: &str) -> std::path::PathBuf {
        let d = std::env::temp_dir().join(format!("facet-store-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    /// An interrupted write (Ctrl-C, a full disk) left a truncated file that
    /// counted as derived on every later run and was dropped from every report.
    #[test]
    fn a_truncated_record_is_not_derived() {
        let d = dir("trunc");
        let t = d.join("s1.json");
        std::fs::write(&t, r#"{"session_id":"#).unwrap();
        assert!(!already_derived(&t));
        std::fs::write(&t, r#"{"session_id":"s1"}"#).unwrap();
        assert!(already_derived(&t));
        assert!(!already_derived(&d.join("absent.json")));
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// A failed write is an error, not a record counted as derived.
    #[test]
    fn a_failed_write_is_reported() {
        let d = dir("ro");
        let missing = d.join("no-such-dir").join("s1.json");
        assert!(write_atomically(&missing, "{}").is_err());
        let ok = d.join("s1.json");
        write_atomically(&ok, r#"{"session_id":"s1"}"#).unwrap();
        assert!(already_derived(&ok));
        assert!(!d.join("s1.json.tmp").exists(), "no temp file is left behind");
        std::fs::remove_dir_all(&d).unwrap();
    }

    /// A record that will not read is named, not dropped without a word; the
    /// insights file beside the records is not a record.
    #[test]
    fn unreadable_records_are_named() {
        let d = dir("load");
        std::fs::write(d.join("a.json"), r#"{"session_id":"a"}"#).unwrap();
        std::fs::write(d.join("b.json"), r#"{"session_id":"#).unwrap();
        std::fs::write(d.join("_insights.json"), r#"{"recommendations":[]}"#).unwrap();
        let (found, rejected) = read_facets(&d);
        assert_eq!(found.iter().map(|f| f.session_id.as_str()).collect::<Vec<_>>(), ["a"]);
        assert_eq!(rejected, [d.join("b.json")]);
        std::fs::remove_dir_all(&d).unwrap();
    }
}
