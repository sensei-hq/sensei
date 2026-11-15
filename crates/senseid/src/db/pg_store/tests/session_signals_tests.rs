//! The session-signal derivation pass against a real database (spec
//! tracker-sdlc-metrics S2, gate 2b): what is selected, what is stored, and that a
//! failing session never blocks the ones after it. Each test has its own database,
//! so pass counts are exact.
use super::*;
use crate::tasks::session_signals::pass;
use serde_json::json;

const T0: i64 = 1_790_000_000_000;
const NOW: i64 = T0 + 86_400_000;

async fn session(s: &PgStore, minutes_ago: i64) -> (uuid::Uuid, String) {
    let client = format!("sig-{}", uuid::Uuid::new_v4());
    let (id,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO activity.sessions (client_session_id, started_at)
         VALUES ($1, now() - make_interval(mins => $2)) RETURNING id",
    )
    .bind(&client)
    .bind(minutes_ago as i32)
    .fetch_one(s.pool())
    .await
    .unwrap();
    (id, client)
}

/// One captured event; `id` explicit when the test needs to control id order.
async fn event(
    s: &PgStore,
    client: &str,
    id: Option<i64>,
    event_type: &str,
    tool: Option<&str>,
    ts: i64,
    payload: serde_json::Value,
) -> i64 {
    let (id,): (i64,) = sqlx_core::query_as::query_as(
        "INSERT INTO activity.assistant_events (id, session_id, family, event_type, tool_name, ts, payload)
         VALUES (coalesce($1, nextval(pg_get_serial_sequence('activity.assistant_events', 'id'))),
                 $2, 'claude', $3, $4, $5, $6)
         RETURNING id",
    )
    .bind(id)
    .bind(client)
    .bind(event_type)
    .bind(tool)
    .bind(ts)
    .bind(payload)
    .fetch_one(s.pool())
    .await
    .unwrap();
    id
}

async fn mark(s: &PgStore, id: uuid::Uuid) -> Option<(i64, i32, String, Option<String>)> {
    sqlx_core::query_as::query_as(
        "SELECT through_event_id, event_count, catalogue_hash, error
           FROM activity.session_signal_marks WHERE session_id = $1",
    )
    .bind(id)
    .fetch_optional(s.pool())
    .await
    .unwrap()
}

async fn kinds(s: &PgStore, id: uuid::Uuid) -> Vec<String> {
    let rows: Vec<(String,)> = sqlx_core::query_as::query_as(
        "SELECT kind::text FROM activity.session_signals WHERE session_id = $1 ORDER BY seq",
    )
    .bind(id)
    .fetch_all(s.pool())
    .await
    .unwrap();
    rows.into_iter().map(|r| r.0).collect()
}

async fn exec(s: &PgStore, sql: &str) {
    sqlx_core::query::query(sql).execute(s.pool()).await.unwrap_or_else(|e| panic!("{sql}: {e}"));
}

#[tokio::test]
async fn a_pass_derives_marks_what_it_read_and_then_rests() {
    let s = PgStore::test_store().await;
    let (id, c) = session(&s, 10).await;
    event(
        &s,
        &c,
        None,
        "PostToolUse",
        Some("Edit"),
        T0,
        json!({"tool_input": {"file_path": "/r/src/a.rs"}}),
    )
    .await;
    let last = event(
        &s,
        &c,
        None,
        "PostToolUse",
        Some("Bash"),
        T0 + 1_000,
        json!({"tool_use_id": "t1", "tool_input": {"command": "cargo test 2>&1 | tail -1"},
               "tool_response": {"stdout": "test result: FAILED. 1 failed", "stderr": ""}}),
    )
    .await;
    let first = pass(&s, NOW, 10_000).await.expect("the pass runs");
    assert!(first.derived >= 1 && first.failed == 0);
    let m = mark(&s, id).await.unwrap();
    assert_eq!((m.0, m.1, m.3), (last, 2, None));
    assert_eq!(
        kinds(&s, id).await,
        ["code_edit", "slice", "test_fail"],
        "a slice sorts at its start"
    );

    let again = pass(&s, NOW, 10_000).await.unwrap();
    assert_eq!((again.derived, again.failed), (0, 0), "nothing changed, so nothing is selected");
}

#[tokio::test]
async fn a_catalogue_change_re_derives() {
    let s = PgStore::test_store().await;
    let (id, c) = session(&s, 10).await;
    event(&s, &c, None, "Stop", None, T0, json!({})).await;
    pass(&s, NOW, 10_000).await.unwrap();
    let before = mark(&s, id).await.unwrap().2;
    exec(&s, "INSERT INTO sensei.command_markers (toolchain, category, kind, marker) VALUES ('cargo', 'test', 'fail', 'zzz')").await;
    let r = pass(&s, NOW, 10_000).await.unwrap();
    assert!(r.derived >= 1);
    let after = mark(&s, id).await.unwrap().2;
    assert_ne!(after, before);
    assert_eq!(after, s.command_catalogue().await.unwrap().hash);
}

