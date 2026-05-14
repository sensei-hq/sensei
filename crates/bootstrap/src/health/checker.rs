//! Checker trait — platform implementations build concrete checkers and
//! bind them to dependency ids via PlatformProvider::checker_for(id).

use super::types::ComponentStatus;

#[derive(Debug, Clone)]
pub struct CheckOutcome {
    pub status:  ComponentStatus,  // Ready or Failed (Pending/Checking are orchestrator-set)
    pub version: Option<String>,
    pub detail:  Option<String>,   // failure detail when status==Failed
}

impl CheckOutcome {
    pub fn ready(version: impl Into<String>) -> Self {
        Self { status: ComponentStatus::Ready, version: Some(version.into()), detail: None }
    }
    pub fn ready_no_version() -> Self {
        Self { status: ComponentStatus::Ready, version: None, detail: None }
    }
    pub fn failed(detail: impl Into<String>) -> Self {
        Self { status: ComponentStatus::Failed, version: None, detail: Some(detail.into()) }
    }
}

pub trait Checker: Send + Sync {
    fn check(&self) -> CheckOutcome;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ready_constructor() {
        let o = CheckOutcome::ready("16.3");
        assert!(matches!(o.status, ComponentStatus::Ready));
        assert_eq!(o.version.as_deref(), Some("16.3"));
        assert!(o.detail.is_none());
    }

    #[test]
    fn ready_no_version_constructor() {
        let o = CheckOutcome::ready_no_version();
        assert!(matches!(o.status, ComponentStatus::Ready));
        assert!(o.version.is_none());
        assert!(o.detail.is_none());
    }

    #[test]
    fn failed_constructor() {
        let o = CheckOutcome::failed("pg_isready returned 1");
        assert!(matches!(o.status, ComponentStatus::Failed));
        assert!(o.version.is_none());
        assert_eq!(o.detail.as_deref(), Some("pg_isready returned 1"));
    }
}
