//! Doc-drift extraction — pure heuristics that turn a doc node's content
//! into candidate identifier mentions.
//!
//! Kept as a stand-alone module (no DB access) so the extractor is
//! unit-testable without a Postgres fixture. The daemon-side driver in
//! `pg_store::scan_project_doc_drift` calls `extract_identifier_mentions`
//! per doc and joins the results against `sensei.nodes` to decide whether
//! each mention is a real reference or drift.
//!
//! Heuristic scope — intentionally narrow so we surface *high-signal*
//! drift and don't drown the Traceability page in false positives:
//! - Only backtick-wrapped tokens count. A doc that types `foo` clearly
//!   meant to reference something; loose "foo" in prose is ambiguous and
//!   is skipped.
//! - Tokens must look like an identifier (letters, digits, underscore,
//!   starting with a non-digit) and be at least 3 chars long — a plain
//!   `x` is noise.
//! - A small stopword list filters English words and common tokens that
//!   backtick pairs sometimes wrap (`if`, `for`, `null`, …).
//! - Extra shape filter: a single-word lowercase or single-word capitalised
//!   token (`sensei`, `Handler`) is almost always a common noun or project
//!   name, not a code symbol. We require either an underscore (snake_case),
//!   an internal uppercase-after-lowercase transition (camelCase /
//!   PascalCase-with-more-than-one-word), or a trailing `()` call marker
//!   before treating the token as a real code reference.

use std::collections::HashSet;

/// Extract candidate identifier mentions from a doc node's content.
///
/// Deduplicates so a doc that mentions `foo` twenty times contributes
/// one row to the drift scan. Order is insertion (first-mention) order
/// so tests can assert against a stable sequence.
pub fn extract_identifier_mentions(content: &str) -> Vec<String> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();

    // Sliding-window scan over backtick pairs. Multi-line backticks (```)
    // demarcate fenced code blocks — skip those entirely, otherwise a
    // fenced Rust snippet in the doc would flood the mention list.
    let mut in_fence = false;
    for line in content.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        scan_backtick_spans(line, |token| {
            if !is_candidate_identifier(token) {
                return;
            }
            // Strip the trailing `()` on function-call mentions so the
            // stored identifier matches the graph node name (which has no
            // parens).
            let stored = token.strip_suffix("()").unwrap_or(token).to_string();
            if seen.insert(stored.clone()) {
                out.push(stored);
            }
        });
    }
    out
}

/// Iterate every backtick-wrapped span in a single line and hand each raw
/// span to `visit`. Unbalanced trailing backticks (` foo `` ) are ignored.
fn scan_backtick_spans(line: &str, mut visit: impl FnMut(&str)) {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'`' {
            i += 1;
            continue;
        }
        let start = i + 1;
        let mut j = start;
        while j < bytes.len() && bytes[j] != b'`' {
            j += 1;
        }
        if j >= bytes.len() {
            break; // unmatched opening backtick — bail on this line
        }
        if let Ok(span) = std::str::from_utf8(&bytes[start..j]) {
            visit(span.trim());
        }
        i = j + 1;
    }
}

/// True if `token` looks like a code identifier we might find in the graph
/// and *not* a stopword. Rejects empty strings, too-short tokens, and
/// tokens starting with a digit; accepts leading letters / underscores
/// followed by letters, digits, or underscores.
///
/// A trailing `()` is stripped before checking so `` `foo()` `` still
/// resolves to `foo`. Bare single-word tokens (`sensei`, `Handler`,
/// `parser`) are rejected unless the original `raw` carried the `()`
/// call marker — those are almost always common nouns / project names in
/// prose, not code identifiers. `snake_case`, `camelCase`, and
/// `PascalCase` compound tokens still pass.
fn is_candidate_identifier(raw: &str) -> bool {
    let had_call_marker = raw.ends_with("()");
    let token = raw.strip_suffix("()").unwrap_or(raw);
    if token.len() < 3 {
        return false;
    }
    if is_stopword(token) {
        return false;
    }
    let bytes = token.as_bytes();
    let first = bytes[0];
    if !(first.is_ascii_alphabetic() || first == b'_') {
        return false;
    }
    if !bytes.iter().all(|c| c.is_ascii_alphanumeric() || *c == b'_') {
        return false;
    }
    // Compound-shape requirement — see module docs.
    let has_underscore = bytes.contains(&b'_');
    let has_case_transition =
        bytes.windows(2).any(|w| w[0].is_ascii_lowercase() && w[1].is_ascii_uppercase());
    if !(has_underscore || has_case_transition || had_call_marker) {
        return false;
    }
    // SCREAMING_SNAKE_CASE is the env-var / config-key convention
    // (`CLOUDFLARE_API_TOKEN`, `CF_PAGES`, `BASE_PATH`) — a doc mentioning one
    // references configuration, not a project code symbol, so its absence from
    // the graph is not drift. (A removed SCREAMING const is the rare recall
    // tradeoff; such consts that still exist resolve via the known-symbol set.)
    let is_screaming_snake = has_underscore && !token.bytes().any(|b| b.is_ascii_lowercase());
    if is_screaming_snake {
        return false;
    }
    true
}

