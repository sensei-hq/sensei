//! Session API cost (docs/spec/daemon/session-api-cost.md C2–C4): one row per API
//! message, priced at read from the dated `sensei.model_prices`.
use super::*;
use crate::transcript::TranscriptMessage;

/// A model name only this test uses, so seeded prices never collide. The whole
/// uuid, never a slice: an 8-character slice that happens to be all digits reads
/// as a `-YYYYMMDD` date suffix, which pricing strips.
fn model(tag: &str) -> String {
    format!("_test-{tag}-{}", uuid::Uuid::new_v4().simple())
}

/// A price row: `[input, cache_write_5m, cache_write_1h, cache_read, output]` USD/MTok.
async fn price(
    s: &PgStore,
    model: &str,
    from: &str,
    to: Option<&str>,
    prompt_over: i32,
    us: f64,
    r: [f64; 5],
) {
    sqlx_core::query::query(
        "INSERT INTO sensei.model_prices
             (model, speed, prompt_over, effective_from, effective_to,
              input, cache_write_5m, cache_write_1h, cache_read, output, us_multiplier,
              source, retrieved_on)
         VALUES ($1, 'standard', $2, $3::date, $4::date, $5, $6, $7, $8, $9, $10, 'test', '2026-10-23')",
    )
    .bind(model)
    .bind(prompt_over)
    .bind(from)
    .bind(to)
    .bind(r[0])
    .bind(r[1])
    .bind(r[2])
    .bind(r[3])
    .bind(r[4])
    .bind(us)
    .execute(s.pool())
    .await
    .unwrap();
}

async fn drop_prices(s: &PgStore, models: &[&str]) {
    sqlx_core::query::query("DELETE FROM sensei.model_prices WHERE model = ANY($1)")
        .bind(models.iter().map(|m| m.to_string()).collect::<Vec<_>>())
        .execute(s.pool())
        .await
        .unwrap();
}

/// A message at `at` (RFC 3339) with `[input, cw5m, cw1h, cache_read, output]`.
fn msg(id: &str, model: &str, at: &str, t: [i64; 5]) -> TranscriptMessage {
    TranscriptMessage {
        message_id: id.into(),
        turn_index: Some(1),
        at: Some(chrono::DateTime::parse_from_rfc3339(at).unwrap().timestamp_millis()),
        model: model.into(),
        speed: Some("standard".into()),
        inference_geo: Some("global".into()),
        service_tier: Some("standard".into()),
        input: t[0],
        cache_write_5m: t[1],
        cache_write_1h: t[2],
        cache_split_known: true,
        cache_read: t[3],
        output: t[4],
        thinking: None,
    }
}

/// `(message_id, cost)` for a session, by message id.
async fn costs(s: &PgStore, sid: &str) -> Vec<(String, Option<f64>)> {
    sqlx_core::query_as::query_as(
        "SELECT message_id, cost::float8 FROM activity.message_api_cost
          WHERE session_id = $1 ORDER BY message_id",
    )
    .bind(sid)
    .fetch_all(s.pool())
    .await
    .unwrap()
}

/// `(api_cost, priced, unpriced, hook_only_turns)` for a session.
async fn session_cost(s: &PgStore, sid: &str) -> Option<(Option<f64>, i64, i64, i64)> {
    sqlx_core::query_as::query_as(
        "SELECT api_cost::float8, priced, unpriced, hook_only_turns
           FROM activity.session_api_cost WHERE session_id = $1",
    )
    .bind(sid)
    .fetch_optional(s.pool())
    .await
    .unwrap()
}

async fn clear(s: &PgStore, sid: &str) {
    for sql in [
        "DELETE FROM activity.transcript_messages WHERE session_id = $1",
        "DELETE FROM activity.assistant_events WHERE session_id = $1",
    ] {
        sqlx_core::query::query(sql).bind(sid).execute(s.pool()).await.unwrap();
    }
}

fn close(a: Option<f64>, b: f64) -> bool {
    a.is_some_and(|a| (a - b).abs() < 1e-9)
}

