//! A tracker board's delivery facts as the daemon sends them to the dōjō — the pure
//! half (spec docs/spec/daemon/tracker-delivery-metrics.md D3; #352).
//!
//! Everything here is a function of its inputs: the board's pseudonymous key, which
//! dōjō tenant (if any) a board may go to for one persona, the request body, its
//! canonical hash, its size check, and whether a part is due. The orchestration
//! (reading, locking, sending, marking) is `tasks::dojo_delivery`.

use chrono::{DateTime, Duration, Utc};
use serde_json::{Value, json};

use crate::dojo_client::user_plane::MappedRepo;

/// At most this many tickets in one board (D3).
pub const MAX_TICKETS: usize = 5000;

/// At most this many bytes in one request body on the daemon; the dōjō refuses
/// over 4 MB, and the margin keeps a near-limit board from failing every tick (D3).
pub const MAX_BYTES: usize = 3_500_000;

/// An acknowledged mark is re-sent after this long, so a copy withdrawn or
/// replaced in the dōjō is restored by a contributor whose data did not change.
pub const MARK_TTL_HOURS: i64 = 6;

/// A refused hash is not re-sent within this long; a changed one is.
pub const REFUSAL_BACKOFF_HOURS: i64 = 1;

/// `hex(sha256(kind + "\n" + base_url + "\n" + board_ref))`. `base_url` and
/// `board_ref` are canonical on every install (`normalize_base_url`,
/// `canonical_board_id`), so one board has one key everywhere — and no URL travels.
pub fn board_key(kind: &str, base_url: &str, board_ref: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(format!("{kind}\n{base_url}\n{board_ref}").as_bytes());
    hex::encode(h.finalize())
}

/// Why a board may not go to the dōjō for a persona.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ineligible {
    /// None of the workspace's repositories is in the persona's plan.
    NotShared,
    /// They are, but in more than one dōjō tenant (B8).
    Ambiguous,
}

impl Ineligible {
    /// The code a skipped board is recorded under.
    pub fn code(self) -> &'static str {
        match self {
            Ineligible::NotShared => "tracker_not_shared",
            Ineligible::Ambiguous => "tracker_workspace_ambiguous",
        }
    }
}

/// Where a board may go for a persona: one tenant, and the repositories naming it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Eligible {
    pub tenant_id: String,
    /// Sorted, without repeats.
    pub repo_keys: Vec<String>,
}

/// The workspace's repositories (`workspace_repo_keys`) that the persona's plan
/// allows, and their one plan tenant (`MappedRepo.tenant_id`, the value the dōjō's
/// own consent check compares).
pub fn eligibility(
    allowed: &[MappedRepo],
    workspace_repo_keys: &[String],
) -> Result<Eligible, Ineligible> {
    let mine: Vec<&MappedRepo> =
        allowed.iter().filter(|m| workspace_repo_keys.contains(&m.repo_key)).collect();
    let Some(first) = mine.first() else {
        return Err(Ineligible::NotShared);
    };
    if mine.iter().any(|m| m.tenant_id != first.tenant_id) {
        return Err(Ineligible::Ambiguous);
    }
    let mut repo_keys: Vec<String> = mine.iter().map(|m| m.repo_key.clone()).collect();
    repo_keys.sort();
    repo_keys.dedup();
    Ok(Eligible { tenant_id: first.tenant_id.clone(), repo_keys })
}

/// One sprint of a board, as it travels.
#[derive(Debug, Clone, PartialEq)]
pub struct FactSprint {
    pub external_id: String,
    pub seq: i64,
    pub name: String,
    pub state: String,
    pub start_at: Option<DateTime<Utc>>,
    pub end_at: Option<DateTime<Utc>>,
    pub complete_at: Option<DateTime<Utc>>,
    pub removed: bool,
    /// The sprint's `SprintSdlc`; `None` for a sprint with no replay (removed).
    pub sdlc: Option<Value>,
}

/// One non-removed ticket of a board, as it travels: the title is its only free text (B9).
#[derive(Debug, Clone, PartialEq)]
pub struct FactTicket {
    pub key: String,
    pub title: Option<String>,
    pub status_category: Option<String>,
    pub points: Option<f64>,
    /// External ids of every non-removed sprint that lists it.
    pub sprints: Vec<String>,
    pub issue_created_at: Option<DateTime<Utc>>,
}

/// A board's facts: the part the dōjō keeps one copy of.
#[derive(Debug, Clone, PartialEq)]
pub struct BoardFacts {
    pub history_at: Option<DateTime<Utc>>,
    pub pending_tickets: i64,
    pub predictability: Option<f64>,
    pub predictability_reason: Option<String>,
    pub sprints: Vec<FactSprint>,
    pub tickets: Vec<FactTicket>,
}

