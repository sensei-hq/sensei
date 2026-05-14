//! AndChecker — every sub-checker must succeed. Used to combine binary +
//! port checks (e.g. postgres = pg_isready binary AND port 5432).

use crate::health::checker::{Checker, CheckOutcome};
use crate::health::types::ComponentStatus;

pub struct AndChecker(pub Vec<Box<dyn Checker>>);

impl Checker for AndChecker {
    fn check(&self) -> CheckOutcome {
        let mut version: Option<String> = None;
        for c in &self.0 {
            let out = c.check();
            match out.status {
                ComponentStatus::Failed => return out,
                ComponentStatus::Ready  => {
                    if version.is_none() { version = out.version; }
                }
                _ => {}
            }
        }
        CheckOutcome { status: ComponentStatus::Ready, version, detail: None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct AlwaysReady(Option<String>);
    impl Checker for AlwaysReady {
        fn check(&self) -> CheckOutcome {
            CheckOutcome { status: ComponentStatus::Ready, version: self.0.clone(), detail: None }
        }
    }
    struct AlwaysFailed(&'static str);
    impl Checker for AlwaysFailed {
        fn check(&self) -> CheckOutcome { CheckOutcome::failed(self.0.to_string()) }
    }

    #[test]
    fn all_ready_yields_ready() {
        let c = AndChecker(vec![
            Box::new(AlwaysReady(None)),
            Box::new(AlwaysReady(Some("1.0".into()))),
        ]);
        let o = c.check();
        assert!(matches!(o.status, ComponentStatus::Ready));
        assert_eq!(o.version.as_deref(), Some("1.0"));
    }

    #[test]
    fn first_failed_short_circuits() {
        let c = AndChecker(vec![
            Box::new(AlwaysFailed("binary missing")),
            Box::new(AlwaysReady(None)),
        ]);
        let o = c.check();
        assert!(matches!(o.status, ComponentStatus::Failed));
        assert_eq!(o.detail.as_deref(), Some("binary missing"));
    }

    #[test]
    fn empty_andchecker_is_vacuously_ready() {
        let c = AndChecker(vec![]);
        let o = c.check();
        assert!(matches!(o.status, ComponentStatus::Ready));
        assert!(o.version.is_none());
    }

    #[test]
    fn version_from_first_versioned_sub() {
        let c = AndChecker(vec![
            Box::new(AlwaysReady(None)),
            Box::new(AlwaysReady(Some("first-version".into()))),
            Box::new(AlwaysReady(Some("second-version".into()))),
        ]);
        let o = c.check();
        assert_eq!(o.version.as_deref(), Some("first-version"));
    }
}