/// Done gate 4: each category at its own rate, per million tokens.
#[tokio::test]
async fn a_message_costs_each_category_at_its_rate() {
    let s = pg_store().await;
    let m = model("opus");
    price(&s, &m, "2026-01-01", None, 0, 1.0, [4.0, 5.0, 8.0, 0.20, 20.0]).await;
    let sid = format!("cost-{}", uuid::Uuid::new_v4());
    s.upsert_transcript_messages(
        "claude_code",
        &sid,
        &[msg("a", &m, "2026-10-20T10:00:00Z", [2, 0, 33826, 26784, 313])],
    )
    .await
    .unwrap();
    let got = costs(&s, &sid).await;
    let want = (2.0 * 4.0 + 33826.0 * 8.0 + 26784.0 * 0.20 + 313.0 * 20.0) / 1e6;
    assert!(close(got[0].1, want), "{got:?} != {want}");
    assert!((want - 0.2822328).abs() < 1e-9);
    clear(&s, &sid).await;
    drop_prices(&s, &[&m]).await;
}

/// The writer replaces a session's rows, so a re-read never duplicates them.
#[tokio::test]
async fn re_ingesting_a_session_replaces_its_messages() {
    let s = pg_store().await;
    let m = model("opus");
    let sid = format!("cost-{}", uuid::Uuid::new_v4());
    let first = [
        msg("a", &m, "2026-10-20T10:00:00Z", [1, 0, 0, 0, 1]),
        msg("b", &m, "2026-10-20T10:01:00Z", [1, 0, 0, 0, 1]),
    ];
    s.upsert_transcript_messages("claude_code", &sid, &first).await.unwrap();
    s.upsert_transcript_messages("claude_code", &sid, &first[..1]).await.unwrap();
    let ids: Vec<String> = costs(&s, &sid).await.into_iter().map(|(id, _)| id).collect();
    assert_eq!(ids, vec!["a".to_string()]);
    clear(&s, &sid).await;
}

/// Done gate 5: a prompt over a band's threshold (input + cache) pays that band;
/// US-only processing pays the row's multiplier.
#[tokio::test]
async fn the_prompt_band_and_the_us_multiplier_apply_per_message() {
    let s = pg_store().await;
    let m = model("haiku");
    price(&s, &m, "2026-01-01", None, 0, 1.1, [0.10, 0.125, 0.20, 0.01, 0.50]).await;
    price(&s, &m, "2026-01-01", None, 100_000, 1.1, [0.50, 0.625, 1.0, 0.05, 2.50]).await;
    let sid = format!("cost-{}", uuid::Uuid::new_v4());
    let mut us = msg("c-us", &m, "2026-10-20T10:02:00Z", [1_000_000, 0, 0, 0, 0]);
    us.inference_geo = Some("us".into());
    s.upsert_transcript_messages(
        "claude_code",
        &sid,
        &[
            msg("a-small", &m, "2026-10-20T10:00:00Z", [10_000, 0, 0, 80_000, 1_000_000]),
            msg("b-large", &m, "2026-10-20T10:01:00Z", [10_000, 0, 0, 110_000, 1_000_000]),
            us,
        ],
    )
    .await
    .unwrap();
    let got = costs(&s, &sid).await;
    assert!(
        close(got[0].1, (10_000.0 * 0.10 + 80_000.0 * 0.01) / 1e6 + 0.50),
        "90k prompt: base band {got:?}"
    );
    assert!(
        close(got[1].1, (10_000.0 * 0.50 + 110_000.0 * 0.05) / 1e6 + 2.50),
        "120k prompt: over-100k band {got:?}"
    );
    assert!(close(got[2].1, 1.0 * 0.50 * 1.1), "1M input is over 100k too, at 1.1x {got:?}");
    clear(&s, &sid).await;
    drop_prices(&s, &[&m]).await;
}

