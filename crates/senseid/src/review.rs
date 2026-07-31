//! Review risk-class gate (workstream E1).
//!
//! Decides *how hard* a change must be reviewed from the blast radius of the
//! files it touches, so the review flow can escalate the changes that matter and
//! not burn the full adversarial pass on a docs typo. This is the gate that fixes
//! "reviews gloss over": `Approve` demands the full multi-agent adversarial review
//! + human sign-off; `Review` a standard review; `Auto` can skip the heavy pass.
//!
//! Pure + fully unit-tested — the disk facts (the changed paths, the task text)
//! are injected, so the policy is deterministic and testable. Ported in spirit
//! from agent-context's `resolve_risk_class`, adapted to this repo's layout.

use serde::Serialize;

/// Review depth required for a change. Ordered `Auto < Review < Approve`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RiskClass {
    /// Low blast radius (docs, tests, config) — the heavy review pass may be skipped.
    Auto,
    /// Production source — a standard review.
    Review,
    /// Identity / auth / money / secrets / schema / governance — full adversarial
    /// review + human sign-off. Never auto-merged.
    Approve,
}

impl RiskClass {
    fn rank(self) -> u8 {
        match self {
            Self::Auto => 0,
            Self::Review => 1,
            Self::Approve => 2,
        }
    }
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Review => "review",
            Self::Approve => "approve",
        }
    }
}

/// The gate's decision: the class + the human-readable reasons that drove it.
#[derive(Debug, Clone, Serialize)]
pub struct RiskAssessment {
    pub class: RiskClass,
    pub reasons: Vec<String>,
}

/// Path substrings that force `Approve`, each with the reason to surface. High-signal
/// only — deliberately NOT bare `token`/`key` (design tokens, map keys) to avoid
/// false Approves that would erode trust in the gate.
const APPROVE_NEEDLES: &[(&str, &str)] = &[
    ("credential", "credentials"),
    ("secret", "secret material"),
    ("password", "password handling"),
    ("passwd", "password handling"),
    ("oauth", "oauth"),
    ("/auth/", "authentication"),
    ("authn", "authentication"),
    ("authz", "authorization"),
    ("/login", "login flow"),
    ("payment", "payments"),
    ("billing", "billing"),
    ("stripe", "payment provider"),
    ("invoice", "billing"),
    (".ddl", "database schema (DDL)"),
    (".sql", "SQL / schema"),
    ("database/ddl", "database schema"),
    ("migration", "database migration"),
    ("row_level", "row-level security"),
    ("/rls", "row-level security"),
    ("rbac", "access control"),
    ("permission", "permissions"),
    ("/security", "security-sensitive"),
    ("/identity", "identity"),
    ("resolution.rs", "identity resolution (fail-closed)"),
    ("keychain", "secret storage"),
    (".pem", "private key material"),
    ("private_key", "private key"),
    ("privatekey", "private key"),
    ("kavach", "auth library"),
];

/// True for a low-blast-radius path (test / doc / fixture) — checked BEFORE the
/// approve needles so a *test for* an auth flow is still `Auto` (a test doesn't ship
/// production behavior).
fn is_test_or_doc(p: &str) -> bool {
    p.ends_with(".md")
        || p.ends_with(".snap")
        || p.contains("/test")
        || p.contains("test/")
        || p.contains("_test.")
        || p.contains(".test.")
        || p.contains(".spec.")
        || p.contains("__tests__")
        || p.contains("/docs/")
        || p.starts_with("docs/")
        || p.contains("/fixtures/")
        || p.contains("/mocks/")
        || p.contains("__mocks__")
        || p.contains("/testdata/")
}

/// True for a production source file by extension — drives `Review`.
fn is_source(p: &str) -> bool {
    const EXTS: &[&str] = &[
        ".rs", ".ts", ".tsx", ".js", ".jsx", ".svelte", ".py", ".go", ".rb", ".java",
        ".kt", ".swift", ".c", ".cc", ".cpp", ".h", ".css", ".scss",
    ];
    EXTS.iter().any(|e| p.ends_with(e))
}

/// Classify one path. Highest-severity concern wins; test/doc paths short-circuit to
/// `Auto` so content keywords never re-escalate a test.
fn classify_path(path: &str) -> (RiskClass, Option<&'static str>) {
    let p = path.to_ascii_lowercase();
    if is_test_or_doc(&p) {
        return (RiskClass::Auto, None);
    }
    for (needle, reason) in APPROVE_NEEDLES {
        if p.contains(needle) {
            return (RiskClass::Approve, Some(reason));
        }
    }
    if is_source(&p) {
        return (RiskClass::Review, Some("production source"));
    }
    (RiskClass::Auto, None)
}

/// Whether the task text describes a sensitive/destructive operation — used only to
/// ESCALATE (never de-escalate) a low path-risk to `Review`.
fn task_is_sensitive(task: &str) -> bool {
    let t = task.to_ascii_lowercase();
    const KW: &[&str] = &[
        "delete", "drop ", "truncate", "wipe", "purge", "migrat", "auth", "password",
        "secret", "credential", "payment", "billing", "security", "permission",
        "rls", "encrypt", "decrypt",
    ];
    KW.iter().any(|k| t.contains(k))
}