/// Reverse of the drift-writer format — given a detail string like
/// ``Mentions `foo` which is not in the code.``, return `Some("foo")` if
/// the mention name can be recovered, `None` otherwise.
///
/// Kept alongside the extractor so a format change to the detail template
/// is a single-file edit.
pub fn extract_mention_from_detail(detail: &str) -> Option<String> {
    let start = detail.find('`')?;
    let rest = &detail[start + 1..];
    let end = rest.find('`')?;
    let name = &rest[..end];
    if name.is_empty() { None } else { Some(name.to_string()) }
}

/// Decide whether a doc mention is *broken drift*: it names something that WAS a
/// real code symbol (present in the ever-seen `symbol_names` registry) but no
/// longer RESOLVES — it isn't a current symbol or a DB schema identifier
/// (`known`). A mention that was never a symbol (a Rust enum variant, a
/// serde-renamed camelCase field, a string-dispatched MCP tool name) is prose or
/// config, not a broken code reference, so it is NOT drift — the previous
/// "not in the graph ⇒ drift" rule over-fired ~408 such false positives.
///
/// Used for BOTH the insert gate (flag a new mention only when this is true) and
/// resolution (clear an open row once this is false — the doc got fixed, the code
/// came back, OR the mention was never a real symbol).
pub fn is_broken_drift(
    mention: &str,
    known: &HashSet<String>,
    ever_symbols: &HashSet<String>,
) -> bool {
    !known.contains(mention) && ever_symbols.contains(mention)
}

