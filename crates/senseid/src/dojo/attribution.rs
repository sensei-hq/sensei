//! Deterministic client-work DEREFERENCE — the confidentiality SAFETY NET.
//!
//! This is the hardest-stakes code in the Dōjō layer: client identifiers must
//! NEVER leave the machine. The rule (`docs/spec/pipeline/dojo-lifecycle.md`
//! → Attribution): "Client work | **Source-dereferenced** | Source reference
//! (repo, identifiers, pointer). Learning + cause + context travel." The wrong
//! gate: "A client-project memory got published with its source repo identifiers
//! intact ... this is the confidentiality gate; nothing else matters more."
//!
//! ## Two design commitments
//!
//! 1. **Deterministic, no LLM.** This layer NEVER calls a model. It is a pure
//!    function of `(text, ProjectIdentifiers)`; it always runs; it is the
//!    guarantee. The LLM prose-generalisation in [`crate::collective::anonymize`]
//!    sits ON TOP of this for readability and is never trusted to strip.
//! 2. **Fail closed.** After stripping, a heuristic re-scans the output for any
//!    surviving identifier (an absolute path, a still-present known token, a
//!    uuid / email / session id, or a repo-ish secret token). If anything looks
//!    like an identifier, [`DereferenceResult::residual_risk`] is `true` and the
//!    caller MUST refuse to publish. Uncertainty never leaks.
//!
//! ## Matching strategy
//!
//! Generic leak vectors are matched by regex even when NOT in the known set
//! (defense in depth): absolute filesystem paths (POSIX `/…`, `~/…`, Windows
//! `C:\…`), email addresses, uuids, and `s-1234` / `sess_…` session ids.
//!
//! Known identifiers ([`ProjectIdentifiers`] — project name, client/org name,
//! repo names, folder paths, session ids, person names) are matched in every
//! realistic form: exact, snake_case, kebab-case, camelCase, PascalCase, bare,
//! and as a distinctive substring within a longer token. A multi-word token
//! (e.g. `acme-api`) is matched aggressively (its words in sequence separated by
//! optional `-`/`_`/`.`/space, case-insensitively) so `acmeApi`, `acme_api`, and
//! the `acmeApi` prefix of `acmeApiClient` all fall. A single-word token
//! (e.g. `acme`) is matched at a leading boundary; the residual squash-check is
//! the backstop for anything a boundary miss lets survive.
//!
//! Forward seam: C5 defines and exhaustively tests this; its first non-test
//! caller is the upstream publish path (C6). The module surface is
//! `dead_code`-allowed until that chunk wires it in.
#![allow(dead_code)]

use regex::{Captures, Regex};
use std::cell::Cell;
use std::sync::LazyLock;

/// Neutral placeholders the strip replaces sensitive tokens with. Chosen so no
/// realistic identifier collides with the placeholder's own letters, and so the
/// residual squash-check can subtract them before scanning.
const PH_PATH: &str = "<path>";
const PH_EMAIL: &str = "<email>";
const PH_UUID: &str = "<id>";
const PH_SESSION: &str = "<session>";
const PH_PROJECT: &str = "<project>";
const PH_CLIENT: &str = "<client>";
const PH_REPO: &str = "<repo>";
const PH_PERSON: &str = "<person>";

/// Every placeholder — subtracted from the output before the residual
/// squash-check so a placeholder's own letters can't be mistaken for a leak.
const ALL_PLACEHOLDERS: [&str; 8] =
    [PH_PATH, PH_EMAIL, PH_UUID, PH_SESSION, PH_PROJECT, PH_CLIENT, PH_REPO, PH_PERSON];

/// Minimum alnum length for a known token to be treated as "distinctive". Below
/// this a token would over-match ordinary prose to the point of uselessness
/// (every English word contains 2-char runs). Ultra-short identifiers are out of
/// scope — a deliberate, documented floor (see module ambiguity notes).
const MIN_DISTINCTIVE_SQUASH: usize = 3;

// ── Generic leak-vector regexes (also power the residual check) ─────────────

