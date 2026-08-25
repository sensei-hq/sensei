//! Relay P5.2 — the ACP (Agent Client Protocol) **observe-first** backend.
//!
//! The daemon acts as an ACP *client* that OBSERVES an ACP-speaking agent's
//! session (Zed + adopters) and surfaces its progress into the relay outline —
//! **no drive, no gate/permission handling** (those are P5.2b+). The observe
//! direction reads the agent's `session/update` notification stream
//! ([`SessionNotification`] / [`SessionUpdate`], the spec's own wire types from
//! the Apache-2.0 `agent-client-protocol` SDK) and projects it into the SAME
//! [`RelaySegment`] shape the [`crate::dojo::relay_project`] TodoWrite projection
//! produces, published via the SAME [`crate::dojo::client::DojoClient::upsert_segments`]
//! path (DRY — one segment shape, one publish seam for every backend).
//!
//! **Zero-knowledge (relay-engine D10):** a segment carries only summarized
//! *logical* status — a plan entry's `content` (a short human phrase) and its
//! state. NEVER code, diffs, file contents, or raw tool args. The ACP wire types
//! expose `raw_input` / `raw_output` / diff `content` on a [`ToolCall`]; the
//! mapping below **never** reads those fields — [`acp_update_to_segments`] only
//! ever copies a plan entry's `content` (exactly as the todo projection copies a
//! todo's `content`), and the leak-guard tests assert nothing else crosses.
//!
//! ## What is done vs deferred (P5.2b)
//!
//! - **Done + heavily tested:** the PURE [`acp_update_to_segments`] mapping (the
//!   core value), and the [`AcpObserveDriver`] behind the P5.1 [`RunDriver`] seam
//!   with `capability() == DriveCapability::Acp`. Since this is observe-first,
//!   `drive_step` is intentionally NOT the ACP path — it returns a clear
//!   unsupported error (drive-over-ACP is P5.2b). The observe direction is a new,
//!   minimal, backward-compatible seam method [`RunDriver::observe_update`]
//!   (default-`None` on the trait, so ClaudeDriver is unaffected) that runs the
//!   pure mapping.
//! - **Deferred to P5.2b (`// FOLLOW-UP`):** the impure live connection loop —
//!   spawning/connecting a real ACP agent over stdio JSON-RPC via the SDK's
//!   builder/`ConnectionTo` actor runtime, receiving its `SessionNotification`
//!   stream, and calling `upsert_segments` per update. In `agent-client-protocol`
//!   1.x the client side is a role/builder/dispatch runtime (not a simple handler
//!   trait), so a live loop can't be cleanly unit-tested here; per the plan we
//!   ship the tested core now and wire the live loop in P5.2b rather than land an
//!   untested loop.

use super::trait_def::{DriveCapability, DriveStep, RunDriver};
use crate::agent_spawn::{AgentOutput, AgentSpawnError};
use agent_client_protocol::schema::v1::{PlanEntryStatus, SessionUpdate};
use dojo_protocol::relay::{RelaySegment, SegmentState};

/// Map an ACP [`PlanEntryStatus`] to a relay [`SegmentState`]. Mirrors the
/// TodoWrite projection's `todo_state` exactly (pending → Pending, in-progress →
/// Active, completed → Done) so an ACP-observed run and a Claude-observed run
/// render identically on the phone. `#[non_exhaustive]` on the SDK enum forces a
/// wildcard arm → any future status defaults to `Pending` (safe, never leaks).
fn plan_entry_state(status: &PlanEntryStatus) -> SegmentState {
    match status {
        PlanEntryStatus::Completed => SegmentState::Done,
        PlanEntryStatus::InProgress => SegmentState::Active,
        // Pending + any future non_exhaustive variant → Pending.
        _ => SegmentState::Pending,
    }
}