/// The facts as JSON, in a fixed order: sprints by `(seq, external_id)`, tickets
/// by key, each ticket's sprints sorted. Read order never changes the bytes.
pub fn facts_json(f: &BoardFacts) -> Value {
    let mut sprints: Vec<&FactSprint> = f.sprints.iter().collect();
    sprints.sort_by(|a, b| (a.seq, &a.external_id).cmp(&(b.seq, &b.external_id)));
    let mut tickets: Vec<&FactTicket> = f.tickets.iter().collect();
    tickets.sort_by(|a, b| a.key.cmp(&b.key));
    json!({
        "history_at": f.history_at,
        "pending_tickets": f.pending_tickets,
        "predictability": f.predictability,
        "predictability_reason": f.predictability_reason,
        "sprints": sprints.iter().map(|s| json!({
            "external_id": s.external_id,
            "seq": s.seq,
            "name": s.name,
            "state": s.state,
            "start_at": s.start_at,
            "end_at": s.end_at,
            "complete_at": s.complete_at,
            "removed": s.removed,
            "sdlc": s.sdlc,
        })).collect::<Vec<_>>(),
        "tickets": tickets.iter().map(|t| {
            let mut ids = t.sprints.clone();
            ids.sort();
            json!({
                "key": t.key,
                "title": t.title,
                "status_category": t.status_category,
                "points": t.points,
                "sprints": ids,
                "issue_created_at": t.issue_created_at,
            })
        }).collect::<Vec<_>>(),
    })
}

/// sha256 of what the dōjō keeps of the facts part — the board's kind and name and
/// its facts — in canonical JSON, without `history_at`, which moves on every history
/// run even one that read nothing new (D3). A renamed board hashes differently.
pub fn facts_hash(kind: &str, board_name: &str, facts: &Value) -> String {
    use sha2::{Digest, Sha256};
    let mut facts = facts.clone();
    if let Some(o) = facts.as_object_mut() {
        o.remove("history_at");
    }
    let kept = json!({ "kind": kind, "board_name": board_name, "facts": facts });
    hex::encode(Sha256::digest(crate::canonical_json::canonical_json(&kept).as_bytes()))
}

/// The request body (D3). The effort part is `null` until F3 sends one.
pub fn request_body(
    to: &Eligible,
    board_key: &str,
    kind: &str,
    board_name: &str,
    facts: &Value,
) -> Value {
    json!({
        "v": 1,
        "tenant_id": to.tenant_id,
        "repo_keys": to.repo_keys,
        "board_key": board_key,
        "kind": kind,
        "board_name": board_name,
        "facts": facts,
        "effort": Value::Null,
    })
}

/// A board too big to send, by ticket count or bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TooLarge {
    pub tickets: usize,
    pub bytes: usize,
}

/// The serialized body, or why it is too large to send.
pub fn sized_body(body: &Value, tickets: usize) -> Result<Vec<u8>, TooLarge> {
    let bytes = body.to_string().into_bytes();
    if tickets > MAX_TICKETS || bytes.len() > MAX_BYTES {
        return Err(TooLarge { tickets, bytes: bytes.len() });
    }
    Ok(bytes)
}

/// What this machine last recorded of a part (`sensei.tracker_push_marks`).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MarkState {
    pub hash: Option<String>,
    pub pushed_at: Option<DateTime<Utc>>,
    pub failed_hash: Option<String>,
    pub failed_at: Option<DateTime<Utc>>,
}

