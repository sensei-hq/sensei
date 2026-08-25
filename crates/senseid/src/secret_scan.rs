//! Secret detection for the memory / governance write guard (workstream C4).
//!
//! `save_memory` / `propose_memory` write into the shared `sensei.memories` store,
//! which federates and drives governance. A secret must never land there. This
//! module is the single, reusable detector — the write handler fails CLOSED on a
//! hit (rejects the write, surfacing only the secret *kind*, never the value).
//!
//! Patterns are deliberately HIGH-SIGNAL: unambiguous prefixed tokens, key-material
//! blocks, and a secret-named assignment with a substantial value. Because the guard
//! rejects, a false positive blocks a legitimate memory — so breadth is traded for
//! precision.
//!
//! NOTE: the dōjō federation strip pipeline (`crates/senseid/src/dojo/contribute.rs`,
//! `collective/anonymize.rs`) has its own embedded secret-survival logic. That path
//! should migrate onto this shared detector rather than keep a parallel copy — filed
//! as a follow-up, not duplicated here.

use regex::Regex;
use std::sync::OnceLock;

/// A detected secret: its kind and a short REDACTED preview (never the full value).
#[derive(Debug, Clone, PartialEq)]
pub struct SecretFinding {
    pub kind: &'static str,
    pub preview: String,
}

