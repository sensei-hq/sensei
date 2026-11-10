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
    /// Whole seconds since the board's last successful sync, on its database's clock;
    /// only for a tracker that reads no history (A5). Never hashed.
    pub synced_age_s: Option<i64>,
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
        "synced_age_s": f.synced_age_s,
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
/// its facts — in canonical JSON, without `history_at` and `synced_age_s`, which move
/// on every history run and every tick even when nothing was read (D3, A5). A renamed
/// board hashes differently.
pub fn facts_hash(kind: &str, board_name: &str, facts: &Value) -> String {
    let mut facts = facts.clone();
    if let Some(o) = facts.as_object_mut() {
        o.remove("history_at");
        o.remove("synced_age_s");
    }
    effort_hash(&json!({ "kind": kind, "board_name": board_name, "facts": facts }))
}

/// What a board sends as `synced_age_s` (A5), or `None` to skip it this tick.
/// `token_at` is the consistency token's `last_success_at`; `read` is the
/// `(last_success_at, age)` read after the facts. A null or moved sync skips the
/// board: never pushed without a sync (Data invariants), never with an age from a
/// sync the facts do not reflect. A negative age (the clock stepped back after the
/// sync) skips too: the dōjō refuses one, and 0 would claim the freshest copy. A
/// tracker that reads history sends no age.
pub fn sync_age(
    history: bool,
    token_at: Option<DateTime<Utc>>,
    read: (Option<DateTime<Utc>>, Option<i64>),
) -> Option<Option<i64>> {
    let (at, age) = read;
    if at.is_none() || at != token_at || age.is_some_and(|a| a < 0) {
        return None;
    }
    Some(if history { None } else { age })
}

/// One effort row (A2): the repositories its sessions came from, its values, and
/// the reason for every value that is `null`.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffortRow {
    pub repo_keys: Vec<String>,
    pub values: serde_json::Map<String, Value>,
    pub reasons: std::collections::BTreeMap<String, String>,
}

/// This persona's effort on one board (A2): the project row, each sprint's row by
/// `external_id`, each ticket's by key.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Effort {
    pub project: Option<EffortRow>,
    pub sprints: Vec<(String, EffortRow)>,
    pub tickets: Vec<(String, EffortRow)>,
}

/// The ticket wire fields, exactly (D1); nothing else of `TicketSdlc` travels.
#[cfg(test)]
const TICKET_WIRE: [&str; 28] = [
    "sessions",
    "api_cost_n",
    "api_cost_partial",
    "worked_ms",
    "prompts",
    "ai_token_cost",
    "compile_defects",
    "test_defects",
    "review_fixes",
    "escaped_task_defects",
    "build_ok",
    "build_fail",
    "runs_unknown",
    "unclassified_failures",
    "build_unrecovered",
    "recovery_ms_sum",
    "recovery_n",
    "on_hold_ms",
    "on_hold_count",
    "on_hold_by_reason",
    "execution_ms",
    "rework_ms",
    "rework_cycles",
    "effort_cost",
    "rework_cost",
    "work_started_at",
    "submitted_at",
    "submitted_via",
];

