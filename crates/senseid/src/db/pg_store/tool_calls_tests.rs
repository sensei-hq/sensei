//! The Replay timeline pairs each tool call with ITS response (#238).
//!
//! `sensei.session_tool_calls` used to pair PreToolUse with PostToolUse by
//! firing order within (session, tool). Parallel calls to one tool finish out of
//! order, so the order pairing put a request beside another call's response —
//! measured 2026-10-08: 183,376 of 264,589 paired rows (69%) joined two
//! different `tool_use_id`s. The duration came from the two hooks' own
//! timestamps on those wrong pairs.
//!
//! Both events carry Claude Code's `tool_use_id`, and the PostToolUse carries
//! the engine's `duration_ms` and the subagent's `agent_id` / `agent_type` —
//! all present in the payload, none read by any SQL before this.
use super::*;

fn event(
    session: &str,
    kind: &'static str,
    ts: i64,
    payload: serde_json::Value,
) -> (String, &'static str, i64, serde_json::Value) {
    (session.to_string(), kind, ts, payload)
}

async fn seed(pg: &PgStore, rows: &[(String, &'static str, i64, serde_json::Value)]) {
    for (session, kind, ts, payload) in rows {
        pg.insert_hook_event(&HookEventRow {
            session_id: session,
            assistant_family: "claude",
            event_type: kind,
            tool_name: Some("Bash"),
            cwd: None,
            ts: *ts,
            success: None,
            payload,
        })
        .await
        .unwrap();
    }
}

/// Mutations that must break this: pair by firing order again, or take the
/// duration from the hook timestamps instead of the engine's `duration_ms`.
#[tokio::test]
async fn each_call_is_paired_with_its_own_response_and_the_engines_duration() {
    let pg = PgStore::test_store().await;
    let session = format!("_test-replay-{}", uuid::Uuid::new_v4());
    let (a, b) = (format!("toolu_a_{session}"), format!("toolu_b_{session}"));
    // A starts first and finishes LAST; B overtakes it. Order pairing would put
    // B's response beside A's request.
    seed(
        &pg,
        &[
            event(
                &session,
                "PreToolUse",
                1_000,
                serde_json::json!({ "tool_use_id": a, "tool_input": { "command": "slow" } }),
            ),
            event(
                &session,
                "PreToolUse",
                1_010,
                serde_json::json!({ "tool_use_id": b, "tool_input": { "command": "fast" } }),
            ),
            event(
                &session,
                "PostToolUse",
                1_030,
                serde_json::json!({
                "tool_use_id": b, "duration_ms": 12, "agent_id": "agent-7", "agent_type": "Explore",
                "tool_response": { "stdout": "fast done" } }),
            ),
            event(
                &session,
                "PostToolUse",
                1_900,
                serde_json::json!({
                "tool_use_id": a, "duration_ms": 880, "tool_response": { "stdout": "slow done" } }),
            ),
        ],
    )
    .await;

    let calls = pg.get_session_tool_calls(&session, 10).await.unwrap();
    assert_eq!(calls.len(), 2, "{calls:#?}");
    let by = |id: &str| calls.iter().find(|c| c["toolUseId"] == id).expect(id).clone();

    let slow = by(&a);
    assert_eq!(slow["request"]["tool_input"]["command"], "slow");
    assert_eq!(
        slow["response"]["tool_response"]["stdout"], "slow done",
        "A's own response, not B's"
    );
    assert_eq!(slow["durationMs"], 880, "the engine's duration, not the hook gap (900)");
    assert!(slow["agentId"].is_null(), "a main-loop call has no subagent");

    let fast = by(&b);
    assert_eq!(fast["response"]["tool_response"]["stdout"], "fast done");
    assert_eq!(fast["durationMs"], 12);
    assert_eq!(fast["agentId"], "agent-7");
    assert_eq!(fast["agentType"], "Explore");
}

/// A call captured twice (identical copies — the 2026-06 double ingestion) is
/// still ONE call, and a call with no `tool_use_id` is unpaired rather than
/// guessed at.
///
/// Mutations that must break this: drop the per-id de-duplication, or fall
/// back to order pairing when the id is missing.
#[tokio::test]
async fn a_duplicated_capture_is_one_call_and_a_missing_id_is_unpaired() {
    let pg = PgStore::test_store().await;
    let session = format!("_test-replay-dup-{}", uuid::Uuid::new_v4());
    let id = format!("toolu_dup_{session}");
    let pre = serde_json::json!({ "tool_use_id": id, "tool_input": {} });
    let post = serde_json::json!({ "tool_use_id": id, "duration_ms": 5, "tool_response": {} });
    seed(
        &pg,
        &[
            event(&session, "PreToolUse", 2_000, pre.clone()),
            event(&session, "PreToolUse", 2_000, pre),
            event(&session, "PostToolUse", 2_010, post.clone()),
            event(&session, "PostToolUse", 2_010, post),
            // No id on either side: nothing says these two belong together.
            event(&session, "PreToolUse", 3_000, serde_json::json!({ "tool_input": {} })),
            event(&session, "PostToolUse", 3_050, serde_json::json!({ "tool_response": {} })),
        ],
    )
    .await;

    let calls = pg.get_session_tool_calls(&session, 10).await.unwrap();
    assert_eq!(calls.len(), 2, "one call per id, plus the id-less request: {calls:#?}");
    let paired: Vec<_> = calls.iter().filter(|c| c["toolUseId"] == id).collect();
    assert_eq!(paired.len(), 1, "two identical captures are one call");
    assert_eq!(paired[0]["durationMs"], 5);
    let idless = calls.iter().find(|c| c["toolUseId"].is_null()).expect("the id-less request");
    assert!(idless["response"].is_null(), "no id, no pairing — never a guess");
    assert!(idless["durationMs"].is_null());
}

/// The Replay timeline hangs each usage verdict on the call it belongs to. It
/// used to re-pair calls by order in its own SQL, so a verdict recorded against
/// B's response showed on A's row whenever B overtook A.
///
/// Mutation that must break this: join verdicts on any id but the view's
/// `response_id`.
#[tokio::test]
async fn a_verdict_shows_on_its_own_call_in_the_replay_timeline() {
    let pg = PgStore::test_store().await;
    let session = format!("_test-replay-verdict-{}", uuid::Uuid::new_v4());
    let (a, b) = (format!("toolu_va_{session}"), format!("toolu_vb_{session}"));
    seed(
        &pg,
        &[
            event(&session, "PreToolUse", 1_000, serde_json::json!({ "tool_use_id": a })),
            event(&session, "PreToolUse", 1_010, serde_json::json!({ "tool_use_id": b })),
            event(
                &session,
                "PostToolUse",
                1_030,
                serde_json::json!({ "tool_use_id": b, "duration_ms": 9 }),
            ),
            event(
                &session,
                "PostToolUse",
                1_900,
                serde_json::json!({ "tool_use_id": a, "duration_ms": 800 }),
            ),
        ],
    )
    .await;
    let (b_post,): (i64,) = sqlx_core::query_as::query_as(
        "SELECT id FROM activity.assistant_events
          WHERE session_id = $1 AND event_type = 'PostToolUse' AND payload->>'tool_use_id' = $2",
    )
    .bind(&session)
    .bind(&b)
    .fetch_one(pg.pool())
    .await
    .unwrap();
    sqlx_core::query::query(
        "INSERT INTO sensei.tool_call_verdicts (session_id, event_id, tool_name, verdict) VALUES ($1, $2, 'Bash', 'ignored')",
    )
    .bind(&session)
    .bind(b_post)
    .execute(pg.pool())
    .await
    .unwrap();

    let rows = pg.get_session_replay_timeline(&session, 10).await.unwrap();
    assert_eq!(rows.len(), 2, "{rows:#?}");
    let first = &rows[0];
    let second = &rows[1];
    assert_eq!(first["request"]["tool_use_id"], a.as_str(), "ordered by start: A first");
    assert!(first["verdict"].is_null(), "A's row carries no verdict: {first}");
    assert_eq!(first["durationMs"], 800);
    assert_eq!(second["postEventId"], b_post);
    assert_eq!(second["verdict"], "ignored", "B's verdict is on B's row");

    sqlx_core::query::query("DELETE FROM sensei.tool_call_verdicts WHERE session_id = $1")
        .bind(&session)
        .execute(pg.pool())
        .await
        .ok();
}

/// A failed call is paired with its FAILURE, not left in flight forever. Claude
/// Code fires `PostToolUseFailure` instead of `PostToolUse` when a call fails, so
/// a timeline that only pairs PostToolUse showed every failure as still running.
///
/// Mutation that must break this: pair only `event_type = 'PostToolUse'`.
#[tokio::test]
async fn a_failed_call_is_paired_with_its_failure() {
    let pg = PgStore::test_store().await;
    let session = format!("_test-replay-fail-{}", uuid::Uuid::new_v4());
    let id = format!("toolu_fail_{session}");
    let rows = [
        (session.clone(), "PreToolUse", 4_000_i64, serde_json::json!({ "tool_use_id": id })),
        (
            session.clone(),
            "PostToolUseFailure",
            4_020_i64,
            serde_json::json!({ "tool_use_id": id, "error": "Exit code 1" }),
        ),
    ];
    for (s, kind, ts, payload) in &rows {
        pg.insert_hook_event(&HookEventRow {
            session_id: s,
            assistant_family: "claude",
            event_type: kind,
            tool_name: Some("Bash"),
            cwd: None,
            ts: *ts,
            success: if *kind == "PostToolUseFailure" { Some(false) } else { None },
            payload,
        })
        .await
        .unwrap();
    }

    let calls = pg.get_session_tool_calls(&session, 10).await.unwrap();
    assert_eq!(calls.len(), 1, "{calls:#?}");
    let c = &calls[0];
    assert_eq!(c["inFlight"], false, "a failed call has finished: {c}");
    assert_eq!(c["success"], false);
    assert_eq!(c["response"]["error"], "Exit code 1");
}

/// A duration the engine did not send as a number reads as unknown. It used to
/// be cast in the view (`::numeric`), so one malformed payload failed every
/// read of the timeline — and of the workspace tool stats (#298).
///
/// Mutation that must break this: derive `duration_ms` with a bare cast again.
#[tokio::test]
async fn a_malformed_duration_reads_as_unknown_and_never_breaks_the_timeline() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let session = format!("_test-replay-baddur-{}", uuid::Uuid::new_v4());
    let id = format!("toolu_baddur_{session}");
    seed(
        &pg,
        &[
            event(&session, "PreToolUse", 4_000, serde_json::json!({ "tool_use_id": id })),
            event(
                &session,
                "PostToolUse",
                4_020,
                serde_json::json!({ "tool_use_id": id, "duration_ms": "n/a" }),
            ),
        ],
    )
    .await;

    let calls = pg.get_session_tool_calls(&session, 10).await.expect("the timeline still reads");
    assert_eq!(calls.len(), 1, "{calls:#?}");
    assert_eq!(calls[0]["toolUseId"], id.as_str(), "still paired");
    assert!(calls[0]["durationMs"].is_null(), "unknown, not an error");
}

/// The view pairs on keys stored when the event is written, never on keys
/// pulled out of `payload` at read time (#298). A PostToolUse payload carries
/// the whole tool response; extracting `tool_use_id` from every one of them is
/// what made workspace Instruments take 11–28s to open.
///
/// Mutation that must break this: read `payload->>'tool_use_id'` or
/// `payload->>'duration_ms'` in the view again, or drop the stored columns.
#[tokio::test]
async fn the_timeline_pairs_on_stored_keys_not_on_the_payload() {
    let Ok(pg) = PgStore::connect_test().await else {
        return;
    };
    let stored: Vec<(String,)> = sqlx_core::query_as::query_as(
        "select attname::text from pg_attribute
          where attrelid = 'activity.assistant_events'::regclass
            and attname in ('tool_use_id', 'duration_ms') and attgenerated = 's'
          order by 1",
    )
    .fetch_all(&pg.pool)
    .await
    .unwrap();
    assert_eq!(
        stored.iter().map(|(n,)| n.as_str()).collect::<Vec<_>>(),
        ["duration_ms", "tool_use_id"],
        "both keys are stored generated columns"
    );

    let (def,): (String,) = sqlx_core::query_as::query_as(
        "select pg_get_viewdef('sensei.session_tool_calls'::regclass)",
    )
    .fetch_one(&pg.pool)
    .await
    .unwrap();
    for key in ["'tool_use_id'", "'duration_ms'"] {
        assert!(!def.contains(key), "the view reads {key} out of the payload:\n{def}");
    }
}