#[tokio::test]
async fn a_failing_session_is_recorded_and_never_blocks_the_next() {
    let s = PgStore::test_store().await;
    let (bad, bc) = session(&s, 600).await;
    let (good, gc) = session(&s, 5).await;
    event(&s, &bc, None, "Stop", None, 9_000_000_000_000_000_000, json!({})).await;
    event(&s, &gc, None, "Stop", None, T0, json!({})).await;

    let r = pass(&s, NOW, 10_000).await.expect("one bad session doesn't fail the pass");
    assert_eq!(r.failed, 1);
    assert!(mark(&s, bad).await.unwrap().3.unwrap().contains("9000000000000000000"));
    assert_eq!(mark(&s, good).await.unwrap().3, None);

    let again = pass(&s, NOW, 10_000).await.unwrap();
    assert_eq!((again.derived, again.failed), (0, 0), "not retried until its inputs change");
}

#[tokio::test]
async fn a_failure_removes_the_signals_it_supersedes() {
    let s = PgStore::test_store().await;
    let (id, c) = session(&s, 10).await;
    event(&s, &c, None, "UserPromptSubmit", None, T0, json!({})).await;
    pass(&s, NOW, 10_000).await.unwrap();
    assert!(!kinds(&s, id).await.is_empty());
    event(&s, &c, None, "Stop", None, 1_000, json!({})).await;
    pass(&s, NOW, 10_000).await.unwrap();
    assert!(kinds(&s, id).await.is_empty());
    assert!(mark(&s, id).await.unwrap().3.is_some());
}

#[tokio::test]
async fn a_store_failure_is_retried_once_it_clears() {
    let s = PgStore::test_store().await;
    let (id, c) = session(&s, 10).await;
    event(&s, &c, None, "UserPromptSubmit", None, T0, json!({})).await;
    exec(
        &s,
        "ALTER TABLE activity.session_signals ADD CONSTRAINT refuse_all CHECK (false) NOT VALID",
    )
    .await;
    let r = pass(&s, NOW, 10_000).await.unwrap();
    assert!(r.failed >= 1);
    assert!(mark(&s, id).await.unwrap().3.is_some());

    exec(&s, "ALTER TABLE activity.session_signals DROP CONSTRAINT refuse_all").await;
    pass(&s, NOW, 10_000).await.unwrap();
    let m = mark(&s, id).await.unwrap();
    assert_eq!((m.1, m.3), (1, None), "a write failure is transient: the session is retried");
}

#[tokio::test]
async fn an_event_committed_out_of_id_order_re_derives_the_session() {
    let s = PgStore::test_store().await;
    let (id, c) = session(&s, 10).await;
    event(&s, &c, None, "Stop", None, T0 + 2_000, json!({})).await;
    let high = event(&s, &c, None, "Stop", None, T0 + 3_000, json!({})).await;
    pass(&s, NOW, 10_000).await.unwrap();
    assert_eq!(mark(&s, id).await.map(|m| (m.0, m.1)), Some((high, 2)));

    // An id below the mark's, visible only now (as a slower commit would be).
    event(&s, &c, Some(high - 1_000_000), "UserPromptSubmit", None, T0 + 1_000, json!({})).await;
    pass(&s, NOW, 10_000).await.unwrap();
    assert_eq!(mark(&s, id).await.map(|m| m.1), Some(3));
    assert!(kinds(&s, id).await.contains(&"prompt".to_string()));
}

#[tokio::test]
async fn sessions_without_events_or_a_client_id() {
    let s = PgStore::test_store().await;
    let (empty, _) = session(&s, 10).await;
    let (orphan,): (uuid::Uuid,) = sqlx_core::query_as::query_as(
        "INSERT INTO activity.sessions (started_at) VALUES (now()) RETURNING id",
    )
    .fetch_one(s.pool())
    .await
    .unwrap();
    pass(&s, NOW, 10_000).await.unwrap();
    assert_eq!(mark(&s, empty).await.map(|m| (m.0, m.1, m.3)), Some((0, 0, None)));
    assert_eq!(mark(&s, orphan).await, None, "no client id, never selected");
}