/// POSIX absolute path: two or more `/segment` runs (`/Users/jerry/acme-api`).
/// Requiring `{2,}` segments avoids nuking a lone `TCP/IP`-style token.
static RE_POSIX_PATH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?:/[A-Za-z0-9._@+\-]+){2,}/?").unwrap());
/// Home-relative path: `~/.sensei/acme`.
static RE_HOME_PATH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"~(?:/[A-Za-z0-9._@+\-]+)+/?").unwrap());
/// Windows absolute path: `C:\work\acme-api`.
static RE_WIN_PATH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Za-z]:\\(?:[A-Za-z0-9._@+\- ]+\\?)+").unwrap());
/// Email address.
static RE_EMAIL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}").unwrap());
/// UUID (any case).
static RE_UUID: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"\b[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}\b")
        .unwrap()
});
/// Session-id shapes: `s-1234`, `sess_abc`, `session-abc123`.
static RE_SESSION: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b(?:s-[0-9]+|sess(?:ion)?[_-][A-Za-z0-9]+)\b").unwrap());
/// SCREAMING_SNAKE token with ≥2 segments and length ≥7 — almost always an
/// env-var / secret / constant, never generalised prose (residual-only).
static RE_SCREAMING: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\b[A-Z][A-Z0-9]*(?:_[A-Z0-9]+)+\b").unwrap());
/// A long alnum blob carrying ≥2 digits — a plausible key/token (residual-only).
static RE_KEYISH: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\b[A-Za-z0-9]{24,}\b").unwrap());

/// Squash to a case-insensitive alnum-only form for order-insensitive presence
/// checks (`Acme-API` → `acmeapi`).
fn squash(s: &str) -> String {
    s.chars().filter(char::is_ascii_alphanumeric).map(|c| c.to_ascii_lowercase()).collect()
}

/// Split a token into lowercase "words" on separators and camelCase boundaries.
/// `AcmeApiClient` / `acme-api-client` / `acme_api_client` → `[acme, api, client]`.
fn token_words(token: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut prev_lower_or_digit = false;
    for ch in token.chars() {
        if ch.is_ascii_alphanumeric() {
            // camelCase boundary: a lower/digit immediately before an uppercase.
            if ch.is_ascii_uppercase() && prev_lower_or_digit && !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            cur.push(ch.to_ascii_lowercase());
            prev_lower_or_digit = ch.is_ascii_lowercase() || ch.is_ascii_digit();
        } else {
            if !cur.is_empty() {
                words.push(std::mem::take(&mut cur));
            }
            prev_lower_or_digit = false;
        }
    }
    if !cur.is_empty() {
        words.push(cur);
    }
    words
}

/// Build the variant-matching regex for a known token, or `None` when the token
/// is not distinctive enough to match safely.
///
/// - Multi-word tokens match their words in sequence with an optional single
///   separator between each (`acme[-_.\s]?api`), case-insensitively — catching
///   snake/kebab/camel/concat forms and the distinctive prefix of a longer token.
/// - Single-word tokens match at a leading boundary (start-of-text or a
///   non-alnum char), which the closure re-emits; camel-embedded single words a
///   boundary can't reach are caught by the residual squash-check instead.
fn token_regex(token: &str) -> Option<(Regex, bool)> {
    let words = token_words(token);
    let total: usize = words.iter().map(String::len).sum();
    if words.is_empty() || total < MIN_DISTINCTIVE_SQUASH {
        return None;
    }
    if words.len() == 1 {
        let body = regex::escape(&words[0]);
        // Capture the leading boundary so the closure can re-emit it.
        let re = Regex::new(&format!(r"(?i)(^|[^A-Za-z0-9])({body})")).ok()?;
        Some((re, true))
    } else {
        let body = words.iter().map(|w| regex::escape(w)).collect::<Vec<_>>().join(r"[-_.\s]?");
        let re = Regex::new(&format!(r"(?i){body}")).ok()?;
        Some((re, false))
    }
}

/// One category of redaction that was applied, with how many occurrences fell.
/// Deliberately carries only the category + placeholder + count — NEVER the raw
/// sensitive value, so an audit trail built from this can't itself re-leak the
/// identifier it recorded removing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Redaction {
    pub kind: RedactionKind,
    pub placeholder: &'static str,
    pub count: usize,
}

