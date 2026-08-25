//! Relay nudge pickup (P1, feature e) — surface a human's steer for a run.
//!
//! The phone can send an unsolicited nudge/chat TO a run (`sendNudge` →
//! `POST relay/nudge`, direction `human_to_agent`). This module is the PURE,
//! testable core that picks those out of a polled inbox for one run: given the
//! rows [`crate::dojo::client::DojoClient::poll_inbox`] returned, select the
//! human→agent steer rows for this run so the run loop can surface them (log /
//! control-channel).
//!
//! **STEER, not drive.** This only *surfaces* the human's message — it never
//! consumes it to autonomously advance the run. `SENSEI_RUN_DRIVE` stays OFF; the
//! nudge is observability + a manual-steer signal, not an execution trigger. A
//! reply that answers a hard-block *gate* is a different path (that is the gate's
//! own `await_reply`, blocking-PreToolUse leg — not this pickup).

use dojo_protocol::relay::{RelayInboxItem, RelayMessageDirection};

/// One picked-up steer message for a run — the logical, code-free fields worth
/// surfacing. `text` is the human's message (best-effort out of the stripped
/// payload); `kind`/`id`/`answered_at` locate it. Never carries code or diffs.
#[derive(Debug, Clone, PartialEq)]
pub struct Nudge {
    /// The inbox row id (`relay_inbox.id`).
    pub id: Option<String>,
    /// `relay_inbox_kind` — typically `nudge` or `chat`.
    pub kind: String,
    /// The human's message, pulled from the stripped payload (`text`/`prompt`/
    /// `message`), or empty when the payload carries none.
    pub text: String,
    /// When the human sent it (RFC-3339), if the row carries it.
    pub answered_at: Option<String>,
}

/// Best-effort human message out of a stripped inbox payload. Reads the common
/// keys the phone writes (`text`, then `message`, then `prompt`); empty when none
/// is a string. Never reaches into anything code-bearing (there is none — the
/// payload is confidentiality-checked upstream).
fn nudge_text(payload: &serde_json::Value) -> String {
    for key in ["text", "message", "prompt"] {
        if let Some(s) = payload.get(key).and_then(serde_json::Value::as_str) {
            return s.to_string();
        }
    }
    String::new()
}

/// Pick the human→agent steer rows for `run_id` out of a polled inbox batch.
///
/// A row surfaces iff it is directed `human_to_agent` (the human steering, not the
/// daemon asking) AND belongs to this run. Direction is the discriminator: an
/// `agent_to_human` gate the daemon raised is NOT a pickup (it flows back through
/// the gate's own reply path). Pure — no I/O, no dedup state; the caller decides
/// what to do with the surfaced nudges (log / control-channel).
pub fn pickup_nudges(items: &[RelayInboxItem], run_id: &str) -> Vec<Nudge> {
    items
        .iter()
        .filter(|it| it.direction == RelayMessageDirection::HumanToAgent && it.run_id == run_id)
        .map(|it| Nudge {
            id: it.id.clone(),
            kind: it.kind.as_db_str().to_string(),
            text: nudge_text(&it.payload),
            answered_at: it.answered_at.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dojo_protocol::relay::{RelayInboxKind, RelayInboxStatus};

    fn item(
        id: &str,
        run_id: &str,
        kind: RelayInboxKind,
        direction: RelayMessageDirection,
        payload: serde_json::Value,
    ) -> RelayInboxItem {
        RelayInboxItem {
            id: Some(id.into()),
            run_id: run_id.into(),
            segment_id: None,
            kind,
            direction,
            status: RelayInboxStatus::Answered,
            payload,
            reply: None,
            created_at: None,
            answered_at: Some("2026-07-24T10:10:00Z".into()),
        }
    }

    #[test]
    fn picks_human_to_agent_rows_for_the_run() {
        let items = vec![
            // A human nudge for our run → picked up.
            item(
                "n1",
                "run-1",
                RelayInboxKind::Nudge,
                RelayMessageDirection::HumanToAgent,
                serde_json::json!({ "text": "try the other adapter" }),
            ),
            // A human chat for our run → picked up.
            item(
                "c1",
                "run-1",
                RelayInboxKind::Chat,
                RelayMessageDirection::HumanToAgent,
                serde_json::json!({ "message": "how's it going?" }),
            ),
        ];
        let got = pickup_nudges(&items, "run-1");
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].id.as_deref(), Some("n1"));
        assert_eq!(got[0].kind, "nudge");
        assert_eq!(got[0].text, "try the other adapter");
        assert_eq!(got[1].kind, "chat");
        assert_eq!(got[1].text, "how's it going?");
        assert_eq!(got[0].answered_at.as_deref(), Some("2026-07-24T10:10:00Z"));
    }

    #[test]
    fn ignores_agent_to_human_gates_and_other_runs() {
        let items = vec![
            // The daemon's own gate (agent→human) is NOT a pickup — it flows back
            // via the gate reply path, not this steer channel.
            item(
                "g1",
                "run-1",
                RelayInboxKind::Approval,
                RelayMessageDirection::AgentToHuman,
                serde_json::json!({ "prompt": "Approve Bash?" }),
            ),
            // A human nudge, but for a DIFFERENT run → not ours.
            item(
                "n2",
                "run-2",
                RelayInboxKind::Nudge,
                RelayMessageDirection::HumanToAgent,
                serde_json::json!({ "text": "not for run-1" }),
            ),
        ];
        assert!(pickup_nudges(&items, "run-1").is_empty());
    }

    #[test]
    fn empty_text_when_payload_has_no_message() {
        let items = vec![item(
            "n3",
            "run-1",
            RelayInboxKind::Nudge,
            RelayMessageDirection::HumanToAgent,
            serde_json::json!({ "unrelated": 1 }),
        )];
        let got = pickup_nudges(&items, "run-1");
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].text, "", "no text/message/prompt ⇒ empty, not a panic");
    }
}
