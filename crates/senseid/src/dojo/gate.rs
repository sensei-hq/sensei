//! Hook-gate decision core — the PURE, testable helpers behind the daemon↔agent
//! control leg (relay-engine feature B).
//!
//! A Claude Code `PreToolUse` hook asks the daemon whether a tool call may
//! proceed; the daemon raises a relay gate to the phone and blocks for the
//! human's answer. This module holds only the pure logic: which tools are gated
//! (from an env allow-list), whether a given tool is in that set, and how an
//! answered reply maps to allow/deny. The impure part (raise + await + publish)
//! lives in the `/hook/gate` handler.
//!
//! **Fail-open is the whole contract.** Gating is OFF unless a tool is named in
//! `SENSEI_RELAY_GATE_TOOLS`, and only an explicit human `deny` reply yields
//! `"deny"`. Every other outcome — no allow-list, tool not listed, no reply,
//! timeout, malformed reply — resolves to `"allow"`. A gate must NEVER block a
//! tool call because of infrastructure; only a person can.

/// Parse the comma-separated gated-tool allow-list from the raw
/// `SENSEI_RELAY_GATE_TOOLS` env value. Trims each entry and drops empties.
/// Case-sensitive (tool names match Claude's exactly, e.g. `Bash`, `Write`).
/// `None` or an all-empty value → an empty list (⇒ gating fully off).
///
/// Pure: takes the raw string as a param so it needs no environment to test.
pub fn gated_tools_from_env(var: Option<&str>) -> Vec<String> {
    var.map(|s| {
        s.split(',')
            .map(str::trim)
            .filter(|t| !t.is_empty())
            .map(str::to_string)
            .collect()
    })
    .unwrap_or_default()
}

/// Whether a tool call should be gated: true iff its `tool_name` is in the
/// (case-sensitive) allow-list. An empty list ⇒ always false ⇒ never gate.
pub fn should_gate(tool_name: &str, gated: &[String]) -> bool {
    gated.iter().any(|t| t == tool_name)
}

/// Map an answered gate reply to the Claude `permissionDecision` string —
/// `"allow"` or `"deny"`. **Fail-open:** only an explicit human decline yields
/// `"deny"` — that is, `reply.verdict == "deny"` (case-insensitive) OR
/// `reply.approve == false`. Everything else — `None` (timeout / no reply), an
/// approve verdict, a missing or malformed shape — yields `"allow"`. A gate can
/// only ever block on a deliberate human deny.
pub fn decision_from_reply(reply: Option<&serde_json::Value>) -> &'static str {
    let Some(reply) = reply else {
        return "allow"; // no reply (timeout) → allow
    };

    let verdict_deny = reply
        .get("verdict")
        .and_then(|v| v.as_str())
        .is_some_and(|s| s.eq_ignore_ascii_case("deny"));

    // `approve: false` is an explicit decline; `approve: true`/absent is not.
    let approve_false = reply.get("approve").and_then(|v| v.as_bool()) == Some(false);

    if verdict_deny || approve_false {
        "deny"
    } else {
        "allow"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn gated_tools_parses_and_trims() {
        assert_eq!(
            gated_tools_from_env(Some("Bash,Write")),
            vec!["Bash".to_string(), "Write".to_string()]
        );
        // whitespace around entries is trimmed
        assert_eq!(
            gated_tools_from_env(Some(" Bash , Write ")),
            vec!["Bash".to_string(), "Write".to_string()]
        );
    }

    #[test]
    fn gated_tools_empty_when_unset_or_blank() {
        assert!(gated_tools_from_env(None).is_empty());
        assert!(gated_tools_from_env(Some("")).is_empty());
        assert!(gated_tools_from_env(Some("  ")).is_empty());
        // stray commas produce no empty entries
        assert!(gated_tools_from_env(Some(",,")).is_empty());
        assert_eq!(gated_tools_from_env(Some("Bash,,")), vec!["Bash".to_string()]);
    }

    #[test]
    fn should_gate_only_listed_tools_case_sensitive() {
        let gated = gated_tools_from_env(Some("Bash,Write"));
        assert!(should_gate("Bash", &gated));
        assert!(should_gate("Write", &gated));
        assert!(!should_gate("Read", &gated), "unlisted tool is not gated");
        assert!(!should_gate("bash", &gated), "case-sensitive: bash != Bash");
    }

    #[test]
    fn should_gate_false_when_list_empty() {
        // The default: no allow-list ⇒ nothing is ever gated.
        assert!(!should_gate("Bash", &[]));
        assert!(!should_gate("Write", &[]));
    }

    #[test]
    fn decision_deny_only_on_explicit_deny() {
        // explicit deny verdict → deny
        assert_eq!(decision_from_reply(Some(&json!({"verdict": "deny"}))), "deny");
        // case-insensitive verdict match
        assert_eq!(decision_from_reply(Some(&json!({"verdict": "DENY"}))), "deny");
        // approve == false → deny
        assert_eq!(decision_from_reply(Some(&json!({"approve": false}))), "deny");
    }

    #[test]
    fn decision_allow_on_approve_or_anything_else() {
        // explicit approve verdict → allow
        assert_eq!(decision_from_reply(Some(&json!({"verdict": "approve"}))), "allow");
        // approve == true → allow
        assert_eq!(decision_from_reply(Some(&json!({"approve": true}))), "allow");
        // free-text / unrelated shapes → allow (fail-open)
        assert_eq!(decision_from_reply(Some(&json!({"note": "looks fine"}))), "allow");
        assert_eq!(decision_from_reply(Some(&json!("whatever"))), "allow");
        assert_eq!(decision_from_reply(Some(&json!({}))), "allow");
    }

    #[test]
    fn decision_allow_on_none_timeout() {
        // No reply (the await_reply timeout path) is fail-open → allow.
        assert_eq!(decision_from_reply(None), "allow");
    }
}
