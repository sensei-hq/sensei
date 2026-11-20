//! The lane's host (#455): which editor a lane runs in.
//!
//! The MCP binary detects the editor (client name, then `TERM_PROGRAM`) and sends it
//! as `host_detected`; the lane may also report a `host` itself. [`resolve_host`]
//! decides which one the lane keeps: a detection the catalogue knows as an editor
//! wins, the self-report is the fallback, and nothing is ever guessed.
use crate::assistants::catalogue::CatalogueNames;

/// What [`resolve_host`] decided, and what it set aside.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedHost {
    /// The host the lane keeps; none for a terminal lane.
    pub host: Option<String>,
    /// A disagreement worth a warning, when there was one.
    pub note: Option<HostNote>,
}

/// Why a register's host values disagreed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostNote {
    /// Both were given and differ; the detected one won.
    Mismatch { reported: String, detected: String },
    /// The detected value is not an editor the catalogue knows; it was dropped.
    IgnoredDetection { detected: String },
}

/// The lane's host from its self-report and the MCP binary's detection.
pub fn resolve_host(
    _reported: Option<&str>,
    _detected: Option<&str>,
    _catalogue: &CatalogueNames,
) -> ResolvedHost {
    ResolvedHost { host: None, note: None }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistants::catalogue::CatalogueWordRow;

    fn catalogue() -> CatalogueNames {
        let row = |word: &str, id: &str, name: &str, kind: &str| CatalogueWordRow {
            word: word.into(),
            id: id.into(),
            name: name.into(),
            kind: kind.into(),
            joins_crew: true,
        };
        CatalogueNames::from_rows(vec![
            row("zed", "zed", "Zed", "editor"),
            row("cursor", "cursor", "Cursor", "editor"),
            row("claude", "claude", "Claude Code", "assistant"),
            row("claude-code", "claude", "Claude Code", "assistant"),
        ])
    }

    fn resolved(host: Option<&str>, note: Option<HostNote>) -> ResolvedHost {
        ResolvedHost { host: host.map(str::to_string), note }
    }

    #[test]
    fn a_detected_editor_wins_over_a_different_report() {
        assert_eq!(
            resolve_host(Some("cursor"), Some("zed"), &catalogue()),
            resolved(
                Some("zed"),
                Some(HostNote::Mismatch { reported: "cursor".into(), detected: "zed".into() })
            )
        );
    }

    #[test]
    fn the_same_host_both_ways_is_no_mismatch() {
        assert_eq!(
            resolve_host(Some(" zed "), Some("zed"), &catalogue()),
            resolved(Some("zed"), None)
        );
    }

    #[test]
    fn a_detection_alone_is_the_host() {
        assert_eq!(resolve_host(None, Some("zed"), &catalogue()), resolved(Some("zed"), None));
        assert_eq!(
            resolve_host(Some("  "), Some("zed"), &catalogue()),
            resolved(Some("zed"), None)
        );
    }

    #[test]
    fn the_report_is_the_fallback() {
        assert_eq!(
            resolve_host(Some(" cursor "), None, &catalogue()),
            resolved(Some("cursor"), None)
        );
        assert_eq!(
            resolve_host(Some("cursor"), Some("  "), &catalogue()),
            resolved(Some("cursor"), None)
        );
    }

    #[test]
    fn a_detection_that_is_not_a_catalogue_editor_is_ignored() {
        for detected in ["muse", "claude"] {
            assert_eq!(
                resolve_host(Some("cursor"), Some(detected), &catalogue()),
                resolved(
                    Some("cursor"),
                    Some(HostNote::IgnoredDetection { detected: detected.into() })
                ),
                "{detected}"
            );
        }
        assert_eq!(
            resolve_host(None, Some("muse"), &catalogue()),
            resolved(None, Some(HostNote::IgnoredDetection { detected: "muse".into() }))
        );
    }

    #[test]
    fn nothing_given_is_no_host() {
        assert_eq!(resolve_host(None, None, &catalogue()), resolved(None, None));
        assert_eq!(resolve_host(Some(""), None, &catalogue()), resolved(None, None));
    }

    #[test]
    fn an_unknown_reported_word_is_kept_as_given() {
        assert_eq!(resolve_host(Some("helix"), None, &catalogue()), resolved(Some("helix"), None));
    }
}