/// Done gate 7: a message is priced by the row whose period holds it, and a
/// model id with a date suffix matches its undated price.
#[tokio::test]
async fn a_message_keeps_the_price_of_its_day_and_a_dated_id_matches() {
    let s = pg_store().await;
    let m = model("sonnet");
    price(&s, &m, "2026-01-01", Some("2026-09-01"), 0, 1.0, [3.0, 3.75, 6.0, 0.30, 15.0]).await;
    price(&s, &m, "2026-09-01", None, 0, 1.0, [2.0, 2.50, 4.0, 0.20, 10.0]).await;
    let sid = format!("cost-{}", uuid::Uuid::new_v4());
    s.upsert_transcript_messages(
        "claude_code",
        &sid,
        &[
            msg("a-old", &format!("{m}-20250929"), "2026-08-31T23:00:00Z", [0, 0, 0, 0, 1_000_000]),
            msg("b-new", &m, "2026-09-01T00:00:00Z", [0, 0, 0, 0, 1_000_000]),
        ],
    )
    .await
    .unwrap();
    let got = costs(&s, &sid).await;
    assert!(close(got[0].1, 15.0) && close(got[1].1, 10.0), "{got:?}");
    clear(&s, &sid).await;
    drop_prices(&s, &[&m]).await;
}

/// Done gate 6 + C4: no price row is no cost (never $0); the session reports
/// what it priced, what it could not, and the hook-only (subagent) turns.
#[tokio::test]
async fn a_session_reports_its_priced_part_and_what_it_could_not_price() {
    let s = pg_store().await;
    let m = model("opus");
    price(&s, &m, "2026-01-01", None, 0, 1.0, [4.0, 5.0, 8.0, 0.20, 20.0]).await;
    let sid = format!("cost-{}", uuid::Uuid::new_v4());
    let mut fast = msg("c-fast", &m, "2026-10-20T10:02:00Z", [0, 0, 0, 0, 1_000_000]);
    fast.speed = Some("fast".into());
    s.upsert_transcript_messages(
        "claude_code",
        &sid,
        &[
            msg("a", &m, "2026-10-20T10:00:00Z", [0, 0, 0, 0, 1_000_000]),
            msg("b-unknown", &model("nobody"), "2026-10-20T10:01:00Z", [0, 0, 0, 0, 1_000_000]),
            fast,
        ],
    )
    .await
    .unwrap();
    sqlx_core::query::query(
        "INSERT INTO activity.assistant_events (session_id, event_type, ts, payload)
         VALUES ($1, 'TurnComplete', 1792500000000, $2)",
    )
    .bind(&sid)
    .bind(serde_json::json!({"hook_event_name": "TurnComplete", "turn_id": "t9", "agent_id": "agent-x",
        "usage": {"model": m, "input_tokens": 1, "cache_read_input_tokens": 0,
                  "cache_creation_input_tokens": 0, "output_tokens": 1}}))
    .execute(s.pool())
    .await
    .unwrap();

    let got = costs(&s, &sid).await;
    assert_eq!(got[1].1, None, "unknown model: no cost, never $0");
    assert_eq!(got[2].1, None, "fast with no fast price: no cost");
    let (cost, priced, unpriced, hook_only) = session_cost(&s, &sid).await.unwrap();
    assert!(close(cost, 20.0), "{cost:?}");
    assert_eq!((priced, unpriced, hook_only), (1, 2, 1));
    assert_eq!(session_cost(&s, "no-such-session").await, None, "no messages: no row");
    clear(&s, &sid).await;
    drop_prices(&s, &[&m]).await;
}

/// C6: the one-time re-read clears the Claude watermarks once, so every Claude
/// transcript is parsed again under C1; a second call is a no-op.
#[tokio::test]
async fn the_message_rewrite_resets_claude_watermarks_once() {
    let s = pg_store().await;
    sqlx_core::query::query("DELETE FROM sensei.config WHERE key = 'transcripts.messages_v1'")
        .execute(s.pool())
        .await
        .unwrap();
    let file = format!("/_test/{}.jsonl", uuid::Uuid::new_v4());
    s.set_capture_watermark("claude_code", &file, None, 1, 0).await.unwrap();
    assert!(s.reset_claude_watermarks_once().await.unwrap(), "first call resets");
    assert_eq!(s.get_capture_watermark("claude_code", &file).await.unwrap(), None);
    s.set_capture_watermark("claude_code", &file, None, 2, 0).await.unwrap();
    assert!(!s.reset_claude_watermarks_once().await.unwrap(), "second call is a no-op");
    assert_eq!(s.get_capture_watermark("claude_code", &file).await.unwrap(), Some(2));
    sqlx_core::query::query("DELETE FROM activity.capture_watermarks WHERE file_path = $1")
        .bind(&file)
        .execute(s.pool())
        .await
        .unwrap();
}