/// Whether a part with this hash is due: not when the dōjō refused this very hash
/// within the hour; not when it acknowledged it within six hours; otherwise yes.
pub fn is_due(mark: Option<&MarkState>, hash: &str, now: DateTime<Utc>) -> bool {
    let Some(m) = mark else {
        return true;
    };
    let within = |at: Option<DateTime<Utc>>, hours: i64| {
        at.is_some_and(|t| now - t < Duration::hours(hours))
    };
    if m.failed_hash.as_deref() == Some(hash) && within(m.failed_at, REFUSAL_BACKOFF_HOURS) {
        return false;
    }
    !(m.hash.as_deref() == Some(hash) && within(m.pushed_at, MARK_TTL_HOURS))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn at(h: u32) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 11, 4, h, 0, 0).unwrap()
    }

    fn mapped(repo_key: &str, tenant_id: &str) -> MappedRepo {
        MappedRepo {
            workspace_error: None,
            repo_key: repo_key.to_string(),
            tenant: "organization/ztest".to_string(),
            tenant_id: tenant_id.to_string(),
            repo_id: "r".to_string(),
            disabled_metrics: vec![],
            graph_branches: vec![],
        }
    }

    fn sprint(id: &str, seq: i64) -> FactSprint {
        FactSprint {
            external_id: id.into(),
            seq,
            name: format!("Sprint {id}"),
            state: "closed".into(),
            start_at: Some(at(1)),
            end_at: Some(at(2)),
            complete_at: None,
            removed: false,
            sdlc: Some(json!({"velocity": 21, "reasons": {}})),
        }
    }

    fn ticket(key: &str, sprints: &[&str]) -> FactTicket {
        FactTicket {
            key: key.into(),
            title: Some(format!("{key} title")),
            status_category: Some("done".into()),
            points: Some(3.0),
            sprints: sprints.iter().map(|s| s.to_string()).collect(),
            issue_created_at: None,
        }
    }

    fn facts() -> BoardFacts {
        BoardFacts {
            history_at: Some(at(9)),
            pending_tickets: 0,
            predictability: Some(12.5),
            predictability_reason: None,
            sprints: vec![sprint("102", 102), sprint("101", 101)],
            tickets: vec![ticket("AUTH-10", &["102", "101"]), ticket("AUTH-2", &["102"])],
        }
    }

    #[test]
    fn board_key_is_the_sha256_of_the_canonical_connection() {
        // sha256("jira\nhttps://acme.atlassian.net\n42"), computed independently (Python hashlib).
        assert_eq!(
            board_key("jira", "https://acme.atlassian.net", "42"),
            "6e89c376a712d534df9d055c8ff50955faa1c8f2102970e13540791b942a5d45"
        );
        let k = board_key("jira", "https://acme.atlassian.net", "42");
        assert_eq!(k.len(), 64);
        assert!(k.chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_ne!(k, board_key("jira", "https://acme.atlassian.net", "43"));
        // The separator matters: "a\nbc" and "ab\nc" are different boards.
        assert_ne!(board_key("jira", "https://a", "bc"), board_key("jira", "https://ab", "c"));
    }

    #[test]
    fn eligibility_is_the_workspace_repositories_the_plan_allows_in_one_tenant() {
        let ws = vec!["github.com/acme/api".to_string(), "github.com/acme/web".to_string()];
        assert_eq!(eligibility(&[], &ws), Err(Ineligible::NotShared));
        assert_eq!(
            eligibility(&[mapped("github.com/other/x", "t1")], &ws),
            Err(Ineligible::NotShared)
        );
        assert_eq!(
            eligibility(
                &[
                    mapped("github.com/acme/web", "t1"),
                    mapped("github.com/acme/api", "t1"),
                    mapped("github.com/other/x", "t2")
                ],
                &ws
            ),
            Ok(Eligible {
                tenant_id: "t1".into(),
                repo_keys: vec!["github.com/acme/api".into(), "github.com/acme/web".into()]
            })
        );
        assert_eq!(
            eligibility(
                &[mapped("github.com/acme/api", "t1"), mapped("github.com/acme/web", "t2")],
                &ws
            ),
            Err(Ineligible::Ambiguous)
        );
        assert_eq!(Ineligible::NotShared.code(), "tracker_not_shared");
        assert_eq!(Ineligible::Ambiguous.code(), "tracker_workspace_ambiguous");
    }

    #[test]
    fn facts_json_has_a_fixed_order_and_exactly_the_d3_keys() {
        let v = facts_json(&facts());
        let sprints: Vec<&str> = v["sprints"]
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["external_id"].as_str().unwrap())
            .collect();
        assert_eq!(sprints, ["101", "102"]);
        let tickets: Vec<&str> =
            v["tickets"].as_array().unwrap().iter().map(|t| t["key"].as_str().unwrap()).collect();
        assert_eq!(tickets, ["AUTH-10", "AUTH-2"]);
        assert_eq!(v["tickets"][0]["sprints"], json!(["101", "102"]));
        let mut keys: Vec<&str> =
            v["tickets"][0].as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort();
        assert_eq!(
            keys,
            ["issue_created_at", "key", "points", "sprints", "status_category", "title"]
        );
        let mut sk: Vec<&str> =
            v["sprints"][0].as_object().unwrap().keys().map(String::as_str).collect();
        sk.sort();
        assert_eq!(
            sk,
            [
                "complete_at",
                "end_at",
                "external_id",
                "name",
                "removed",
                "sdlc",
                "seq",
                "start_at",
                "state"
            ]
        );
        assert_eq!(v["pending_tickets"], json!(0));
        assert_eq!(v["predictability"], json!(12.5));
        assert_eq!(v["history_at"], json!("2026-11-04T09:00:00Z"));
    }

    #[test]
    fn a_removed_sprint_travels_with_no_figures() {
        let mut f = facts();
        f.sprints[0].removed = true;
        f.sprints[0].sdlc = None;
        let v = facts_json(&f);
        let removed =
            v["sprints"].as_array().unwrap().iter().find(|s| s["external_id"] == "102").unwrap();
        assert_eq!(removed["removed"], json!(true));
        assert_eq!(removed["sdlc"], Value::Null);
    }

    #[test]
    fn the_hash_ignores_read_order_key_order_and_history_at() {
        let a = facts();
        let mut b = facts();
        b.sprints.reverse();
        b.tickets.reverse();
        b.tickets[1].sprints.reverse();
        b.history_at = Some(at(11));
        assert_eq!(
            facts_hash("jira", "AUTH Scrum", &facts_json(&a)),
            facts_hash("jira", "AUTH Scrum", &facts_json(&b))
        );
        // Same content, keys inserted in another order.
        let v = facts_json(&a);
        let mut reordered = serde_json::Map::new();
        for (k, val) in v.as_object().unwrap().iter().rev() {
            reordered.insert(k.clone(), val.clone());
        }
        assert_eq!(
            facts_hash("jira", "AUTH Scrum", &v),
            facts_hash("jira", "AUTH Scrum", &Value::Object(reordered))
        );
        // A real change moves it.
        let mut c = facts();
        c.tickets[0].title = Some("renamed".into());
        assert_ne!(
            facts_hash("jira", "AUTH Scrum", &facts_json(&a)),
            facts_hash("jira", "AUTH Scrum", &facts_json(&c))
        );
        let mut d = facts();
        d.pending_tickets = 1;
        assert_ne!(
            facts_hash("jira", "AUTH Scrum", &facts_json(&a)),
            facts_hash("jira", "AUTH Scrum", &facts_json(&d))
        );
    }

    #[test]
    fn a_renamed_board_or_another_kind_hashes_differently() {
        let v = facts_json(&facts());
        assert_ne!(facts_hash("jira", "Mobile Scrum", &v), facts_hash("jira", "Mobile Team", &v));
        assert_ne!(
            facts_hash("jira", "Mobile Scrum", &v),
            facts_hash("linear", "Mobile Scrum", &v)
        );
    }

    #[test]
    fn the_body_names_the_tenant_repositories_and_board_and_carries_no_url() {
        let to = Eligible { tenant_id: "t1".into(), repo_keys: vec!["github.com/acme/api".into()] };
        let key = board_key("jira", "https://acme.atlassian.net", "42");
        let body = request_body(&to, &key, "jira", "Mobile Scrum", &facts_json(&facts()));
        assert_eq!(body["v"], json!(1));
        assert_eq!(body["tenant_id"], json!("t1"));
        assert_eq!(body["repo_keys"], json!(["github.com/acme/api"]));
        assert_eq!(body["board_key"], json!(key));
        assert_eq!(body["board_name"], json!("Mobile Scrum"));
        assert_eq!(body["effort"], Value::Null);
        assert!(!body.to_string().contains("atlassian"));
    }

    #[test]
    fn a_board_over_5000_tickets_or_3_5_mb_is_too_large() {
        let small = json!({"x": "y"});
        assert!(sized_body(&small, 10).is_ok());
        assert_eq!(sized_body(&small, MAX_TICKETS + 1).unwrap_err().tickets, MAX_TICKETS + 1);
        let big = json!({"x": "y".repeat(MAX_BYTES)});
        let err = sized_body(&big, 1).unwrap_err();
        assert!(err.bytes > MAX_BYTES);
    }

    #[test]
    fn a_part_is_due_unless_acknowledged_lately_or_refused_lately() {
        let now = at(12);
        assert!(is_due(None, "h1", now));
        let acked =
            MarkState { hash: Some("h1".into()), pushed_at: Some(at(10)), ..Default::default() };
        assert!(!is_due(Some(&acked), "h1", now));
        assert!(is_due(Some(&acked), "h2", now));
        let old = MarkState { pushed_at: Some(at(5)), ..acked.clone() };
        assert!(is_due(Some(&old), "h1", now), "a mark older than 6 h re-sends");
        let refused = MarkState {
            failed_hash: Some("h2".into()),
            failed_at: Some(at(11) + Duration::minutes(30)),
            ..acked.clone()
        };
        assert!(!is_due(Some(&refused), "h2", now), "a hash refused within the hour waits");
        assert!(is_due(Some(&refused), "h3", now), "a changed hash goes at once");
        let refused_long_ago = MarkState { failed_at: Some(at(10)), ..refused.clone() };
        assert!(is_due(Some(&refused_long_ago), "h2", now));
        let attempted_only = MarkState::default();
        assert!(is_due(Some(&attempted_only), "h1", now));
    }
}
