//! #447: a user turn's intent label — stored on its transcript turn, read back with
//! it, cleared when the prompt itself changes, and written only when it changes.
use super::*;
use crate::tasks::handlers::prompt_classify::{IntentSource, PromptClass, TurnIntent};

const CORRECTION: TurnIntent =
    TurnIntent { class: PromptClass::Correction, source: IntentSource::Intent };
const PHRASE_NEITHER: TurnIntent =
    TurnIntent { class: PromptClass::Neither, source: IntentSource::Phrase };

fn source(sid: &str) -> TranscriptSource<'_> {
    TranscriptSource {
        source: "claude_code",
        session_id: sid,
        family: "claude",
        provider: None,
        model: None,
    }
}

fn turn(i: i32, origin: &str, user: &str, reply: &str) -> crate::transcript::TranscriptTurn {
    crate::transcript::TranscriptTurn {
        turn_index: i,
        origin: origin.to_string(),
        user_text: Some(user.to_string()),
        assistant_text: reply.to_string(),
        ..Default::default()
    }
}

async fn turn_id(s: &PgStore, sid: &str, i: i32) -> uuid::Uuid {
    let (id,): (uuid::Uuid,) = query_as(
        "SELECT id FROM activity.transcript_turns WHERE session_id = $1 AND turn_index = $2",
    )
    .bind(sid)
    .bind(i)
    .fetch_one(s.pool())
    .await
    .unwrap();
    id
}

async fn cleanup(s: &PgStore, sid: &str) {
    sqlx_core::query::query("DELETE FROM activity.transcript_turns WHERE session_id = $1")
        .bind(sid)
        .execute(s.pool())
        .await
        .unwrap();
}

#[tokio::test]
async fn a_label_reads_back_survives_a_reingest_and_clears_when_the_prompt_changes() {
    let s = pg_store().await;
    let sid = format!("_test-intent-{}", uuid::Uuid::new_v4());
    let turns = [
        turn(0, "human", "add a cache", "I added it."),
        turn(1, "human", "no, the other file", "Moved."),
    ];
    s.upsert_transcript_turns(&source(&sid), &turns).await.unwrap();
    let id = turn_id(&s, &sid, 1).await;

    assert_eq!(s.set_turn_intents(&[(id, CORRECTION)]).await.unwrap(), 1);
    let (read, _) = s.get_transcript_turns_for_session(&sid).await.unwrap();
    assert_eq!(read[0].intent, None);
    assert_eq!(read[1].intent, Some(CORRECTION));

    // The same prompt read again keeps its label.
    s.upsert_transcript_turns(&source(&sid), &turns).await.unwrap();
    let (read, _) = s.get_transcript_turns_for_session(&sid).await.unwrap();
    assert_eq!(read[1].intent, Some(CORRECTION), "a re-read of the same prompt keeps it");

    // A different prompt at that index is a different turn: the label goes.
    s.upsert_transcript_turns(&source(&sid), &[turn(1, "human", "now add tests", "Moved.")])
        .await
        .unwrap();
    let (read, _) = s.get_transcript_turns_for_session(&sid).await.unwrap();
    assert_eq!(read[1].intent, None, "a changed prompt clears its label");
    cleanup(&s, &sid).await;
}

#[tokio::test]
async fn set_turn_intents_writes_only_what_changed() {
    let s = pg_store().await;
    let sid = format!("_test-intent-{}", uuid::Uuid::new_v4());
    s.upsert_transcript_turns(
        &source(&sid),
        &[turn(0, "human", "a", "b"), turn(1, "human", "c", "d")],
    )
    .await
    .unwrap();
    let id = turn_id(&s, &sid, 1).await;
    assert_eq!(s.set_turn_intents(&[(id, PHRASE_NEITHER)]).await.unwrap(), 1);
    assert_eq!(
        s.set_turn_intents(&[(id, PHRASE_NEITHER)]).await.unwrap(),
        0,
        "unchanged: not rewritten"
    );
    assert_eq!(
        s.set_turn_intents(&[(id, CORRECTION)]).await.unwrap(),
        1,
        "the model relabels a phrase label"
    );
    let (stamped,): (bool,) = query_as(
        "SELECT intent_at IS NOT NULL AND intent_source = 'intent' FROM activity.transcript_turns WHERE id = $1",
    )
    .bind(id)
    .fetch_one(s.pool())
    .await
    .unwrap();
    assert!(stamped);
    cleanup(&s, &sid).await;
}

#[tokio::test]
async fn candidates_are_your_turns_that_answer_a_reply_and_lack_a_model_label() {
    let s = pg_store().await;
    let sid = format!("_test-intent-{}", uuid::Uuid::new_v4());
    let mut side = turn(8, "human", "a subagent's prompt", "ok");
    side.facts.is_sidechain = Some(true);
    let turns = [
        turn(0, "human", "start the parser", "I wrote parser.rs."), // first turn: answers nothing
        turn(1, "human", "no, the other file", "Moved it to lexer.rs."), // a candidate
        turn(2, "scheduled", "loop tick", "Checked."),              // not yours
        turn(3, "human", "<command-name>/clear</command-name>", "x"), // machine text
        turn(4, "human", "[Request interrupted by user]", "x"),     // machine text
        turn(5, "human", "ok", "x"),                                // under 3 chars
        turn(6, "human", "looks good, ship it", "Shipped."),        // labelled by the model
        turn(7, "human", "why did you drop the index?", "Dropped."), // phrase label only: relabel
        side,
    ];
    s.upsert_transcript_turns(&source(&sid), &turns).await.unwrap();
    let modelled = turn_id(&s, &sid, 6).await;
    let phrased = turn_id(&s, &sid, 7).await;
    s.set_turn_intents(&[
        (modelled, TurnIntent { class: PromptClass::Neither, source: IntentSource::Intent }),
        (phrased, TurnIntent { class: PromptClass::Correction, source: IntentSource::Phrase }),
    ])
    .await
    .unwrap();

    let mine: Vec<IntentCandidateRow> = s
        .intent_candidates(1_000_000, &[])
        .await
        .unwrap()
        .into_iter()
        .filter(|c| c.session_id == sid)
        .collect();
    let mut got: Vec<(&str, &str)> =
        mine.iter().map(|c| (c.user_text.as_str(), c.prev_reply.as_str())).collect();
    got.sort();
    assert_eq!(
        got,
        [("no, the other file", "I wrote parser.rs."), ("why did you drop the index?", "Shipped.")]
    );
    cleanup(&s, &sid).await;
}

#[tokio::test]
async fn candidates_leave_out_the_turns_a_run_has_already_handled() {
    let s = pg_store().await;
    let sid = format!("_test-intent-{}", uuid::Uuid::new_v4());
    let turns = [
        turn(0, "human", "start", "Started."),
        turn(1, "human", "go on then", "Went."),
        turn(2, "human", "and the tests", "Added."),
    ];
    s.upsert_transcript_turns(&source(&sid), &turns).await.unwrap();
    let seen = turn_id(&s, &sid, 2).await;
    let left: Vec<String> = s
        .intent_candidates(1_000_000, &[seen])
        .await
        .unwrap()
        .into_iter()
        .filter(|c| c.session_id == sid)
        .map(|c| c.user_text)
        .collect();
    assert_eq!(left, ["go on then"], "a turn handled this run is not offered again");
    cleanup(&s, &sid).await;
}