/// Resolve the review depth for a change from its changed paths (+ optional task
/// text). Highest path-risk wins; a sensitive task escalates an otherwise-`Auto`
/// change to `Review`. Empty paths → `Review` (fail toward MORE review — we can't
/// see the blast radius, so we don't wave it through).
pub fn resolve_risk_class(paths: &[String], task: Option<&str>) -> RiskAssessment {
    if paths.is_empty() {
        return RiskAssessment {
            class: RiskClass::Review,
            reasons: vec!["no changed paths provided — cannot assess blast radius, defaulting to review".into()],
        };
    }

    let mut class = RiskClass::Auto;
    let mut approve_reasons: Vec<String> = Vec::new();
    let mut source_count = 0usize;
    for p in paths {
        let (c, reason) = classify_path(p);
        if c.rank() > class.rank() {
            class = c;
        }
        match c {
            RiskClass::Approve => approve_reasons.push(format!("{p} → {}", reason.unwrap_or("sensitive"))),
            RiskClass::Review => source_count += 1,
            RiskClass::Auto => {}
        }
    }

    let mut reasons = approve_reasons;
    if source_count > 0 {
        reasons.push(format!("{source_count} production source file(s) → review"));
    }

    if let Some(t) = task {
        if class == RiskClass::Auto && task_is_sensitive(t) {
            class = RiskClass::Review;
            reasons.push("task describes a sensitive/destructive operation → escalated to review".into());
        }
    }

    if reasons.is_empty() {
        reasons.push(format!("{} path(s), all docs/tests/config — low risk", paths.len()));
    }

    RiskAssessment { class, reasons }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn class_of(paths: &[&str]) -> RiskClass {
        resolve_risk_class(&paths.iter().map(|s| s.to_string()).collect::<Vec<_>>(), None).class
    }

    #[test]
    fn auth_and_money_and_schema_require_approve() {
        assert_eq!(class_of(&["crates/senseid/src/auth/session.rs"]), RiskClass::Approve);
        assert_eq!(class_of(&["dojo/src/lib/billing/invoice.ts"]), RiskClass::Approve);
        assert_eq!(class_of(&["database/ddl/table/sensei/folders.ddl"]), RiskClass::Approve);
        assert_eq!(class_of(&["crates/senseid/src/resolution.rs"]), RiskClass::Approve, "the fail-closed identity resolver");
        assert_eq!(class_of(&["dojo/migrations/003_rls.sql"]), RiskClass::Approve);
        assert_eq!(class_of(&["crates/senseid/src/dojo/credentials.rs"]), RiskClass::Approve);
    }

    #[test]
    fn plain_production_source_is_review() {
        assert_eq!(class_of(&["crates/senseid/src/tasks/handlers/scan.rs"]), RiskClass::Review);
        assert_eq!(class_of(&["app/src/lib/components/Card.svelte"]), RiskClass::Review);
        assert_eq!(class_of(&["dojo/src/lib/health-series.ts"]), RiskClass::Review);
    }

    #[test]
    fn docs_and_tests_and_config_are_auto() {
        assert_eq!(class_of(&["docs/spec/2026-07-31-sensei-evolution.md"]), RiskClass::Auto);
        assert_eq!(class_of(&["README.md"]), RiskClass::Auto);
        assert_eq!(class_of(&["dojo/src/lib/health-series.spec.ts"]), RiskClass::Auto);
        assert_eq!(class_of(&["crates/senseid/tests/wire.rs"]), RiskClass::Auto);
    }

    #[test]
    fn a_test_for_auth_is_still_auto_not_approve() {
        // The test/doc short-circuit must beat the auth keyword.
        assert_eq!(class_of(&["crates/senseid/src/auth/session_test.rs"]), RiskClass::Auto);
        assert_eq!(class_of(&["dojo/src/lib/auth/login.spec.ts"]), RiskClass::Auto);
    }

    #[test]
    fn design_tokens_are_not_a_false_approve() {
        // "tokens.css" must not trip an auth needle — it's UI styling (review), not secrets.
        assert_eq!(class_of(&["app/src/tokens.css"]), RiskClass::Review);
    }

    #[test]
    fn the_highest_risk_path_wins() {
        // A docs+source+auth mix classifies as the max (approve).
        assert_eq!(
            class_of(&["docs/x.md", "crates/senseid/src/util.rs", "crates/senseid/src/auth/mod.rs"]),
            RiskClass::Approve
        );
        // docs + source (no sensitive) → review.
        assert_eq!(class_of(&["docs/x.md", "crates/senseid/src/util.rs"]), RiskClass::Review);
    }

    #[test]
    fn empty_paths_fail_toward_review() {
        assert_eq!(resolve_risk_class(&[], None).class, RiskClass::Review);
    }

    #[test]
    fn a_sensitive_task_escalates_an_otherwise_auto_change() {
        let docs = vec!["docs/runbook.md".to_string()];
        assert_eq!(resolve_risk_class(&docs, Some("update the runbook")).class, RiskClass::Auto);
        assert_eq!(
            resolve_risk_class(&docs, Some("delete all stale user records")).class,
            RiskClass::Review,
            "a destructive task raises a docs-only change to review"
        );
        // But a task can never de-escalate a real approve.
        let ddl = vec!["database/ddl/x.ddl".to_string()];
        assert_eq!(resolve_risk_class(&ddl, Some("just a comment tweak")).class, RiskClass::Approve);
    }

    #[test]
    fn reasons_name_the_sensitive_files() {
        let a = resolve_risk_class(&["crates/senseid/src/auth/mod.rs".to_string()], None);
        assert_eq!(a.class, RiskClass::Approve);
        assert!(a.reasons.iter().any(|r| r.contains("auth/mod.rs") && r.contains("authentication")),
            "the approve reason names the file + why: {:?}", a.reasons);
    }

    #[test]
    fn serializes_lowercase() {
        assert_eq!(serde_json::to_value(RiskClass::Approve).unwrap(), serde_json::json!("approve"));
        assert_eq!(serde_json::to_value(RiskClass::Auto).unwrap(), serde_json::json!("auto"));
    }
}