/// A ticket's row values from its figures over this persona's provable sessions
/// and their usage (D1, A3): each null keeps the local reason; `no_activity`
/// travels as 0. A null without a reason is a fault, never sent as unknown.
pub fn ticket_effort(
    t: &crate::sdlc::ticket::TicketSdlc,
    u: &crate::tracker::usage::Usage,
) -> Result<EffortRow, String> {
    // The figures as the app serves them (timestamps RFC 3339), so a value never
    // means one thing locally and another in the dōjō.
    let api = serde_json::to_value(t).map_err(|e| format!("ticket figures: {e}"))?;
    let served = |k: &str| api.get(k).filter(|v| !v.is_null()).cloned();
    let n = &t.numerators;
    let num = |v: Option<i64>| v.map(Value::from);
    // (wire field, value, the local figure whose reason explains a null)
    let fields: Vec<(&str, Option<Value>, &str)> = vec![
        ("worked_ms", num(n.worked_ms), "worked_minutes"),
        ("build_ok", num(n.build_ok), "build_runs"),
        ("build_fail", num(n.build_fail), "build_runs"),
        ("recovery_ms_sum", num(n.recovery_ms_sum), "build_unrecovered"),
        ("recovery_n", num(n.recovery_n), "build_unrecovered"),
        ("on_hold_ms", num(n.on_hold_ms), "on_hold_minutes"),
        ("execution_ms", num(n.execution_ms), "execution_hours"),
        ("rework_ms", num(n.rework_ms), "rework_minutes"),
    ];
    let same = [
        "prompts",
        "ai_token_cost",
        "compile_defects",
        "test_defects",
        "review_fixes",
        "escaped_task_defects",
        "runs_unknown",
        "unclassified_failures",
        "build_unrecovered",
        "on_hold_count",
        "on_hold_by_reason",
        "rework_cycles",
        "effort_cost",
        "rework_cost",
        "work_started_at",
        "submitted_at",
        "submitted_via",
    ];
    let mut row = EffortRow::default();
    row.values.insert("sessions".into(), json!(u.sessions));
    row.values.insert("api_cost_n".into(), json!(u.api_cost_n));
    row.values.insert("api_cost_partial".into(), json!(u.api_cost_partial));
    for (wire, value, figure) in fields.into_iter().chain(same.iter().map(|k| (*k, served(k), *k)))
    {
        match value {
            Some(v) => {
                row.values.insert(wire.into(), v);
            }
            None => {
                let why = t
                    .reasons
                    .get(figure)
                    .ok_or_else(|| format!("{wire} is unknown with no reason for {figure}"))?;
                // Measured and found none: a real zero (A3).
                if *why == "no_activity" {
                    row.values.insert(wire.into(), json!(0));
                } else {
                    row.values.insert(wire.into(), Value::Null);
                    row.reasons.insert(wire.into(), (*why).into());
                }
            }
        }
    }
    Ok(row)
}

/// A sprint's row values: its `linked_sprint` usage over provable sessions, whose
/// `_n` fields carry their own coverage (A3).
pub fn sprint_effort(u: &crate::tracker::usage::Usage) -> Result<EffortRow, String> {
    match serde_json::to_value(u).map_err(|e| format!("sprint usage: {e}"))? {
        Value::Object(values) => Ok(EffortRow { values, ..Default::default() }),
        other => Err(format!("sprint usage is not an object: {other}")),
    }
}

/// The project row (A2, A3): the provable sessions linked to the board's live
/// tickets, their tokens (`null` `no_usage` when no session has any) and prompts.
pub fn project_effort(
    sessions: i64,
    tokens: i64,
    without_tokens: i64,
    prompts: Result<i64, &'static str>,
) -> EffortRow {
    let mut row = EffortRow::default();
    row.values.insert("sessions".into(), json!(sessions));
    row.values.insert("sessions_without_tokens".into(), json!(without_tokens));
    if without_tokens >= sessions {
        row.values.insert("tokens".into(), Value::Null);
        row.reasons.insert("tokens".into(), "no_usage".into());
    } else {
        row.values.insert("tokens".into(), json!(tokens));
    }
    match prompts {
        Ok(p) => {
            row.values.insert("prompts".into(), json!(p));
        }
        Err(why) => {
            row.values.insert("prompts".into(), Value::Null);
            row.reasons.insert("prompts".into(), why.into());
        }
    }
    row
}