fn patterns() -> &'static Vec<(&'static str, Regex)> {
    static P: OnceLock<Vec<(&'static str, Regex)>> = OnceLock::new();
    P.get_or_init(|| {
        vec![
            ("private key block", Regex::new(r"-----BEGIN [A-Z ]*PRIVATE KEY-----").unwrap()),
            ("aws access key id", Regex::new(r"\bAKIA[0-9A-Z]{16}\b").unwrap()),
            ("github token", Regex::new(r"\bgh[pousr]_[A-Za-z0-9]{36,}\b").unwrap()),
            ("github fine-grained pat", Regex::new(r"\bgithub_pat_[A-Za-z0-9_]{22,}").unwrap()),
            ("slack token", Regex::new(r"\bxox[baprs]-[A-Za-z0-9-]{10,}\b").unwrap()),
            ("stripe secret key", Regex::new(r"\b[rs]k_live_[A-Za-z0-9]{16,}\b").unwrap()),
            ("google api key", Regex::new(r"\bAIza[0-9A-Za-z_\-]{35}\b").unwrap()),
            ("anthropic api key", Regex::new(r"\bsk-ant-[A-Za-z0-9_\-]{20,}").unwrap()),
            ("openai api key", Regex::new(r"\bsk-[A-Za-z0-9]{32,}\b").unwrap()),
            (
                "secret assignment",
                Regex::new(r#"(?i)(?:password|secret|api[_-]?key|access[_-]?token|client[_-]?secret|credential)\s*[:=]\s*["']?(?P<val>[A-Za-z0-9/+_\-]{12,})"#).unwrap(),
            ),
        ]
    })
}

/// True for an env-var / constant NAME (`SENSEI_API_KEY`, `DATABASE_URL`) — a
/// SCREAMING_SNAKE identifier. Memories often document these; they are not secret
/// VALUES, so a `key: NAME` assignment to one must not be flagged.
fn is_env_var_name(val: &str) -> bool {
    !val.is_empty()
        && val.starts_with(|c: char| c.is_ascii_uppercase())
        && val.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// Redact a matched secret to a safe preview: first 4 chars + an ellipsis.
fn redact(s: &str) -> String {
    let head: String = s.chars().take(4).collect();
    format!("{head}…")
}

/// Scan `text` for secrets. Returns one finding per distinct kind (kind + redacted
/// preview). Empty when clean.
pub fn scan(text: &str) -> Vec<SecretFinding> {
    let mut out: Vec<SecretFinding> = Vec::new();
    for (kind, re) in patterns() {
        if out.iter().any(|f| f.kind == *kind) {
            continue;
        }
        // The assignment pattern is the low-confidence one: skip a value that is a
        // SCREAMING_SNAKE env-var/constant NAME (documented, not a secret value).
        if *kind == "secret assignment" {
            if let Some(caps) = re.captures(text) {
                let val = caps.name("val").map(|m| m.as_str()).unwrap_or("");
                if !is_env_var_name(val) {
                    out.push(SecretFinding { kind, preview: redact(val) });
                }
            }
            continue;
        }
        if let Some(m) = re.find(text) {
            out.push(SecretFinding { kind, preview: redact(m.as_str()) });
        }
    }
    out
}

/// True if `text` carries at least one secret. Consistent with [`scan`] (applies the
/// same env-var-name exclusion). Public boolean entrypoint for callers that only
/// need presence; the memory guard uses [`scan`] directly (it surfaces the kinds).
#[allow(dead_code)]
pub fn has_secret(text: &str) -> bool {
    !scan(text).is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(text: &str) -> Vec<&'static str> {
        scan(text).into_iter().map(|f| f.kind).collect()
    }

    #[test]
    fn detects_prefixed_provider_tokens() {
        // Fixtures are built from split parts at runtime so no contiguous secret-format
        // literal appears in the SOURCE (GitHub push protection would otherwise flag the
        // test data as a real secret). Low-entropy bodies keep them obviously-fake.
        let ghp = format!("ghp_{}", "b".repeat(38));
        assert!(kinds(&format!("token {ghp}")).contains(&"github token"));
        let aws = format!("AKIA{}", "A".repeat(16));
        assert!(kinds(&aws).contains(&"aws access key id"));
        let slack = format!("xox{}-{}", "b", "0".repeat(24));
        assert!(kinds(&slack).contains(&"slack token"));
        let stripe = format!("sk_live_{}", "0".repeat(20));
        assert!(kinds(&stripe).contains(&"stripe secret key"));
        let google = format!("AIza{}", "X".repeat(35));
        assert!(kinds(&google).contains(&"google api key"));
        let anthropic = format!("sk-ant-{}", "0".repeat(24));
        assert!(kinds(&anthropic).contains(&"anthropic api key"));
    }

    #[test]
    fn detects_private_key_block_and_secret_assignment() {
        assert!(kinds("-----BEGIN RSA PRIVATE KEY-----\nMIIB...").contains(&"private key block"));
        assert!(kinds("API_KEY = \"abcd1234efgh5678\"").contains(&"secret assignment"));
        assert!(kinds("password: hunter2longenough").contains(&"secret assignment"));
    }

    #[test]
    fn clean_prose_and_short_values_are_not_flagged() {
        assert!(
            scan("The importer resolves the folder by abs_path and reuses the shared helper.")
                .is_empty()
        );
        assert!(scan("set the timeout to 30 and retries to 3").is_empty());
        // A short/placeholder value under the length floor is not flagged.
        assert!(scan("password: x").is_empty());
        // 'token' as a design concept (not an assignment) is fine.
        assert!(scan("the design tokens live in tokens.css").is_empty());
    }

    #[test]
    fn documenting_an_env_var_name_is_not_a_secret() {
        // A memory that documents the env-var/constant NAME (not its value) must pass.
        assert!(scan("the api_key comes from SENSEI_API_KEY").is_empty());
        assert!(scan("secret = DATABASE_PASSWORD_ENV").is_empty());
        assert!(scan("credential: OAUTH_CLIENT_SECRET").is_empty());
        // ...but an actual secret-looking value in the same shape IS flagged.
        assert!(kinds("api_key = aB3xY9kLmN2pQr7s").contains(&"secret assignment"));
    }

    #[test]
    fn preview_never_leaks_the_full_secret() {
        let ghp = format!("ghp_{}", "b".repeat(38));
        let f = &scan(&ghp)[0];
        assert!(!f.preview.contains("bbbb"), "preview is redacted, not the body: {}", f.preview);
        assert_eq!(f.preview, "ghp_…");
    }

    #[test]
    fn has_secret_agrees_with_scan() {
        assert!(has_secret(&format!("ghp_{}", "b".repeat(38))));
        assert!(!has_secret("just some ordinary memory content"));
    }
}