/// The category of a stripped identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RedactionKind {
    Path,
    Email,
    Uuid,
    SessionId,
    Project,
    Client,
    Repo,
    Person,
}

/// The known sensitive tokens for one project — the deterministic stripper's
/// context. DB-free: [`crate::db::pg_store::PgStore::project_identifiers`] builds
/// this, but the strip itself takes it by value so it is exhaustively unit-
/// testable without a database.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ProjectIdentifiers {
    /// `sensei.projects.name`.
    pub project_name: Option<String>,
    /// `sensei.projects.client` (the client/org name).
    pub client_name: Option<String>,
    /// Repo names — folder names + names parsed from git remote urls.
    pub repo_names: Vec<String>,
    /// Absolute folder paths (`sensei.folders.abs_path`).
    pub folder_paths: Vec<String>,
    /// Session ids in every form: `activity.sessions.id` (uuid) +
    /// `client_session_id` (the assistant's own id).
    pub session_ids: Vec<String>,
    /// Any person names available (contributors, authors).
    pub person_names: Vec<String>,
}

impl ProjectIdentifiers {
    /// The distinctive name tokens (project / client / repo / person) with their
    /// redaction category — the tokens the residual squash-check backstops.
    fn name_tokens(&self) -> Vec<(&str, RedactionKind)> {
        let mut out = Vec::new();
        if let Some(p) = self.project_name.as_deref() {
            out.push((p, RedactionKind::Project));
        }
        if let Some(c) = self.client_name.as_deref() {
            out.push((c, RedactionKind::Client));
        }
        out.extend(self.repo_names.iter().map(|r| (r.as_str(), RedactionKind::Repo)));
        out.extend(self.person_names.iter().map(|p| (p.as_str(), RedactionKind::Person)));
        out
    }
}

/// The outcome of a deterministic dereference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DereferenceResult {
    /// The stripped text with neutral placeholders.
    pub text: String,
    /// Categories removed (no raw values — see [`Redaction`]).
    pub removed: Vec<Redaction>,
    /// `true` when a likely identifier still survives after stripping. Callers
    /// MUST refuse to publish when this is set — never leak on uncertainty.
    pub residual_risk: bool,
}

/// Accumulates masked spans as opaque sentinels so later passes can't corrupt an
/// earlier placeholder (a short token can't match inside `\0<n>\0`). Sentinels
/// expand to their human placeholder only in [`Redactor::finish`].
struct Redactor {
    text: String,
    placeholders: Vec<&'static str>,
    removed: Vec<Redaction>,
}

impl Redactor {
    fn new(text: &str) -> Self {
        Self { text: text.to_string(), placeholders: Vec::new(), removed: Vec::new() }
    }

    fn record(&mut self, kind: RedactionKind, placeholder: &'static str, count: usize) {
        if count > 0 {
            for _ in 0..count {
                self.placeholders.push(placeholder);
            }
            self.removed.push(Redaction { kind, placeholder, count });
        }
    }

    /// Replace every match of `re` (whole match) with a fresh sentinel.
    fn mask_re(&mut self, re: &Regex, placeholder: &'static str, kind: RedactionKind) {
        let start = self.placeholders.len();
        let counter = Cell::new(start);
        let new = re
            .replace_all(&self.text, |_: &Captures| {
                let i = counter.get();
                counter.set(i + 1);
                format!("\u{0}{i}\u{0}")
            })
            .into_owned();
        let n = counter.get() - start;
        if n > 0 {
            self.text = new;
            self.record(kind, placeholder, n);
        }
    }

    /// Replace a single-word token at a leading boundary, re-emitting the
    /// boundary char the regex captured (group 1) so only the token is removed.
    fn mask_boundary_token(&mut self, re: &Regex, placeholder: &'static str, kind: RedactionKind) {
        let start = self.placeholders.len();
        let counter = Cell::new(start);
        let new = re
            .replace_all(&self.text, |caps: &Captures| {
                let boundary = caps.get(1).map_or("", |m| m.as_str());
                let i = counter.get();
                counter.set(i + 1);
                format!("{boundary}\u{0}{i}\u{0}")
            })
            .into_owned();
        let n = counter.get() - start;
        if n > 0 {
            self.text = new;
            self.record(kind, placeholder, n);
        }
    }

