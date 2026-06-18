//! Pure, DB-free health vocabulary + freshness math for ACP adapters.
//! Everything here is unit-testable without a daemon, DB, or network.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CheckStatus { Ok, Warn, Fail, Unknown }

impl CheckStatus {
    /// Severity for "worst-of" aggregation: Fail > Warn > Unknown > Ok.
    fn rank(self) -> u8 {
        match self { CheckStatus::Ok => 0, CheckStatus::Unknown => 1, CheckStatus::Warn => 2, CheckStatus::Fail => 3 }
    }
    /// The more-severe of two statuses.
    pub fn worse(self, other: CheckStatus) -> CheckStatus {
        if other.rank() > self.rank() { other } else { self }
    }
    /// Aggregate an iterator of statuses to the worst. Empty => Ok.
    pub fn worst_of<'a>(it: impl Iterator<Item = &'a CheckStatus>) -> CheckStatus {
        it.fold(CheckStatus::Ok, |acc, s| acc.worse(*s))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterCheck {
    pub id: String,
    pub label: String,
    pub status: CheckStatus,
    pub detail: Option<String>,
}

impl AdapterCheck {
    pub fn new(id: &str, label: &str, status: CheckStatus, detail: Option<String>) -> Self {
        Self { id: id.to_string(), label: label.to_string(), status, detail }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterHealth {
    pub adapter_id: String,
    pub family: String,
    pub status: CheckStatus,
    pub checks: Vec<AdapterCheck>,
    pub resolvable: bool,
}

impl AdapterHealth {
    /// Build with `status` computed as the worst of `checks`.
    pub fn new(adapter_id: &str, family: &str, checks: Vec<AdapterCheck>, resolvable: bool) -> Self {
        let status = CheckStatus::worst_of(checks.iter().map(|c| &c.status));
        Self { adapter_id: adapter_id.to_string(), family: family.to_string(), status, checks, resolvable }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterResolveReport {
    pub adapter_id: String,
    pub ok: bool,
    pub actions: Vec<String>,
    pub errors: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worst_of_picks_fail_over_warn_over_ok() {
        let s = [CheckStatus::Ok, CheckStatus::Warn, CheckStatus::Fail];
        assert_eq!(CheckStatus::worst_of(s.iter()), CheckStatus::Fail);
        let s2 = [CheckStatus::Ok, CheckStatus::Warn];
        assert_eq!(CheckStatus::worst_of(s2.iter()), CheckStatus::Warn);
        let empty: [CheckStatus; 0] = [];
        assert_eq!(CheckStatus::worst_of(empty.iter()), CheckStatus::Ok);
    }

    #[test]
    fn unknown_is_worse_than_ok_but_better_than_warn() {
        assert_eq!(CheckStatus::Ok.worse(CheckStatus::Unknown), CheckStatus::Unknown);
        assert_eq!(CheckStatus::Unknown.worse(CheckStatus::Warn), CheckStatus::Warn);
    }

    #[test]
    fn adapter_health_status_is_worst_of_checks() {
        let checks = vec![
            AdapterCheck::new("a", "A", CheckStatus::Ok, None),
            AdapterCheck::new("b", "B", CheckStatus::Fail, Some("boom".into())),
        ];
        let h = AdapterHealth::new("claude-code", "claude", checks, true);
        assert_eq!(h.status, CheckStatus::Fail);
        assert_eq!(h.checks.len(), 2);
    }

    #[test]
    fn check_status_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&CheckStatus::Fail).unwrap(), "\"fail\"");
        assert_eq!(serde_json::to_string(&CheckStatus::Ok).unwrap(), "\"ok\"");
    }
}