fn is_stopword(token: &str) -> bool {
    matches!(
        token,
        // Control flow / booleans / keywords that appear backticked in prose.
        "if" | "else" | "for" | "while" | "loop" | "match" | "return"
        | "true" | "false" | "null" | "none" | "some" | "let" | "const"
        | "fn" | "async" | "await" | "yield" | "break" | "continue"
        // Common documentation nouns that also make lousy code references.
        | "the" | "and" | "not" | "any" | "all" | "one" | "two" | "here"
        | "this" | "that" | "yes" | "TODO" | "NOTE" | "FIXME"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_snake_and_pascal_multiword_identifiers() {
        // `process_event` has underscores → snake_case.
        // `EventHandler` has an internal lowercase→uppercase transition → PascalCase.
        // Bare `Handler` (single word, no compound shape) is rejected.
        let content = "The `process_event` helper delegates to `EventHandler`.";
        assert_eq!(
            extract_identifier_mentions(content),
            vec!["process_event".to_string(), "EventHandler".to_string()],
        );
    }

    #[test]
    fn rejects_screaming_snake_env_var_convention() {
        // SCREAMING_SNAKE tokens are env vars / config keys, not code symbols —
        // a doc mentioning `CLOUDFLARE_API_TOKEN` or `CF_PAGES` isn't drift.
        // A snake_case symbol alongside them still passes.
        let content = "Set `CLOUDFLARE_API_TOKEN` and `CF_PAGES`, then call `process_event`.";
        assert_eq!(extract_identifier_mentions(content), vec!["process_event".to_string()],);
    }

    #[test]
    fn rejects_single_word_bare_identifiers() {
        // Common nouns and project names in prose — `sensei`, `Handler`,
        // `parser` — always look like identifiers but are almost never real
        // code references. The compound-shape rule filters them.
        let content = "The `sensei` project ships a `Handler` and a `parser`.";
        assert!(extract_identifier_mentions(content).is_empty());
    }

    #[test]
    fn strips_trailing_parens_from_function_call_mentions() {
        let content = "Call `foo()` and `bar()`.";
        assert_eq!(
            extract_identifier_mentions(content),
            vec!["foo".to_string(), "bar".to_string()],
        );
    }

    #[test]
    fn deduplicates_repeated_mentions() {
        // Use a compound-shape identifier so the shape filter doesn't
        // reject it — the assertion is about dedup, not about acceptance.
        let content = "First `foo_bar`, then `foo_bar`, then `foo_bar`.";
        assert_eq!(extract_identifier_mentions(content), vec!["foo_bar".to_string()]);
    }

    #[test]
    fn skips_stopwords_short_tokens_and_digits() {
        let content =
            "Use `if` (skip) or `x` (short) or `9lives` (bad start), but keep `parse_line`.";
        assert_eq!(extract_identifier_mentions(content), vec!["parse_line".to_string()],);
    }

    #[test]
    fn accepts_single_word_call_marker_identifiers() {
        // `parse()` — bare word + call marker — is a clear code reference
        // even without a compound shape.
        let content = "Then `parse()` is invoked.";
        assert_eq!(extract_identifier_mentions(content), vec!["parse".to_string()],);
    }

    #[test]
    fn skips_fenced_code_blocks() {
        // A fenced Rust example must NOT contribute mentions — otherwise
        // every example function shows up as "referenced but missing".
        let content = "\
Intro talks about `real_reference`.

```rust
fn ignored_in_code_block() {}
let x = another_should_be_ignored();
```

Back to prose: `still_counted`.";
        assert_eq!(
            extract_identifier_mentions(content),
            vec!["real_reference".to_string(), "still_counted".to_string()],
        );
    }

    #[test]
    fn ignores_unmatched_backticks() {
        let content = "Half-open ` never closes on this line.\nBut `closed_pair` is fine.";
        assert_eq!(extract_identifier_mentions(content), vec!["closed_pair".to_string()],);
    }

    #[test]
    fn empty_string_yields_no_mentions() {
        assert!(extract_identifier_mentions("").is_empty());
    }

    #[test]
    fn rejects_tokens_with_disallowed_characters() {
        // A backticked span with punctuation isn't a valid identifier —
        // e.g. `Foo::bar` or `foo.bar`.
        let content = "See `Foo::bar` and `foo.bar` and `foo bar` — none should count.";
        assert!(extract_identifier_mentions(content).is_empty());
    }

    #[test]
    fn is_broken_drift_flags_only_previously_indexed_now_missing() {
        // known = things that resolve NOW (a current symbol + a DB schema name).
        let known: HashSet<String> =
            ["current_fn".to_string(), "project_id".to_string()].into_iter().collect();
        // ever = every name that was ever a real symbol (history).
        let ever: HashSet<String> =
            ["current_fn".to_string(), "removed_fn".to_string()].into_iter().collect();

        // Was a symbol (in history), now gone (doesn't resolve) → real drift.
        assert!(is_broken_drift("removed_fn", &known, &ever));
        // A current symbol resolves → not drift.
        assert!(!is_broken_drift("current_fn", &known, &ever));
        // Never a symbol (enum variant / camelCase field / tool string): absent
        // from history → NOT drift. This is the false-positive class we killed.
        assert!(!is_broken_drift("battle_tested", &known, &ever));
        // Resolves via a DB schema identifier (in `known`, never a code symbol) →
        // not drift.
        assert!(!is_broken_drift("project_id", &known, &ever));
    }

    #[test]
    fn extract_mention_recovers_name_from_detail_string() {
        let detail = "Mentions `parse_line` which is not in the code.";
        assert_eq!(extract_mention_from_detail(detail), Some("parse_line".to_string()));
    }

    #[test]
    fn extract_mention_returns_none_when_no_backtick_pair() {
        assert!(extract_mention_from_detail("no code refs here").is_none());
        assert!(extract_mention_from_detail("").is_none());
        assert!(extract_mention_from_detail("only one ` backtick").is_none());
    }

    #[test]
    fn extract_mention_returns_none_for_empty_backtick_pair() {
        assert!(extract_mention_from_detail("empty `` span").is_none());
    }
}