    /// Replace a known literal (case-insensitively), longest such literals first
    /// being the caller's responsibility.
    fn mask_literal(&mut self, literal: &str, placeholder: &'static str, kind: RedactionKind) {
        if literal.trim().is_empty() {
            return;
        }
        if let Ok(re) = Regex::new(&format!(r"(?i){}", regex::escape(literal))) {
            self.mask_re(&re, placeholder, kind);
        }
    }

    /// Expand sentinels back to human placeholders. Sentinels are `\0<n>\0`; the
    /// NUL delimiters make each index unambiguous (`\01\0` is not a substring of
    /// `\010\0`), so replacement order is irrelevant.
    fn finish(mut self) -> (String, Vec<Redaction>) {
        for (i, ph) in self.placeholders.iter().enumerate() {
            self.text = self.text.replace(&format!("\u{0}{i}\u{0}"), ph);
        }
        (self.text, self.removed)
    }
}

/// Deterministically strip every sensitive identifier from `text`, replacing
/// each with a neutral placeholder. Always runs, never calls a model. The
/// returned [`DereferenceResult::residual_risk`] is the fail-closed signal.
///
/// Pass order matters: paths and known session ids are masked before the generic
/// uuid pass (so a uuid session id reads `<session>`, not `<id>`), and name
/// tokens are applied longest-first so `acme-api` falls before `acme`.
pub fn dereference(text: &str, ctx: &ProjectIdentifiers) -> DereferenceResult {
    let mut r = Redactor::new(text);

    // 1. Known folder paths (longest first) — explicit, in case the generic path
    //    regex is too conservative for an unusual path.
    let mut paths: Vec<&String> = ctx.folder_paths.iter().collect();
    paths.sort_by_key(|p| std::cmp::Reverse(p.len()));
    for p in paths {
        r.mask_literal(p, PH_PATH, RedactionKind::Path);
    }
    // 2. Generic absolute paths (Windows before POSIX so a drive prefix wins).
    r.mask_re(&RE_WIN_PATH, PH_PATH, RedactionKind::Path);
    r.mask_re(&RE_HOME_PATH, PH_PATH, RedactionKind::Path);
    r.mask_re(&RE_POSIX_PATH, PH_PATH, RedactionKind::Path);
    // 3. Emails.
    r.mask_re(&RE_EMAIL, PH_EMAIL, RedactionKind::Email);
    // 4. Known session ids (before the uuid pass so uuid-shaped ones read
    //    `<session>`), longest first.
    let mut sessions: Vec<&String> = ctx.session_ids.iter().collect();
    sessions.sort_by_key(|s| std::cmp::Reverse(s.len()));
    for s in sessions {
        r.mask_literal(s, PH_SESSION, RedactionKind::SessionId);
    }
    // 5. Generic uuids, then generic session shapes.
    r.mask_re(&RE_UUID, PH_UUID, RedactionKind::Uuid);
    r.mask_re(&RE_SESSION, PH_SESSION, RedactionKind::SessionId);
    // 6. Known names (project / client / repo / person), longest first.
    let mut names = ctx.name_tokens();
    names.sort_by_key(|(t, _)| std::cmp::Reverse(squash(t).len()));
    for (token, kind) in names {
        let placeholder = match kind {
            RedactionKind::Project => PH_PROJECT,
            RedactionKind::Client => PH_CLIENT,
            RedactionKind::Person => PH_PERSON,
            _ => PH_REPO,
        };
        if let Some((re, is_single)) = token_regex(token) {
            if is_single {
                r.mask_boundary_token(&re, placeholder, kind);
            } else {
                r.mask_re(&re, placeholder, kind);
            }
        }
    }

    let (out, removed) = r.finish();
    let residual_risk = residual_risk(&out, ctx);
    DereferenceResult { text: out, removed, residual_risk }
}

