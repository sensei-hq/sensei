//! Library version semantics for the update scheduler (workstream F, v0 pure core).
//!
//! FAIL-CLOSED by construction: anything we can't parse as an EXACT semver — a range
//! (`^1.2.3`, `>=2.0`), a PEP440 specifier (`~=1.4`, `.*`), a git sha, `workspace:` —
//! classifies as `Unknown` → `Ignore`. We never fabricate a "newer version available".
//!
//! This is the whole feature's policy heart; v1 (auto-apply patch / compat-check minor)
//! and v2 (security) reuse `update_action` unchanged — only the caller's dispatch and
//! the `is_security` source grow later.

use semver::Version;

/// The kind of version change from current → available.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bump {
    /// available == current, or available < current (a downgrade).
    None,
    Patch,
    Minor,
    Major,
    /// Either side isn't an exact semver — cannot compare (fail-closed).
    Unknown,
}

/// What to do about a bump. `update_action` maps severity → this.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateAction {
    Ignore,
    Notify,
    AutoApply,
}

/// Parse an EXACT semver, stripping a leading `v`/`V` + surrounding whitespace.
/// Returns `None` for anything semver 2.0 can't parse exactly. Deliberately does
/// NOT strip range operators: a `^1.2.3` pin already accepts newer releases, so
/// treating it as `1.2.3` would fabricate a spurious "update available".
pub fn parse_semver(raw: &str) -> Option<Version> {
    let s = raw.trim().trim_start_matches(['v', 'V']).trim();
    Version::parse(s).ok()
}

/// Classify current → available. `None` when equal or a downgrade; `Unknown` when
/// either side isn't an exact semver (fail-closed → the caller sends no notice).
pub fn classify_bump(current: &str, available: &str) -> Bump {
    let (Some(cur), Some(avail)) = (parse_semver(current), parse_semver(available)) else {
        return Bump::Unknown;
    };
    if avail <= cur {
        Bump::None
    } else if avail.major != cur.major {
        Bump::Major
    } else if avail.minor != cur.minor {
        Bump::Minor
    } else {
        Bump::Patch
    }
}

/// The apply policy the whole feature grounds out to. A non-bump (None/Unknown) is
/// never acted on — even under `is_security` (a security flag escalates a REAL bump,
/// it doesn't invent one). v0's caller collapses `Notify | AutoApply → notify`;
/// v1 honors `AutoApply` for a patch (re-index), v2 sets `is_security` from an
/// advisory source.
pub fn update_action(bump: Bump, is_security: bool) -> UpdateAction {
    match bump {
        Bump::None | Bump::Unknown => UpdateAction::Ignore,
        _ if is_security => UpdateAction::AutoApply,
        Bump::Patch => UpdateAction::AutoApply,
        Bump::Minor | Bump::Major => UpdateAction::Notify,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_bump_by_semver_component() {
        assert_eq!(classify_bump("1.2.3", "1.2.4"), Bump::Patch);
        assert_eq!(classify_bump("1.2.3", "1.3.0"), Bump::Minor);
        assert_eq!(classify_bump("1.2.3", "2.0.0"), Bump::Major);
        assert_eq!(classify_bump("1.2.3", "1.2.3"), Bump::None, "equal → None");
        assert_eq!(classify_bump("2.0.0", "1.9.9"), Bump::None, "downgrade → None");
        assert_eq!(classify_bump("v1.2.3", "v1.2.4"), Bump::Patch, "leading v stripped");
    }

    #[test]
    fn non_exact_versions_are_unknown_never_a_bump() {
        // This is the load-bearing fail-closed guard: a RANGE pin already accepts the
        // latest, so it must NOT surface as an update. We intentionally do not strip
        // range operators (no clean_version fallback).
        assert_eq!(classify_bump("^1.2.3", "1.9.0"), Bump::Unknown, "caret range → Unknown, NOT a spurious Minor");
        assert_eq!(classify_bump(">=2.0", "3.1.0"), Bump::Unknown);
        assert_eq!(classify_bump("~=1.4", "1.9.0"), Bump::Unknown, "PEP440 → Unknown");
        assert_eq!(classify_bump("workspace:*", "1.0.0"), Bump::Unknown);
        assert_eq!(classify_bump("abc123", "1.0.0"), Bump::Unknown, "git sha → Unknown");
        assert_eq!(classify_bump("1.2.3", "not-a-version"), Bump::Unknown);
    }

    #[test]
    fn update_action_policy_truth_table() {
        // Non-bumps are never acted on, even with a security flag.
        assert_eq!(update_action(Bump::None, false), UpdateAction::Ignore);
        assert_eq!(update_action(Bump::Unknown, false), UpdateAction::Ignore);
        assert_eq!(update_action(Bump::None, true), UpdateAction::Ignore, "security never invents a bump");
        assert_eq!(update_action(Bump::Unknown, true), UpdateAction::Ignore);
        // Real bumps.
        assert_eq!(update_action(Bump::Patch, false), UpdateAction::AutoApply, "patch = v1 auto-apply target");
        assert_eq!(update_action(Bump::Minor, false), UpdateAction::Notify);
        assert_eq!(update_action(Bump::Major, false), UpdateAction::Notify);
        // Security escalates a real bump to auto-apply (v2).
        assert_eq!(update_action(Bump::Minor, true), UpdateAction::AutoApply);
        assert_eq!(update_action(Bump::Major, true), UpdateAction::AutoApply);
    }
}
