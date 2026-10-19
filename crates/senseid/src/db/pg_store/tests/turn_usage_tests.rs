//! `activity.turn_usage` (#280, decided 2026-10-19): one row per turn, linking the
//! hook's `TurnComplete` usage to the transcript's turn so a turn both saw is
//! counted once, with the sources that saw it and whether they agreed.
use super::*;

/// A transcript turn: `(turn_index, started_at seconds, fresh, cache_read, cache_write, out)`.
async fn transcript_turn(s: &PgStore, sid: &str, t: (i32, i64, i64, i64, i64, i64)) {
    sqlx_core::query::query(
        "INSERT INTO activity.transcript_turns
             (source, session_id, family, turn_index, started_at,
              tokens_in, cache_read, cache_write, tokens_out, is_sidechain)
         VALUES ('claude_code', $1, 'claude', $2, to_timestamp($3), $4, $5, $6, $7, false)",
    )
    .bind(sid)
    .bind(t.0)
    .bind(t.1 as f64)
    .bind(t.2)
    .bind(t.3)
    .bind(t.4)
    .bind(t.5)
    .execute(s.pool())
    .await
    .unwrap();
}

/// A hook `TurnComplete` ending at `ended_s` seconds, for the main loop or an agent.
async fn hook_turn(
    s: &PgStore,
    sid: &str,
    turn_id: &str,
    agent: Option<&str>,
    ended_s: i64,
    u: [i64; 4],
) {
    let mut payload = serde_json::json!({
        "hook_event_name": "TurnComplete", "session_id": sid, "turn_id": turn_id,
        "usage": { "model": "claude-opus-5-5", "input_tokens": u[0],
                   "cache_read_input_tokens": u[1], "cache_creation_input_tokens": u[2],
                   "output_tokens": u[3] }
    });
    if let Some(a) = agent {
        payload["agent_id"] = serde_json::json!(a);
    }
    sqlx_core::query::query(
        "INSERT INTO activity.assistant_events (session_id, event_type, ts, payload)
         VALUES ($1, 'TurnComplete', $2, $3)",
    )
    .bind(sid)
    .bind(ended_s * 1000)
    .bind(payload)
    .execute(s.pool())
    .await
    .unwrap();
}

type Row = (Option<i32>, Option<String>, bool, bool, Option<bool>, i64, i64, i64, i64);

async fn rows(s: &PgStore, sid: &str) -> Vec<Row> {
    sqlx_core::query_as::query_as(
        "SELECT turn_index, agent_id, seen_by_hook, seen_by_transcript, agreed,
                fresh, cache_read, cache_write, output
           FROM activity.turn_usage WHERE session_id = $1
          ORDER BY turn_index NULLS LAST, agent_id",
    )
    .bind(sid)
    .fetch_all(s.pool())
    .await
    .unwrap()
}

async fn clear(s: &PgStore, sid: &str) {
    for t in ["activity.transcript_turns", "activity.assistant_events"] {
        sqlx_core::query::query(&format!("DELETE FROM {t} WHERE session_id = $1"))
            .bind(sid)
            .execute(s.pool())
            .await
            .unwrap();
    }
}

/// The live case (db0d67ab, 2026-10-18): one turn, seen by both, equal usage →
/// one row, counted once.
#[tokio::test]
async fn a_turn_both_sources_saw_is_one_agreed_row() {
    let s = pg_store().await;
    let sid = format!("tu-agree-{}", uuid::Uuid::new_v4());
    transcript_turn(&s, &sid, (1, 1_000, 2, 10_327, 55_095, 4)).await;
    hook_turn(&s, &sid, "t1", None, 1_005, [2, 10_327, 55_095, 4]).await;

    assert_eq!(
        rows(&s, &sid).await,
        vec![(Some(1), None, true, true, Some(true), 2, 10_327, 55_095, 4)]
    );
    clear(&s, &sid).await;
}

/// A hook turn belongs to the transcript turn whose span holds its end — not to
/// the next one — and a turn only one source saw is counted from that source.
#[tokio::test]
async fn hook_turns_link_by_the_span_that_holds_their_end() {
    let s = pg_store().await;
    let sid = format!("tu-span-{}", uuid::Uuid::new_v4());
    transcript_turn(&s, &sid, (1, 1_000, 1, 1, 1, 1)).await;
    transcript_turn(&s, &sid, (2, 2_000, 5, 5, 5, 5)).await;
    // Ends inside turn 2's span; turn 1 had no hook (the mod loaded mid-session).
    hook_turn(&s, &sid, "t2", None, 2_010, [5, 5, 5, 5]).await;

    assert_eq!(
        rows(&s, &sid).await,
        vec![
            (Some(1), None, false, true, None, 1, 1, 1, 1),
            (Some(2), None, true, true, Some(true), 5, 5, 5, 5),
        ]
    );
    clear(&s, &sid).await;
}

/// Subagent turns come from hooks alone (sidechain transcripts are not read); a
/// disagreement is flagged, never averaged, and counts the hook's figures.
#[tokio::test]
async fn a_subagent_turn_is_its_own_row_and_a_disagreement_is_flagged() {
    let s = pg_store().await;
    let sid = format!("tu-agent-{}", uuid::Uuid::new_v4());
    transcript_turn(&s, &sid, (1, 1_000, 10, 0, 0, 10)).await;
    hook_turn(&s, &sid, "t1", None, 1_050, [12, 0, 0, 10]).await;
    hook_turn(&s, &sid, "a1", Some("agent-x"), 1_020, [7, 0, 0, 3]).await;

    assert_eq!(
        rows(&s, &sid).await,
        vec![
            (Some(1), None, true, true, Some(false), 12, 0, 0, 10),
            (None, Some("agent-x".into()), true, false, None, 7, 0, 0, 3),
        ]
    );
    clear(&s, &sid).await;
}