/// Heuristic fail-closed check: does `text` still contain a likely identifier?
///
/// Strong signals (any survivor → risk): an absolute path, a uuid, an email, a
/// session-id shape, a SCREAMING_SNAKE constant, or a long key-ish blob.
/// Known-token signal: after subtracting the placeholders, does the squashed
/// output still contain a squashed distinctive known token — catching anything a
/// boundary-limited replacement let survive (e.g. a camel-embedded short name).
pub fn residual_risk(text: &str, ctx: &ProjectIdentifiers) -> bool {
    if RE_POSIX_PATH.is_match(text)
        || RE_HOME_PATH.is_match(text)
        || RE_WIN_PATH.is_match(text)
        || RE_EMAIL.is_match(text)
        || RE_UUID.is_match(text)
        || RE_SESSION.is_match(text)
        || RE_SCREAMING.is_match(text)
        || RE_KEYISH.is_match(text)
    {
        return true;
    }

    // Subtract placeholders so their letters aren't mistaken for a leak, then
    // squash-scan for any surviving distinctive known token.
    let mut scrubbed = text.to_string();
    for ph in ALL_PLACEHOLDERS {
        scrubbed = scrubbed.replace(ph, " ");
    }
    let haystack = squash(&scrubbed);

    for (token, _) in ctx.name_tokens() {
        let needle = squash(token);
        if needle.len() >= MIN_DISTINCTIVE_SQUASH && haystack.contains(&needle) {
            return true;
        }
    }
    // Known folder paths / session ids: their squashed form surviving is a leak.
    for token in ctx.folder_paths.iter().chain(ctx.session_ids.iter()) {
        let needle = squash(token);
        if needle.len() >= MIN_DISTINCTIVE_SQUASH && haystack.contains(&needle) {
            return true;
        }
    }
    false
}

/// A body of text that has PASSED the deterministic dereference with NO residual
/// risk — the only publish-safe carrier of dereferenced client work.
///
/// The fail-closed contract is made unbypassable by construction: `text` is
/// private and the ONLY constructor ([`Dereferenced::new`]) runs [`dereference`]
/// and returns [`ResidualRisk`] whenever the check is not clean. C6's publish
/// path takes a `Dereferenced`, not a raw `String`, so it is *structurally*
/// impossible to publish text that failed the check — there is no other way to
/// obtain the type.
#[derive(Debug, Clone)]
pub struct Dereferenced {
    text: String,
    removed: Vec<Redaction>,
}

impl Dereferenced {
    /// Run the deterministic strip and gate on it. `Ok` only when the output is
    /// residual-risk-free; `Err(ResidualRisk)` means the caller MUST refuse to
    /// publish.
    pub fn new(text: &str, ctx: &ProjectIdentifiers) -> Result<Self, ResidualRisk> {
        let DereferenceResult { text, removed, residual_risk } = dereference(text, ctx);
        if residual_risk { Err(ResidualRisk { removed }) } else { Ok(Self { text, removed }) }
    }

    /// The stripped, publish-safe text.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Categories removed while producing this (no raw values).
    pub fn removed(&self) -> &[Redaction] {
        &self.removed
    }

    /// Consume into the safe text.
    pub fn into_text(self) -> String {
        self.text
    }
}

/// The dereference could not guarantee safety — a likely identifier survived.
/// Carries only the redaction categories that WERE applied (never a raw value);
/// the surviving identifier is deliberately not exposed so this error can't
/// itself re-leak it. A caller receiving this MUST NOT publish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResidualRisk {
    pub removed: Vec<Redaction>,
}

impl std::fmt::Display for ResidualRisk {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "dereference left residual identifier risk ({} categories stripped) — refusing to publish",
            self.removed.len()
        )
    }
}

