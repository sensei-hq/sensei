//! GenericPrerequisite — composes a Checker and a Fixer into a Prerequisite.

use super::{CheckResult, FixResult, GateKind, Prerequisite, Remedy};
use super::checker::Checker;
use super::fixer::Fixer;

/// A prerequisite built by composing a Checker with a Fixer.
pub struct GenericPrerequisite {
    id: String,
    label: String,
    checker: Box<dyn Checker>,
    fixer: Box<dyn Fixer>,
    gate_kind: GateKind,
    remedy: Option<Remedy>,
}

impl GenericPrerequisite {
    pub fn new(
        id: impl Into<String>,
        label: impl Into<String>,
        checker: Box<dyn Checker>,
        fixer: Box<dyn Fixer>,
        gate_kind: GateKind,
        remedy: Option<Remedy>,
    ) -> Self {
        Self { id: id.into(), label: label.into(), checker, fixer, gate_kind, remedy }
    }
}

impl Prerequisite for GenericPrerequisite {
    fn id(&self) -> &str { &self.id }
    fn label(&self) -> &str { &self.label }
    fn check(&self) -> CheckResult { self.checker.check() }
    fn fix(&self) -> Result<FixResult, String> { self.fixer.fix() }
    fn gate_kind(&self) -> GateKind { self.gate_kind.clone() }
    fn remedy(&self) -> Option<&Remedy> { self.remedy.as_ref() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::super::checker::Checker;
    use super::super::fixer::Fixer;

    struct ReadyChecker;
    impl Checker for ReadyChecker {
        fn check(&self) -> CheckResult { CheckResult::ok("1.2.3") }
    }

    struct FailChecker;
    impl Checker for FailChecker {
        fn check(&self) -> CheckResult { CheckResult::fail("binary not found") }
    }

    struct SuccessFixer;
    impl Fixer for SuccessFixer {
        fn fix(&self) -> Result<FixResult, String> { Ok(FixResult::new("installed via test")) }
    }

    struct FailFixer;
    impl Fixer for FailFixer {
        fn fix(&self) -> Result<FixResult, String> { Err("install failed".into()) }
    }

    fn make(checker: Box<dyn Checker>, fixer: Box<dyn Fixer>) -> GenericPrerequisite {
        GenericPrerequisite::new("test-gate", "Test Gate", checker, fixer, GateKind::Install, None)
    }

    #[test]
    fn id_and_label_stored() {
        let p = make(Box::new(ReadyChecker), Box::new(SuccessFixer));
        assert_eq!(p.id(), "test-gate");
        assert_eq!(p.label(), "Test Gate");
    }

    #[test]
    fn check_delegates_to_checker_ready() {
        let p = make(Box::new(ReadyChecker), Box::new(SuccessFixer));
        let r = p.check();
        assert!(r.ok);
        assert_eq!(r.version.as_deref(), Some("1.2.3"));
    }

    #[test]
    fn check_delegates_to_checker_fail() {
        let p = make(Box::new(FailChecker), Box::new(SuccessFixer));
        let r = p.check();
        assert!(!r.ok);
        assert_eq!(r.error.as_deref(), Some("binary not found"));
    }

    #[test]
    fn fix_success_returns_approach() {
        let p = make(Box::new(FailChecker), Box::new(SuccessFixer));
        let r = p.fix();
        assert!(r.is_ok());
        assert_eq!(r.unwrap().approach, "installed via test");
    }

    #[test]
    fn fix_failure_returns_err() {
        let p = make(Box::new(FailChecker), Box::new(FailFixer));
        assert!(p.fix().is_err());
    }

    #[test]
    fn gate_kind_returned() {
        let p = GenericPrerequisite::new(
            "svc", "Svc", Box::new(ReadyChecker), Box::new(SuccessFixer), GateKind::Service, None,
        );
        assert_eq!(p.gate_kind(), GateKind::Service);
    }

    #[test]
    fn remedy_none_by_default() {
        let p = make(Box::new(ReadyChecker), Box::new(SuccessFixer));
        assert!(p.remedy().is_none());
    }

    #[test]
    fn remedy_returned_when_set() {
        let remedy = Remedy { title: "Install".into(), command: "brew install foo".into(), url: None };
        let p = GenericPrerequisite::new(
            "foo", "Foo", Box::new(ReadyChecker), Box::new(SuccessFixer),
            GateKind::Install, Some(remedy),
        );
        assert_eq!(p.remedy().unwrap().command, "brew install foo");
    }
}