/// **The heavily-TDD'd core.** Project one ACP [`SessionUpdate`] into relay
/// outline segments — the phone outline — in the SAME [`RelaySegment`] shape the
/// TodoWrite projection produces.
///
/// The agent's [`SessionUpdate::Plan`] is the direct analog of Claude's
/// TodoWrite list (the agent's own task list), so it — and only it — becomes the
/// outline: each [`agent_client_protocol::schema::v1::PlanEntry`] → one top-level
/// segment (`seq` = index, `title` = the entry `content`, `state` from the entry
/// status). Server ids are `None` (the Worker upserts by session+seq), matching
/// [`crate::dojo::relay_project::todos_to_segments`].
///
/// Every other update variant — message/thought chunks, tool calls, mode/config/
/// usage — projects to an EMPTY outline: those are transient activity (or carry
/// code/args), not outline nodes, and surfacing them is a later P5.2b enrichment.
/// Returning empty here is the zero-knowledge default: a [`ToolCall`]'s
/// `raw_input` / `raw_output` / diff `content` are NEVER read, so no code, diff,
/// or tool argument can cross into a segment field.
///
/// [`ToolCall`]: agent_client_protocol::schema::v1::ToolCall
pub fn acp_update_to_segments(update: &SessionUpdate) -> Vec<RelaySegment> {
    match update {
        SessionUpdate::Plan(plan) => plan
            .entries
            .iter()
            .enumerate()
            .map(|(i, entry)| RelaySegment {
                id: None,
                parent_id: None,
                parent_seq: None,
                seq: i as i32,
                // Only the entry's human-readable `content` crosses — a short
                // phrase, exactly like a todo's `content`. No _meta, no args.
                title: entry.content.clone(),
                summary: None,
                detail: None,
                state: plan_entry_state(&entry.status),
                is_gate: false,
                gate_severity: None,
                response_verdict: None,
                response_note: None,
                agent: None,
                model: None,
                spec_ref: None,
            })
            .collect(),
        // Not part of the outline (transient activity / carries code/args) — an
        // empty projection keeps the mapping zero-knowledge by construction.
        _ => Vec::new(),
    }
}

/// The ACP observe-first backend behind the P5.1 [`RunDriver`] seam.
///
/// `capability()` is [`DriveCapability::Acp`]. This is **observe-only**: it does
/// not spawn/drive an agent (`drive_step` returns unsupported — drive-over-ACP is
/// P5.2b), it *observes* an ACP session's update stream and projects it via
/// [`RunDriver::observe_update`] → [`acp_update_to_segments`].
pub struct AcpObserveDriver;

impl RunDriver for AcpObserveDriver {
    fn id(&self) -> &str {
        "acp"
    }

    fn capability(&self) -> DriveCapability {
        DriveCapability::Acp
    }

    async fn drive_step(&self, _step: DriveStep<'_>) -> Result<AgentOutput, AgentSpawnError> {
        // Observe-first: this backend does not DRIVE. Drive-over-ACP (spawning +
        // hosting an ACP session the daemon steers) is P5.2b. Surface a clear
        // launch-style error rather than silently no-op, so a caller that wires
        // ACP into a drive tick by mistake fails loudly.
        Err(AgentSpawnError::Spawn(
            "acp backend is observe-only (drive-over-acp is P5.2b)".to_string(),
        ))
    }

