//! Security-advisory source for the update scheduler (workstream F, v2).
//!
//! Pure OSV parsing/verdict helpers (unit-tested against sample bodies, no
//! network) + a [`VulnSource`] trait so the scheduler is stub-testable. Mirrors
//! [`super::registry`].
//!
//! FAIL-CLOSED by construction: any fetch/parse miss, an unmapped ecosystem, or
//! a non-exact pin yields NO advisories → `is_security` stays false → the safe v1
//! behavior. A security flag only ever ESCALATES a real bump (via
//! [`super::version::update_action`]); it never invents one, and the apply path is
//! ALWAYS docs/skills refresh only — never the consuming project's code.

use async_trait::async_trait;
use semver::Version;

use super::version::parse_semver;

/// One vulnerability advisory affecting a library. Version-agnostic: the raw set
/// is resolved once per library and a per-pin verdict is recomputed from the
/// pin's `current` version (so two projects pinning the same lib at different
/// versions never cross-contaminate).
#[derive(Debug, Clone)]
pub struct Advisory {
    pub id: String,
    /// HIGH/CRITICAL severity (label-based; see [`is_high_severity`]).
    pub high: bool,
    /// `introduced..fixed` intervals from `affected[].ranges[].events`.
    pub ranges: Vec<AffectedRange>,
}

/// A single `introduced..fixed` interval. `introduced == None` means "from the
/// beginning" (OSV `"0"`); `fixed == None` means unfixed (an upgrade can't
/// resolve it → never drives `is_security`).
#[derive(Debug, Clone)]
pub struct AffectedRange {
    pub introduced: Option<Version>,
    pub fixed: Option<Version>,
}

/// The per-pin security decision.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityVerdict {
    pub is_security: bool,
    /// The advisory id driving the flag (for the recommendation payload).
    pub top: Option<String>,
}

/// Resolves the advisory set for a library. `None` = undetermined (fail-closed →
/// no security flag); `Some(vec![])` = determined clean. A trait so the scheduler
/// is stub-testable.
#[async_trait]
pub trait VulnSource: Send + Sync {
    async fn advisories(&self, ecosystem: &str, name: &str) -> Option<Vec<Advisory>>;
}

/// Map sensei's ecosystem label to OSV's. `None` for anything OSV/we don't cover
/// (→ no scan, fail-closed). OSV uses `PyPI` / `crates.io` / `Go`, not our labels.
pub fn osv_ecosystem(ecosystem: &str) -> Option<&'static str> {
    match ecosystem {
        "npm" => Some("npm"),
        "pypi" => Some("PyPI"),
        "cargo" => Some("crates.io"),
        "go" => Some("Go"),
        _ => None,
    }
}

/// The OSV `/v1/query` POST body for a package's FULL (version-less) advisory set.
pub fn osv_query_body(osv_ecosystem: &str, name: &str) -> String {
    serde_json::json!({ "package": { "ecosystem": osv_ecosystem, "name": name } }).to_string()
}

/// HIGH severity iff a `database_specific.severity` label (at the vuln OR an
/// `affected[]` entry) is `HIGH`/`CRITICAL`. CVSS-vector base-score parsing is a
/// documented follow-up; absent a label, severity is INDETERMINATE → treated as
/// not-high (degrade, never fabricate a score).
pub fn is_high_severity(vuln: &serde_json::Value) -> bool {
    fn label_high(v: &serde_json::Value) -> bool {
        v.get("database_specific")
            .and_then(|d| d.get("severity"))
            .and_then(|s| s.as_str())
            .map(|s| {
                let u = s.to_ascii_uppercase();
                u == "HIGH" || u == "CRITICAL"
            })
            .unwrap_or(false)
    }
    if label_high(vuln) {
        return true;
    }
    vuln.get("affected")
        .and_then(|a| a.as_array())
        .is_some_and(|affected| affected.iter().any(label_high))
}