impl std::error::Error for ResidualRisk {}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ProjectIdentifiers {
        ProjectIdentifiers {
            project_name: Some("Acme API".into()),
            client_name: Some("Acme Corp".into()),
            repo_names: vec!["acme-api".into(), "billing-svc".into()],
            folder_paths: vec!["/Users/jerry/work/acme-api".into()],
            session_ids: vec!["3f2504e0-4f89-41d3-9a0c-0305e82c3301".into(), "s-9931".into()],
            person_names: vec!["Jane Doe".into()],
        }
    }

    fn contains_ci(haystack: &str, needle: &str) -> bool {
        haystack.to_ascii_lowercase().contains(&needle.to_ascii_lowercase())
    }

    // ── token_words / token_regex unit behaviour ───────────────────────────

    #[test]
    fn token_words_splits_camel_snake_kebab() {
        assert_eq!(token_words("AcmeApiClient"), vec!["acme", "api", "client"]);
        assert_eq!(token_words("acme-api-client"), vec!["acme", "api", "client"]);
        assert_eq!(token_words("acme_api"), vec!["acme", "api"]);
        assert_eq!(token_words("Acme Corp"), vec!["acme", "corp"]);
    }

    // ── every identifier FORM is caught (adversarial, table-driven) ─────────

    #[test]
    fn every_repo_form_is_stripped() {
        let c = ctx();
        // (input, the sensitive substring that must NOT survive)
        let cases = [
            ("use the acme-api service", "acme-api"),       // bare kebab
            ("import from acme_api now", "acme_api"),       // snake
            ("call acmeApi.fetch()", "acmeApi"),            // camelCase
            ("the AcmeApi module", "AcmeApi"),              // PascalCase
            ("see acmeapi readme", "acmeapi"),              // lower concat
            ("in acmeApiClient wrapper", "acmeApi"),        // substring of longer token
            ("path acme-api/src/main.rs here", "acme-api"), // repo inside a relative path
        ];
        for (input, sensitive) in cases {
            let out = dereference(input, &c);
            assert!(
                !contains_ci(&out.text, sensitive),
                "form not stripped: {input:?} still had {sensitive:?} → {:?}",
                out.text
            );
            assert!(
                !out.residual_risk,
                "clean strip should not flag residual for {input:?}: {:?}",
                out.text
            );
        }
    }

    #[test]
    fn absolute_paths_all_forms_stripped() {
        let c = ctx();
        let cases = [
            "edit /Users/jerry/work/acme-api/src/main.rs please", // known + POSIX
            "look under /etc/nginx/sites-enabled/default",        // unknown POSIX
            "config at ~/.sensei/acme-api/config.toml",           // home
            r"open C:\work\acme-api\mod.rs in the ide",           // windows
        ];
        for input in cases {
            let out = dereference(input, &c);
            assert!(
                contains_ci(&out.text, "<path>"),
                "no <path> placeholder for {input:?}: {:?}",
                out.text
            );
            assert!(!out.text.contains("/Users/"), "posix path survived: {:?}", out.text);
            assert!(!out.text.contains(r"C:\"), "windows path survived: {:?}", out.text);
            assert!(
                !out.residual_risk,
                "path strip flagged residual for {input:?}: {:?}",
                out.text
            );
        }
    }

    #[test]
    fn uuid_email_session_are_stripped() {
        let c = ctx();
        let out = dereference(
            "session 3f2504e0-4f89-41d3-9a0c-0305e82c3301 opened by jane@acme.com in s-9931 and sess_ab12",
            &c,
        );
        assert!(!contains_ci(&out.text, "3f2504e0"), "uuid survived: {:?}", out.text);
        assert!(!contains_ci(&out.text, "jane@acme.com"), "email survived: {:?}", out.text);
        assert!(!contains_ci(&out.text, "s-9931"), "known session survived: {:?}", out.text);
        assert!(!contains_ci(&out.text, "sess_ab12"), "generic session survived: {:?}", out.text);
        assert!(!out.residual_risk, "clean strip flagged residual: {:?}", out.text);
    }

    #[test]
    fn client_and_person_and_project_are_stripped() {
        let c = ctx();
        let out =
            dereference("During the Acme Corp engagement, Jane Doe led the Acme API rollout.", &c);
        assert!(!contains_ci(&out.text, "Acme Corp"), "client survived: {:?}", out.text);
        assert!(!contains_ci(&out.text, "Jane Doe"), "person survived: {:?}", out.text);
        assert!(!contains_ci(&out.text, "Acme API"), "project survived: {:?}", out.text);
        assert!(!out.residual_risk, "clean strip flagged residual: {:?}", out.text);
    }

    #[test]
    fn known_token_embedded_mid_sentence_and_mid_path_is_removed() {
        let c = ctx();
        let out = dereference(
            "we refactored the billing-svc handler in /Users/jerry/work/acme-api/billing-svc/mod.rs",
            &c,
        );
        assert!(
            !contains_ci(&out.text, "billing-svc"),
            "mid-sentence/path token survived: {:?}",
            out.text
        );
        assert!(!contains_ci(&out.text, "acme-api"), "path token survived: {:?}", out.text);
        assert!(!out.residual_risk, "clean strip flagged residual: {:?}", out.text);
    }

    // ── fail-closed contract ────────────────────────────────────────────────

    #[test]
    fn residual_risk_flags_surviving_absolute_path() {
        // A raw path in the text with NO ctx that describes it: the generic
        // detector must still flag it.
        assert!(residual_risk("see /Users/x/y/secret.rs", &ProjectIdentifiers::default()));
    }

    #[test]
    fn residual_risk_flags_surviving_known_token_camel_embedded() {
        // A 3-char client camel-embedded past a boundary the replacement can't
        // reach: the squash backstop must catch it → publish refused.
        let c = ProjectIdentifiers { client_name: Some("IBM".into()), ..Default::default() };
        assert!(residual_risk("deployed to myIBMservice cluster", &c));
    }

    #[test]
    fn residual_risk_flags_screaming_snake_secret() {
        assert!(residual_risk(
            "set ACME_PROD_DB_PASSWORD before boot",
            &ProjectIdentifiers::default()
        ));
    }

    #[test]
    fn residual_risk_false_for_clean_generalised_prose() {
        let c = ctx();
        assert!(!residual_risk(
            "Prefer a dedicated migration tool over hand-rolled SQL when the schema churns.",
            &c
        ));
    }

    #[test]
    fn dereferenced_new_refuses_when_identifier_survives() {
        // SCREAMING_SNAKE secret isn't a known token or a path — dereference
        // won't strip it, so the residual check must make `Dereferenced::new`
        // fail closed rather than hand back publishable text.
        let err = Dereferenced::new("export ACME_SECRET_TOKEN=1", &ProjectIdentifiers::default());
        assert!(err.is_err(), "obvious surviving identifier must fail closed");
    }

    #[test]
    fn dereferenced_new_ok_and_carries_safe_text() {
        let c = ctx();
        let d = Dereferenced::new("Jane Doe worked in /Users/jerry/work/acme-api", &c).unwrap();
        assert!(!contains_ci(d.text(), "jane doe"));
        assert!(!contains_ci(d.text(), "/Users/"));
        assert!(!d.removed().is_empty(), "should record the categories it stripped");
    }

    #[test]
    fn dereferenced_only_constructs_through_the_check() {
        // Compile-time guarantee (documented): `Dereferenced { text }` cannot be
        // built outside this module because the field is private. This test
        // asserts the runtime half — the constructor always runs the gate.
        let bad = Dereferenced::new("token s-1 at /a/b/c", &ProjectIdentifiers::default());
        // /a/b/c is a POSIX path (2 segments) + s-1 is a session id → stripped,
        // then residual clean → Ok, but proving the gate ran: removed is set.
        let d = bad.unwrap();
        assert!(d.removed().iter().any(|r| r.kind == RedactionKind::Path));
    }

    // ── idempotence ─────────────────────────────────────────────────────────

    #[test]
    fn dereference_is_idempotent() {
        let c = ctx();
        let input =
            "Jane Doe on acme-api at /Users/jerry/work/acme-api, session s-9931, jane@acme.com";
        let once = dereference(input, &c);
        let twice = dereference(&once.text, &c);
        assert_eq!(once.text, twice.text, "second pass changed the text");
        assert!(
            twice.removed.is_empty(),
            "second pass should have nothing left to remove: {:?}",
            twice.removed
        );
        assert!(!twice.residual_risk);
    }

    #[test]
    fn placeholders_are_not_re_redacted() {
        // The placeholders themselves must survive a re-run untouched.
        let c = ctx();
        let masked = dereference("acme-api and Acme Corp and /Users/jerry/work/acme-api", &c).text;
        assert!(
            masked.contains(PH_REPO) || masked.contains(PH_PROJECT) || masked.contains(PH_CLIENT)
        );
        let again = dereference(&masked, &c);
        assert_eq!(again.text, masked);
    }

    // ── redaction bookkeeping ───────────────────────────────────────────────

    #[test]
    fn removed_records_categories_without_raw_values() {
        // Use `billing-svc` (a repo distinct from the project name) so the Repo
        // category is unambiguous — `acme-api` would otherwise be claimed by the
        // equal-squash Project token first (which is itself correct: the token
        // IS stripped, only the label differs).
        let c = ctx();
        let out = dereference("Jane Doe at jane@acme.com in billing-svc", &c);
        let kinds: Vec<RedactionKind> = out.removed.iter().map(|r| r.kind).collect();
        assert!(kinds.contains(&RedactionKind::Person));
        assert!(kinds.contains(&RedactionKind::Email));
        assert!(kinds.contains(&RedactionKind::Repo));
        // The struct only carries kind/placeholder/count — no field can hold the
        // raw value, enforced by the type.
        for r in &out.removed {
            assert!(r.count >= 1);
        }
    }
}

#[cfg(test)]
mod residual_risk_false_positives {
    //! CHARACTERISATION TESTS — these assert the CURRENT, WRONG behaviour.
    //!
    //! They are green because the defect is present. When the token list is
    //! fixed, EVERY assertion here must be INVERTED to `!residual_risk(...)`.
    //! A green suite here is not a healthy one; it is a recorded defect.
    use super::*;

    /// Demonstrates a SYSTEMATIC false positive, observed live on 2026-08-29.
    ///
    /// `squash` strips every non-alphanumeric, so the haystack has NO word
    /// boundaries, and the scan is a raw `contains` with
    /// `MIN_DISTINCTIVE_SQUASH = 3`. `repo_names` is populated from EVERY folder
    /// name in a project — for `sensei` that is ~300 entries including `api`,
    /// `app`, `you`, `org`, `bin`, `src`, `lib`, `code`, `state`, `token`.
    ///
    /// A three-letter folder name therefore matches inside ordinary English, and
    /// across word boundaries once the spaces are gone. These are not contrived:
    /// each token below is a real folder in this repository.
    #[test]
    fn a_three_letter_folder_name_matches_inside_ordinary_english() {
        let ctx = ProjectIdentifiers { repo_names: vec!["app".into()], ..Default::default() };
        // "happens" contains "app". Nothing here identifies anything.
        assert!(
            residual_risk("this happens whenever the cache is cold", &ctx),
            "a folder named `app` must not make the word `happens` a leak"
        );
    }

    #[test]
    fn tokens_match_across_word_boundaries_once_squashed() {
        let ctx = ProjectIdentifiers { repo_names: vec!["api".into()], ..Default::default() };
        // "a pipeline" squashes to "apipeline", which contains "api".
        assert!(
            residual_risk("prefer a pipeline over a barrier", &ctx),
            "squashing removes the boundary, so `a pipeline` reads as `api`"
        );
    }

    #[test]
    fn the_live_case_a_clean_generalised_principle_is_held() {
        // Verbatim from the memory held on 2026-08-29 by the real publish path.
        // `stores` and `providers` are both folder names in this project.
        let ctx = ProjectIdentifiers {
            repo_names: vec!["stores".into(), "providers".into()],
            ..Default::default()
        };
        assert!(
            residual_risk(
                "Integrating specific environment credential stores simplifies usability, \
                 but requires adjustments to component properties and consideration of \
                 supporting multiple provider types.",
                &ctx
            ),
            "this is the text the gate actually held — no path, no email, no id, no name"
        );
    }
}