    fn observe_update(&self, update: &SessionUpdate) -> Option<Vec<RelaySegment>> {
        // The observe seam: ACP contributes the outline projection.
        Some(acp_update_to_segments(update))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agent_client_protocol::schema::v1::{
        ContentBlock, Plan, PlanEntry, PlanEntryPriority, PlanEntryStatus, TextContent, ToolCall,
        ToolCallContent, ToolCallStatus, ToolCallUpdate, ToolCallUpdateFields, ToolKind,
    };

    fn entry(content: &str, status: PlanEntryStatus) -> PlanEntry {
        PlanEntry::new(content, PlanEntryPriority::Medium, status)
    }

    // ---- the pure mapping: Plan → outline segments (mirrors the todo projection) ----

    #[test]
    fn plan_update_maps_entries_to_segments_with_state() {
        let plan = Plan::new(vec![
            entry("add publish_gate", PlanEntryStatus::Completed),
            entry("poll_inbox", PlanEntryStatus::InProgress),
            entry("wire /v1/relay", PlanEntryStatus::Pending),
        ]);
        let segs = acp_update_to_segments(&SessionUpdate::Plan(plan));
        assert_eq!(segs.len(), 3);

        assert_eq!(segs[0].seq, 0);
        assert_eq!(segs[0].title, "add publish_gate");
        assert_eq!(segs[0].state, SegmentState::Done);

        assert_eq!(segs[1].seq, 1);
        assert_eq!(segs[1].title, "poll_inbox");
        assert_eq!(segs[1].state, SegmentState::Active);

        assert_eq!(segs[2].seq, 2);
        assert_eq!(segs[2].title, "wire /v1/relay");
        assert_eq!(segs[2].state, SegmentState::Pending);

        // Same invariants as todos_to_segments: no gate, no server ids, no nesting.
        assert!(segs.iter().all(|s| !s.is_gate && s.id.is_none() && s.parent_id.is_none()));
    }

    #[test]
    fn empty_plan_maps_to_empty_outline() {
        let segs = acp_update_to_segments(&SessionUpdate::Plan(Plan::new(vec![])));
        assert!(segs.is_empty());
    }

    #[test]
    fn plan_entry_state_covers_all_statuses() {
        assert_eq!(plan_entry_state(&PlanEntryStatus::Pending), SegmentState::Pending);
        assert_eq!(plan_entry_state(&PlanEntryStatus::InProgress), SegmentState::Active);
        assert_eq!(plan_entry_state(&PlanEntryStatus::Completed), SegmentState::Done);
    }

    // ---- ZERO-KNOWLEDGE: no code / diffs / tool args ever leak into a segment ----

    #[test]
    fn plan_projection_carries_only_content_no_meta_no_detail() {
        // Only the entry `content` becomes a title; summary/detail stay None and
        // the SDK's `_meta` (arbitrary jsonb) never crosses. Mirrors the todo
        // projection's `projection_carries_no_code_or_diffs`.
        let plan = Plan::new(vec![entry("x", PlanEntryStatus::Pending)]);
        let segs = acp_update_to_segments(&SessionUpdate::Plan(plan));
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].title, "x");
        assert!(segs[0].summary.is_none());
        assert!(segs[0].detail.is_none());
        assert!(segs[0].response_verdict.is_none());
        assert!(segs[0].response_note.is_none());
    }

    #[test]
    fn tool_call_update_never_projects_code_or_args() {
        // A ToolCall carries `raw_input`/`raw_output` (tool args + results) and a
        // diff/text `content` payload — all forbidden by D10. The mapping must
        // NOT surface a tool call in the outline at all, so none of it can leak.
        let mut tc = ToolCall::new("call-1", "Edit crates/senseid/src/secret.rs");
        tc = tc
            .kind(ToolKind::Edit)
            .status(ToolCallStatus::Completed)
            .raw_input(serde_json::json!({
                "file_path": "/Users/jerry/.ssh/id_rsa",
                "command": "cat ~/.aws/credentials"
            }))
            .raw_output(serde_json::json!({"stdout": "AKIA_SECRET_KEY_LEAKED"}))
            .content(vec![ToolCallContent::from(ContentBlock::Text(TextContent::new(
                "- old secret line\n+ new secret line",
            )))]);

        let segs = acp_update_to_segments(&SessionUpdate::ToolCall(tc));
        assert!(segs.is_empty(), "tool calls are not outline nodes → nothing leaks");
    }

    #[test]
    fn tool_call_update_variant_never_projects() {
        // The incremental ToolCallUpdate (status/args deltas) likewise never
        // becomes a segment — same zero-knowledge guarantee.
        let update = ToolCallUpdate::new(
            "call-1",
            ToolCallUpdateFields::new()
                .status(ToolCallStatus::Failed)
                .raw_output(serde_json::json!({"error": "leaked internal path /etc/passwd"})),
        );
        let segs = acp_update_to_segments(&SessionUpdate::ToolCallUpdate(update));
        assert!(segs.is_empty());
    }

    #[test]
    fn message_and_thought_chunks_never_project() {
        // Streamed assistant prose / internal reasoning are not outline nodes and
        // could carry anything → empty projection (no leak surface).
        use agent_client_protocol::schema::v1::ContentChunk;
        let chunk = ContentChunk::new(ContentBlock::Text(TextContent::new(
            "here is the private API key sk-verysecret",
        )));
        assert!(
            acp_update_to_segments(&SessionUpdate::AgentMessageChunk(chunk.clone())).is_empty()
        );
        assert!(
            acp_update_to_segments(&SessionUpdate::AgentThoughtChunk(chunk.clone())).is_empty()
        );
        assert!(acp_update_to_segments(&SessionUpdate::UserMessageChunk(chunk)).is_empty());
    }

    // ---- the driver behind the P5.1 seam ----

    #[test]
    fn id_is_acp_and_capability_is_acp() {
        let d = AcpObserveDriver;
        assert_eq!(d.id(), "acp");
        assert_eq!(d.capability(), DriveCapability::Acp);
    }

    #[tokio::test]
    async fn drive_step_is_unsupported_observe_only() {
        use std::path::Path;
        use std::time::Duration;
        // Observe-first: drive_step must fail loudly (drive-over-ACP is P5.2b),
        // never silently succeed.
        let d = AcpObserveDriver;
        let err = d
            .drive_step(DriveStep {
                cwd: Path::new("."),
                prompt: "would drive",
                timeout: Duration::from_secs(1),
            })
            .await
            .expect_err("acp is observe-only");
        match err {
            AgentSpawnError::Spawn(msg) => assert!(msg.contains("observe-only"), "msg was {msg:?}"),
            other => panic!("expected a Spawn error, got {other:?}"),
        }
    }

    #[test]
    fn observe_update_runs_the_pure_mapping() {
        // The seam method delegates to acp_update_to_segments: a Plan update
        // yields the outline; a ToolCall yields an empty (present) projection.
        let d = AcpObserveDriver;
        let plan = Plan::new(vec![entry("do a thing", PlanEntryStatus::InProgress)]);
        let segs = d.observe_update(&SessionUpdate::Plan(plan)).expect("acp driver observes");
        assert_eq!(segs.len(), 1);
        assert_eq!(segs[0].title, "do a thing");
        assert_eq!(segs[0].state, SegmentState::Active);
    }
}