#[tokio::test]
async fn the_reader_orders_tied_times_by_id() {
    let s = PgStore::test_store().await;
    let (_, c) = session(&s, 10).await;
    let base = 900_000_000_000;
    let high = event(&s, &c, Some(base + 2), "Stop", None, T0, json!({})).await;
    let low = event(&s, &c, Some(base + 1), "UserPromptSubmit", None, T0, json!({})).await;
    let ids: Vec<i64> =
        s.session_events_for_signals(&c).await.unwrap().iter().map(|e| e.id).collect();
    assert_eq!(ids, [low, high]);
}

/// Every projected field, from payloads shaped as Claude Code sends them.
#[tokio::test]
async fn the_reader_projects_each_field_from_real_payload_shapes() {
    let s = PgStore::test_store().await;
    let (_, c) = session(&s, 10).await;
    let long = "a".repeat(100_000) + "TAIL";
    let cases = [
        (
            "PostToolUse",
            Some("Bash"),
            json!({"tool_use_id": "u1", "tool_input": {"command": "ls"},
            "tool_response": {"stdout": long, "stderr": "E"}}),
        ),
        (
            "PostToolUseFailure",
            Some("Bash"),
            json!({"tool_use_id": "u2", "tool_input": {"command": "x"},
            "error": "Exit code 1", "is_interrupt": true}),
        ),
        (
            "PostToolUse",
            Some("NotebookEdit"),
            json!({"tool_input": {"notebook_path": "/r/n.ipynb"}}),
        ),
        (
            "PostToolUse",
            Some("Write"),
            json!({"tool_input": {"file_path": "/r/x/tasks.md", "content": "- [x] a"},
            "tool_response": {"type": "update", "originalFile": "- [ ] a"}}),
        ),
        (
            "PostToolUse",
            Some("Agent"),
            json!({"tool_input": {"subagent_type": "sensei:rev"},
            "tool_response": {"agentId": "ag1", "status": "async_launched"}}),
        ),
        (
            "PostToolUse",
            Some("SubagentHandback"),
            json!({"agent_id": "ag1", "tool_input": {"message": "hb"},
            "tool_response": {"ok": true}}),
        ),
        ("SubagentStop", None, json!({"agent_id": "ag2", "last_assistant_message": "last"})),
        ("Notification", None, json!({"notification_type": "permission_prompt"})),
        (
            "PostToolUse",
            Some("mcp__plugin_sensei_sensei__update_task_status"),
            json!({"tool_input": {"state": "done", "task_id": "t9"}, "tool_response": [{"type": "text", "text": "ok"}]}),
        ),
    ];
    for (i, (kind, tool, payload)) in cases.iter().enumerate() {
        event(&s, &c, None, kind, *tool, T0 + i as i64, payload.clone()).await;
    }
    let e = s.session_events_for_signals(&c).await.unwrap();
    let out = e[0].output.as_deref().unwrap();
    assert!(out.chars().count() <= 65_536 && out.ends_with('E') && out.contains("TAIL"));
    assert_eq!((e[0].command.as_deref(), e[0].tool_use_id.as_deref()), (Some("ls"), Some("u1")));
    assert!(e[1].output.as_deref().unwrap().contains("Exit code 1"));
    assert_eq!(e[1].is_interrupt, Some(true));
    assert_eq!(e[2].path.as_deref(), Some("/r/n.ipynb"));
    assert_eq!(
        (e[3].response_type.as_deref(), e[3].original_file.as_deref(), e[3].content.as_deref()),
        (Some("update"), Some("- [ ] a"), Some("- [x] a"))
    );
    assert_eq!(
        (e[4].subagent_type.as_deref(), e[4].agent_ref.as_deref()),
        (Some("sensei:rev"), Some("ag1"))
    );
    assert_eq!((e[5].agent_ref.as_deref(), e[5].message.as_deref()), (Some("ag1"), Some("hb")));
    assert_eq!(
        (e[6].agent_ref.as_deref(), e[6].last_assistant_message.as_deref()),
        (Some("ag2"), Some("last"))
    );
    assert_eq!(e[7].notification_type.as_deref(), Some("permission_prompt"));
    assert_eq!((e[8].state.as_deref(), e[8].task_id.as_deref()), (Some("done"), Some("t9")));
    assert!(e[8].output.as_deref().unwrap().contains("ok"), "an MCP tool's response is its text");
}

#[tokio::test]
async fn a_catalogue_that_does_not_compile_fails_the_pass_and_touches_nothing() {
    let s = PgStore::test_store().await;
    let (id, c) = session(&s, 10).await;
    event(&s, &c, None, "Stop", None, T0, json!({})).await;
    exec(&s, "INSERT INTO sensei.command_patterns (toolchain, pattern, category) VALUES ('broken', '(', 'build')").await;
    let err = pass(&s, NOW, 10_000).await.expect_err("a bad catalogue fails the pass");
    assert!(err.contains("broken"), "{err}");
    assert_eq!(mark(&s, id).await, None);
}