/// Parse an OSV `/v1/query` response body into advisories. `None` on a JSON
/// parse miss (fail-closed); an absent `vulns` key = determined clean (`[]`).
pub fn extract_advisories(body: &str) -> Option<Vec<Advisory>> {
    let v: serde_json::Value = serde_json::from_str(body).ok()?;
    let Some(vulns) = v.get("vulns").and_then(|x| x.as_array()) else {
        return Some(vec![]); // OSV omits `vulns` when there are none
    };
    let mut out = Vec::with_capacity(vulns.len());
    for vuln in vulns {
        let id = vuln.get("id").and_then(|x| x.as_str()).unwrap_or("").to_string();
        let high = is_high_severity(vuln);
        let mut ranges = Vec::new();
        if let Some(affected) = vuln.get("affected").and_then(|a| a.as_array()) {
            for aff in affected {
                let Some(rs) = aff.get("ranges").and_then(|r| r.as_array()) else { continue };
                for r in rs {
                    // Only version-ordered ranges; ECOSYSTEM versions that aren't
                    // semver simply fail parse_semver → an unusable (no-fixed) range.
                    let rtype = r.get("type").and_then(|t| t.as_str()).unwrap_or("");
                    if rtype != "SEMVER" && rtype != "ECOSYSTEM" {
                        continue;
                    }
                    let Some(events) = r.get("events").and_then(|e| e.as_array()) else { continue };
                    let mut introduced: Option<Version> = None;
                    for ev in events {
                        if let Some(i) = ev.get("introduced").and_then(|x| x.as_str()) {
                            introduced = if i == "0" { None } else { parse_semver(i) };
                        }
                        if let Some(f) = ev.get("fixed").and_then(|x| x.as_str()) {
                            ranges.push(AffectedRange { introduced: introduced.clone(), fixed: parse_semver(f) });
                        }
                    }
                }
            }
        }
        out.push(Advisory { id, high, ranges });
    }
    Some(out)
}

/// True if `current` falls in this range: `introduced <= current < fixed`. An
/// unfixed range (`fixed == None`) is never "fixable by an upgrade", so it never
/// counts here.
fn range_affects<'a>(range: &'a AffectedRange, current: &Version) -> Option<&'a Version> {
    let after_introduced = range.introduced.as_ref().is_none_or(|i| current >= i);
    match &range.fixed {
        Some(fixed) if after_introduced && current < fixed => Some(fixed),
        _ => None,
    }
}

/// Per-pin verdict: `is_security` iff some HIGH advisory AFFECTS `current` AND is
/// FIXED at a version `<= available` (so the available upgrade actually resolves
/// it). Non-exact `current`/`available` → no verdict (fail-closed). The security
/// flag escalates a REAL bump only; a `None`/`Unknown` bump stays `Ignore` in
/// [`super::version::update_action`] regardless.
pub fn security_verdict(advisories: &[Advisory], current: &str, available: &str) -> SecurityVerdict {
    let (Some(cur), Some(avail)) = (parse_semver(current), parse_semver(available)) else {
        return SecurityVerdict { is_security: false, top: None };
    };
    for adv in advisories.iter().filter(|a| a.high) {
        for range in &adv.ranges {
            if let Some(fixed) = range_affects(range, &cur) {
                if *fixed <= avail {
                    return SecurityVerdict { is_security: true, top: Some(adv.id.clone()) };
                }
            }
        }
    }
    SecurityVerdict { is_security: false, top: None }
}

/// Production [`VulnSource`] — OSV.dev `/v1/query` (no auth). FAIL-CLOSED: unmapped
/// ecosystem, network error, non-2xx, or a parse miss all return `None`.
pub struct OsvVulnSource;