/// The effort part as JSON, in a fixed order: sprints by `external_id`, tickets by
/// key, each row's `repo_keys` sorted. Read order never changes the bytes.
pub fn effort_json(e: &Effort) -> Value {
    fn row(id: Option<(&str, &str)>, r: &EffortRow) -> Value {
        let mut keys = r.repo_keys.clone();
        keys.sort();
        keys.dedup();
        let mut o = json!({ "repo_keys": keys, "values": r.values, "reasons": r.reasons });
        if let (Some((name, id)), Some(map)) = (id, o.as_object_mut()) {
            map.insert(name.into(), json!(id));
        }
        o
    }
    let sorted = |rows: &[(String, EffortRow)], name: &str| {
        let mut rows: Vec<&(String, EffortRow)> = rows.iter().collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        rows.iter().map(|(id, r)| row(Some((name, id)), r)).collect::<Vec<_>>()
    };
    json!({
        "project": e.project.as_ref().map(|r| row(None, r)),
        "sprints": sorted(&e.sprints, "external_id"),
        "tickets": sorted(&e.tickets, "key"),
    })
}

/// sha256 of a part's canonical JSON: the `effort` mark's hash (A2), and the
/// facts' once their kept fields are chosen.
pub fn effort_hash(effort: &Value) -> String {
    use sha2::{Digest, Sha256};
    hex::encode(Sha256::digest(crate::canonical_json::canonical_json(effort).as_bytes()))
}

/// The effort part as sent: this install's id beside the rows, so one person's two
/// machines each replace only their own rows in the dōjō (A2). The install is the
/// local persona's id, a random pseudonym.
pub fn effort_from(source: uuid::Uuid, effort: &Value) -> Value {
    let mut sent = effort.clone();
    if let Some(o) = sent.as_object_mut() {
        o.insert("source".into(), json!(source.to_string()));
    }
    sent
}