#[async_trait]
impl VulnSource for OsvVulnSource {
    async fn advisories(&self, ecosystem: &str, name: &str) -> Option<Vec<Advisory>> {
        let osv_eco = osv_ecosystem(ecosystem)?;
        let client = reqwest::Client::builder().build().ok()?;
        let resp = client
            .post("https://api.osv.dev/v1/query")
            .header("Content-Type", "application/json")
            .header("User-Agent", "sensei-daemon")
            .body(osv_query_body(osv_eco, name))
            .send()
            .await
            .ok()?;
        if !resp.status().is_success() {
            return None;
        }
        let body = resp.text().await.ok()?;
        extract_advisories(&body)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn osv_ecosystem_maps_covered_and_rejects_others() {
        assert_eq!(osv_ecosystem("npm"), Some("npm"));
        assert_eq!(osv_ecosystem("pypi"), Some("PyPI"));
        assert_eq!(osv_ecosystem("cargo"), Some("crates.io"));
        assert_eq!(osv_ecosystem("go"), Some("Go"));
        assert_eq!(osv_ecosystem("maven"), None, "uncovered ecosystem → no scan (fail-closed)");
        assert_eq!(osv_ecosystem("docs"), None);
    }

    #[test]
    fn osv_query_body_is_versionless_package_query() {
        let b: serde_json::Value = serde_json::from_str(&osv_query_body("npm", "lodash")).unwrap();
        assert_eq!(b["package"]["ecosystem"], "npm");
        assert_eq!(b["package"]["name"], "lodash");
        assert!(b.get("version").is_none(), "version-less: the full advisory set");
    }

    #[test]
    fn is_high_severity_reads_label_at_vuln_and_affected_level() {
        let vuln_lvl = serde_json::json!({ "database_specific": { "severity": "HIGH" } });
        assert!(is_high_severity(&vuln_lvl));
        let critical = serde_json::json!({ "database_specific": { "severity": "critical" } });
        assert!(is_high_severity(&critical), "case-insensitive; CRITICAL is high");
        let aff_lvl = serde_json::json!({ "affected": [{ "database_specific": { "severity": "HIGH" } }] });
        assert!(is_high_severity(&aff_lvl), "an affected-entry label also counts");
        let moderate = serde_json::json!({ "database_specific": { "severity": "MODERATE" } });
        assert!(!is_high_severity(&moderate));
        let no_label = serde_json::json!({ "severity": [{ "type": "CVSS_V3", "score": "CVSS:3.1/AV:N/AC:L" }] });
        assert!(!is_high_severity(&no_label), "no label → indeterminate → not high (degrade)");
    }

    #[test]
    fn extract_advisories_parses_ranges_and_empty() {
        // No `vulns` key → clean.
        assert_eq!(extract_advisories(r#"{}"#).unwrap().len(), 0);
        // Parse miss → None (fail-closed).
        assert!(extract_advisories("not json").is_none());
        // A HIGH advisory fixed at 4.17.21, introduced at the beginning.
        let body = r#"{"vulns":[{
            "id":"GHSA-x","database_specific":{"severity":"HIGH"},
            "affected":[{"ranges":[{"type":"SEMVER","events":[{"introduced":"0"},{"fixed":"4.17.21"}]}]}]
        }]}"#;
        let advs = extract_advisories(body).unwrap();
        assert_eq!(advs.len(), 1);
        assert_eq!(advs[0].id, "GHSA-x");
        assert!(advs[0].high);
        assert_eq!(advs[0].ranges.len(), 1);
        assert!(advs[0].ranges[0].introduced.is_none(), "\"0\" → from the beginning");
        assert_eq!(advs[0].ranges[0].fixed.as_ref().unwrap().to_string(), "4.17.21");
    }

    fn high_adv(introduced: Option<&str>, fixed: Option<&str>) -> Advisory {
        Advisory {
            id: "GHSA-x".into(),
            high: true,
            ranges: vec![AffectedRange {
                introduced: introduced.and_then(parse_semver),
                fixed: fixed.and_then(parse_semver),
            }],
        }
    }

    #[test]
    fn security_verdict_flags_only_affected_and_fixable() {
        let advs = vec![high_adv(None, Some("4.17.21"))];
        // current 4.17.20 is affected (<fixed) and the upgrade to 4.17.21 fixes it.
        let v = security_verdict(&advs, "4.17.20", "4.17.21");
        assert!(v.is_security);
        assert_eq!(v.top.as_deref(), Some("GHSA-x"));
        // current already at/after the fix → not affected.
        assert!(!security_verdict(&advs, "4.17.21", "4.17.22").is_security, "already patched → not flagged");
        // available doesn't reach the fix → the upgrade wouldn't resolve it.
        assert!(!security_verdict(&advs, "4.17.20", "4.17.20").is_security);
    }

    #[test]
    fn security_verdict_is_fail_closed() {
        let advs = vec![high_adv(None, Some("4.17.21"))];
        // Non-exact current/available → no verdict.
        assert!(!security_verdict(&advs, "^4.0.0", "4.17.21").is_security, "range pin → no verdict");
        assert!(!security_verdict(&advs, "4.17.20", "not-a-version").is_security);
        // A LOW/indeterminate advisory (high=false) never flags.
        let low = vec![Advisory { high: false, ..high_adv(None, Some("4.17.21")) }];
        assert!(!security_verdict(&low, "4.17.20", "4.17.21").is_security);
        // An UNFIXED high advisory can't be resolved by an upgrade → never flags.
        let unfixed = vec![high_adv(None, None)];
        assert!(!security_verdict(&unfixed, "4.17.20", "4.17.21").is_security);
        // Empty set → clean.
        assert!(!security_verdict(&[], "4.17.20", "4.17.21").is_security);
    }
}