/// The request body (D3): both parts, always (A2).
pub fn request_body(
    to: &Eligible,
    board_key: &str,
    kind: &str,
    board_name: &str,
    facts: &Value,
    effort: &Value,
) -> Value {
    json!({
        "v": 1,
        "tenant_id": to.tenant_id,
        "repo_keys": to.repo_keys,
        "board_key": board_key,
        "kind": kind,
        "board_name": board_name,
        "facts": facts,
        "effort": effort,
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
            synced_age_s: Some(90),
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
    fn the_sync_age_travels_and_is_not_hashed() {
        // A5: the age moves every tick, so hashing it would re-send every board.
        let a = facts();
        assert_eq!(facts_json(&a)["synced_age_s"], json!(90));
        let mut b = facts();
        b.synced_age_s = Some(4000);
        assert_eq!(
            facts_hash("gitlab", "Web", &facts_json(&a)),
            facts_hash("gitlab", "Web", &facts_json(&b))
        );
        b.synced_age_s = None;
        assert_eq!(facts_json(&b)["synced_age_s"], Value::Null);
        assert_eq!(
            facts_hash("gitlab", "Web", &facts_json(&a)),
            facts_hash("gitlab", "Web", &facts_json(&b))
        );
    }

    #[test]
    fn a_board_sends_its_sync_age_only_without_history_and_skips_a_moved_or_missing_sync() {
        let t = Some(at(9));
        // No history (GitHub, GitLab): the age read on the same sync travels.
        assert_eq!(sync_age(false, t, (t, Some(90))), Some(Some(90)));
        // History (Jira): nothing travels, so its order is untouched.
        assert_eq!(sync_age(true, t, (t, Some(90))), Some(None));
        // A sync landed while the facts were built: skipped this tick.
        assert_eq!(sync_age(false, t, (Some(at(10)), Some(1))), None);
        assert_eq!(sync_age(true, t, (Some(at(10)), Some(1))), None);
        // A clock stepped back after the sync gives a negative age: skipped, never
        // sent (the dōjō refuses it) nor clamped to 0 (the freshest a copy can look).
        assert_eq!(sync_age(false, t, (t, Some(-1))), None);
        // Reconnected (last_success_at null): never pushed (Data invariants).
        assert_eq!(sync_age(false, t, (None, None)), None);
        assert_eq!(sync_age(false, None, (None, None)), None);
        assert_eq!(sync_age(true, None, (None, None)), None);
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
        let effort = effort_json(&Effort::default());
        let body = request_body(&to, &key, "jira", "Mobile Scrum", &facts_json(&facts()), &effort);
        assert_eq!(body["v"], json!(1));
        assert_eq!(body["tenant_id"], json!("t1"));
        assert_eq!(body["repo_keys"], json!(["github.com/acme/api"]));
        assert_eq!(body["board_key"], json!(key));
        assert_eq!(body["board_name"], json!("Mobile Scrum"));
        assert_eq!(body["effort"], json!({ "project": null, "sprints": [], "tickets": [] }));
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

    fn ticket_figures() -> (crate::sdlc::ticket::TicketSdlc, crate::tracker::usage::Usage) {
        use crate::sdlc::classify::Category;
        use crate::sdlc::signals::{Signal, SignalKind as K};
        let sig = |kind, min: i64, until: Option<i64>, category| Signal {
            kind,
            at: min * 60_000,
            until: until.map(|m| m * 60_000),
            category,
            reason: None,
            reference: None,
        };
        // Worked 0–2, a build failed and recovered, never submitted.
        let sessions = [crate::sdlc::ticket::SessionSignals {
            current: true,
            failed: false,
            signals: vec![
                sig(K::CodeEdit, 0, None, None),
                sig(K::BuildFail, 1, None, Some(Category::Build)),
                sig(K::BuildOk, 2, None, Some(Category::Build)),
                sig(K::Slice, 0, Some(2), None),
            ],
        }];
        let usage = crate::tracker::usage::Usage {
            sessions: 1,
            api_cost: None,
            api_cost_n: 0,
            ..Default::default()
        };
        let settings = crate::sdlc::settings::MetricSettings {
            idle_threshold_minutes: 15,
            hourly_rate: Some(90.0),
            minutes_per_point: Some(30.0),
        };
        let t = crate::sdlc::ticket::compute(&crate::sdlc::ticket::TicketInput {
            sessions: &sessions,
            usage: &usage,
            settings: &settings,
            issue_created_at: None,
            estimate: crate::sdlc::ticket::Estimate::Points(3.0),
        });
        (t, usage)
    }

    #[test]
    fn a_ticket_row_carries_exactly_the_wire_fields_and_a_reason_for_each_null() {
        let (t, u) = ticket_figures();
        let row = ticket_effort(&t, &u).unwrap();
        let mut keys: Vec<&str> = row.values.keys().map(String::as_str).collect();
        keys.sort();
        let mut wire = TICKET_WIRE.to_vec();
        wire.sort();
        assert_eq!(keys, wire);
        assert_eq!(row.values["worked_ms"], json!(120_000));
        assert_eq!(
            (row.values["build_ok"].clone(), row.values["build_fail"].clone()),
            (json!(1), json!(1))
        );
        assert_eq!(row.values["recovery_ms_sum"], json!(60_000));
        assert_eq!(row.values["work_started_at"], json!("1970-01-01T00:00:00Z"));
        // Cost at this person's rate travels; the rate and calibration never do.
        assert_eq!(row.values["effort_cost"], json!(3.0));
        // Unknown, and not applicable, both keep their local reason.
        assert_eq!(row.values["ai_token_cost"], Value::Null);
        assert_eq!(row.reasons["ai_token_cost"], "not_priced");
        assert_eq!(row.values["submitted_at"], Value::Null);
        assert_eq!(row.reasons["submitted_at"], "not_submitted");
        assert_eq!(row.reasons["execution_ms"], "not_submitted");
        assert_eq!(row.reasons["rework_ms"], "not_submitted");
        for (k, v) in &row.values {
            assert_eq!(
                v.is_null(),
                row.reasons.contains_key(k),
                "{k}: a null has a reason, a value none"
            );
        }
        let body = serde_json::to_string(&row.values).unwrap();
        for never in ["expected_minutes", "time_variance_minutes", "estimated_points", "90"] {
            assert!(!body.contains(never), "{never} leaked: {body}");
        }
    }

    #[test]
    fn no_activity_travels_as_zero() {
        let (mut t, u) = ticket_figures();
        t.worked_minutes = None;
        t.numerators.worked_ms = None;
        t.reasons.insert("worked_minutes", "no_activity");
        let row = ticket_effort(&t, &u).unwrap();
        assert_eq!(row.values["worked_ms"], json!(0));
        assert!(!row.reasons.contains_key("worked_ms"));
    }

    #[test]
    fn a_null_without_a_reason_is_a_fault() {
        let (mut t, u) = ticket_figures();
        t.prompts = None;
        t.reasons.remove("prompts");
        assert!(ticket_effort(&t, &u).is_err());
    }

    #[test]
    fn a_sprint_row_is_its_usage_with_coverage_in_the_n_fields() {
        let u = crate::tracker::usage::Usage {
            sessions: 2,
            active_secs: Some(60.0),
            active_n: 1,
            ..Default::default()
        };
        let row = sprint_effort(&u).unwrap();
        assert_eq!(
            (row.values["sessions"].clone(), row.values["active_n"].clone()),
            (json!(2), json!(1))
        );
        assert_eq!(row.values["tokens_fresh"], Value::Null);
        assert!(row.reasons.is_empty(), "coverage is the _n field, never a reason");
        assert!(row.values.contains_key("by_source") && !row.values.contains_key("linked_other"));
    }

    #[test]
    fn a_project_row_without_any_tokens_is_unknown_not_zero() {
        let none = project_effort(3, 0, 3, Ok(7));
        assert_eq!(none.values["tokens"], Value::Null);
        assert_eq!(none.reasons["tokens"], "no_usage");
        let some = project_effort(3, 500, 1, Err("not_derived"));
        assert_eq!(
            (some.values["tokens"].clone(), some.values["sessions_without_tokens"].clone()),
            (json!(500), json!(1))
        );
        assert_eq!(some.values["prompts"], Value::Null);
        assert_eq!(some.reasons["prompts"], "not_derived");
        assert_eq!(some.values["sessions"], json!(3));
    }

    #[test]
    fn the_effort_hash_ignores_read_order() {
        let row =
            |k: &str| EffortRow { repo_keys: vec!["b".into(), k.into()], ..Default::default() };
        let a = Effort {
            project: None,
            sprints: vec![("2".into(), row("a")), ("1".into(), row("c"))],
            tickets: vec![("T-2".into(), row("a")), ("T-1".into(), row("c"))],
        };
        let b = Effort {
            project: None,
            sprints: vec![("1".into(), row("c")), ("2".into(), row("a"))],
            tickets: vec![("T-1".into(), row("c")), ("T-2".into(), row("a"))],
        };
        assert_eq!(effort_json(&a), effort_json(&b));
        assert_eq!(effort_hash(&effort_json(&a)), effort_hash(&effort_json(&b)));
        let j = effort_json(&a);
        assert_eq!(j["tickets"][0]["key"], "T-1");
        assert_eq!(j["tickets"][0]["repo_keys"], json!(["b", "c"]));
        assert_eq!(j["sprints"][0]["external_id"], "1");
        assert_eq!(j["project"], Value::Null);
        let mut c = a.clone();
        c.tickets.pop();
        assert_ne!(effort_hash(&effort_json(&a)), effort_hash(&effort_json(&c)));
    }

    #[test]
    fn the_effort_sent_names_its_install_and_keeps_its_rows() {
        let rows = effort_json(&Effort::default());
        let id = uuid::Uuid::new_v4();
        let sent = effort_from(id, &rows);
        assert_eq!(sent["source"], json!(id.to_string()));
        assert_eq!((sent["project"].clone(), sent["tickets"].clone()), (Value::Null, json!([])));
        assert_eq!(sent.as_object().unwrap().len(), 4);
    }
}
