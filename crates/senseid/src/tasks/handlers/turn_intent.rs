//! The intent pass (#447): label user turns as correction / principle / neither
//! with the measured classifier, so corrections, first-try-right and outcomes
//! read intent rather than key phrases.
//!
//! Runs in the analyzer's daily window, in batches, and stops before its
//! watchdog; while candidates remain it queues itself again, which is how the
//! one-time backfill of history drains. A turn the model gives no answer for is
//! labelled by the phrase list with source `phrase` — never silently — and a later
//! run relabels it. When the model is down altogether, the run phrase-labels one
//! batch and stops: the next daily run tries the model again.

use super::super::Task;
use super::super::executor::TaskContext;
use super::analyze::{correction_signal, principle_signal};
use super::prompt_classify::{
    IntentCandidate, IntentModel, IntentSource, PromptClass, TurnIntent, classify_intent,
};
use crate::db::pg_store::{IntentCandidateRow, PgStore};

/// Candidates per batch: one stage-1 call.
const BATCH: i64 = 10;

/// The phrase list's label for a prompt — the fallback when the model gave none.
pub fn phrase_label(prompt: &str) -> PromptClass {
    if correction_signal(prompt).is_some() {
        PromptClass::Correction
    } else if principle_signal(prompt).is_some() {
        PromptClass::Principle
    } else {
        PromptClass::Neither
    }
}

/// A captured prompt's class for the aggregate signals (correction-prone,
/// rule-candidates, recurring corrections): its stored label when the intent pass
/// reached it, else the phrase list's — where a prompt that answers no reply
/// (`answers_reply == Some(false)`) is never a correction.
pub fn prompt_class(
    prompt: &str,
    label: Option<TurnIntent>,
    answers_reply: Option<bool>,
) -> PromptClass {
    if let Some(l) = label {
        return l.class;
    }
    match phrase_label(prompt) {
        PromptClass::Correction if answers_reply == Some(false) => PromptClass::Neither,
        class => class,
    }
}

/// The labels to store for one batch: the model's where it answered, the phrase
/// list's (marked `phrase`) where it did not. `decided[i]` belongs to `batch[i]`.
pub fn labels_for(
    batch: &[IntentCandidateRow],
    decided: &[Option<PromptClass>],
) -> Vec<(uuid::Uuid, TurnIntent)> {
    batch
        .iter()
        .zip(decided)
        .map(|(c, d)| {
            let label = match d {
                Some(class) => TurnIntent { class: *class, source: IntentSource::Intent },
                None => {
                    TurnIntent { class: phrase_label(&c.user_text), source: IntentSource::Phrase }
                }
            };
            (c.id, label)
        })
        .collect()
}

/// What one run did.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct PassReport {
    /// Labels written (changed rows).
    pub written: u64,
    /// Turns the model decided.
    pub by_model: u32,
    /// Turns labelled by the phrase fallback.
    pub by_phrase: u32,
    /// Candidates remain and the model is answering: run again.
    pub more: bool,
}

/// Label pending turns in batches until none remain, the model stops answering,
/// or `out_of_time` says stop.
pub async fn label_pending<M: IntentModel>(
    pg: &PgStore,
    model: &M,
    out_of_time: impl Fn() -> bool,
) -> Result<PassReport, String> {
    let mut report = PassReport::default();
    // Turns this run has labelled: a phrase-labelled turn is still a candidate,
    // and must not come round again in the same run.
    let mut handled: Vec<uuid::Uuid> = Vec::new();
    loop {
        if out_of_time() {
            report.more = true;
            return Ok(report);
        }
        let batch = pg.intent_candidates(BATCH, &handled).await?;
        if batch.is_empty() {
            return Ok(report);
        }
        let items: Vec<IntentCandidate> = batch
            .iter()
            .map(|c| IntentCandidate { prompt: &c.user_text, prev_reply: &c.prev_reply })
            .collect();
        let decided = classify_intent(model, &items).await;
        let answered = decided.iter().filter(|d| d.is_some()).count() as u32;
        report.by_model += answered;
        report.by_phrase += batch.len() as u32 - answered;
        report.written += pg.set_turn_intents(&labels_for(&batch, &decided)).await?;
        handled.extend(batch.iter().map(|c| c.id));
        if answered == 0 {
            tracing::warn!(
                "intent: the model gave no answer for a whole batch — phrase-labelled it; \
                 the next run tries the model again"
            );
            return Ok(report);
        }
    }
}

/// How long one run labels before it stops and, if needed, queues another —
/// well inside the task's watchdog budget.
const RUN_FOR: std::time::Duration = std::time::Duration::from_secs(15 * 60);

/// Handler for `TaskKind::ClassifyTurnIntent`.
pub async fn classify_turn_intent(ctx: &TaskContext, _task: &Task) -> Result<u32, String> {
    let model =
        super::prompt_classify::GatewayIntentModel { gateway: ctx.app_state.gateway.clone() };
    let started = std::time::Instant::now();
    let report = label_pending(ctx.pg(), &model, || started.elapsed() >= RUN_FOR).await?;
    tracing::info!(
        written = report.written,
        by_model = report.by_model,
        by_phrase = report.by_phrase,
        more = report.more,
        "classify_turn_intent"
    );
    if report.more {
        ctx.queue
            .enqueue_unique(Task::new(super::super::TaskKind::ClassifyTurnIntent, "", ""))
            .await;
    }
    Ok(report.written as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(prompt: &str) -> IntentCandidateRow {
        IntentCandidateRow {
            id: uuid::Uuid::new_v4(),
            session_id: "s".into(),
            user_text: prompt.into(),
            prev_reply: "I changed it.".into(),
        }
    }

    #[test]
    fn the_phrase_fallback_reads_corrections_then_principles() {
        assert_eq!(phrase_label("that's wrong, revert it"), PromptClass::Correction);
        assert_eq!(phrase_label("from now on, run the tests first"), PromptClass::Principle);
        assert_eq!(phrase_label("Hmm, I meant the other config file"), PromptClass::Neither);
    }

    #[test]
    fn a_prompt_class_is_its_label_else_its_phrase() {
        let correction =
            TurnIntent { class: PromptClass::Correction, source: IntentSource::Intent };
        let neither = TurnIntent { class: PromptClass::Neither, source: IntentSource::Intent };
        assert_eq!(
            prompt_class("Hmm, I meant the other config file", Some(correction), Some(true)),
            PromptClass::Correction
        );
        assert_eq!(
            prompt_class("add a revert button", Some(neither), Some(true)),
            PromptClass::Neither
        );
        assert_eq!(prompt_class("that's wrong, revert it", None, None), PromptClass::Correction);
        assert_eq!(
            prompt_class("that's wrong, revert it", None, Some(false)),
            PromptClass::Neither
        );
        assert_eq!(
            prompt_class("from now on, run the tests", None, Some(false)),
            PromptClass::Principle
        );
    }

    #[test]
    fn labels_take_the_model_and_mark_the_fallback() {
        let batch = [row("Hmm, I meant the other config file"), row("that's wrong, revert it")];
        let labels = labels_for(&batch, &[Some(PromptClass::Correction), None]);
        assert_eq!(
            labels,
            vec![
                (
                    batch[0].id,
                    TurnIntent { class: PromptClass::Correction, source: IntentSource::Intent }
                ),
                (
                    batch[1].id,
                    TurnIntent { class: PromptClass::Correction, source: IntentSource::Phrase }
                ),
            ]
        );
    }

    /// Answers every stage-1 batch with `labels` (sized to the batch), and every
    /// stage-2 check with `verdict`; `None` = the model is down.
    struct Fixed {
        label: Option<&'static str>,
        verdict: &'static str,
        calls: std::sync::atomic::AtomicU32,
    }

    impl IntentModel for Fixed {
        fn chat(
            &self,
            system: &str,
            user: String,
            _max_tokens: u32,
        ) -> impl std::future::Future<Output = Option<String>> + Send {
            self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            let answer = if system == super::super::prompt_classify::VERIFY_SYSTEM {
                Some(self.verdict.to_string())
            } else {
                self.label.map(|l| {
                    let n = user.lines().filter(|line| line.contains("DEVELOPER:")).count();
                    serde_json::to_string(&vec![l; n]).unwrap()
                })
            };
            async move { answer }
        }
    }

    async fn seed(pg: &PgStore, n: i32) -> String {
        let sid = format!("_test-pass-{}", uuid::Uuid::new_v4());
        let turns: Vec<crate::transcript::TranscriptTurn> = (0..=n)
            .map(|i| crate::transcript::TranscriptTurn {
                turn_index: i,
                user_text: Some(format!("step {i}: carry on with the parser")),
                assistant_text: format!("Did step {i}."),
                ..Default::default()
            })
            .collect();
        pg.upsert_transcript_turns(
            &crate::db::pg_store::TranscriptSource {
                source: "claude_code",
                session_id: &sid,
                family: "claude",
                provider: None,
                model: None,
            },
            &turns,
        )
        .await
        .unwrap();
        sid
    }

    async fn stored(pg: &PgStore, sid: &str) -> Vec<Option<TurnIntent>> {
        let (turns, _) = pg.get_transcript_turns_for_session(sid).await.unwrap();
        turns.into_iter().map(|t| t.intent).collect()
    }

    #[tokio::test]
    async fn a_run_labels_every_candidate_by_the_model() {
        let ctx = crate::tasks::test_support::make_ctx().await;
        let pg = ctx.pg();
        let sid = seed(pg, 12).await;
        let model = Fixed { label: Some("neither"), verdict: "no", calls: 0.into() };
        let report = label_pending(pg, &model, || false).await.unwrap();
        let labels = stored(pg, &sid).await;
        assert_eq!(labels[0], None, "the first turn answers no reply");
        let neither = TurnIntent { class: PromptClass::Neither, source: IntentSource::Intent };
        assert!(labels[1..].iter().all(|l| *l == Some(neither)), "{labels:?}");
        assert_eq!((report.by_model, report.by_phrase, report.written), (12, 0, 12));
        assert!(!report.more, "nothing left");
    }

    #[tokio::test]
    async fn with_the_model_down_a_run_marks_one_batch_by_phrase_and_stops() {
        let ctx = crate::tasks::test_support::make_ctx().await;
        let pg = ctx.pg();
        let sid = seed(pg, 25).await;
        let model = Fixed { label: None, verdict: "no", calls: 0.into() };
        let report = label_pending(pg, &model, || false).await.unwrap();
        assert_eq!(report.by_model, 0);
        assert_eq!(report.by_phrase, BATCH as u32, "one batch, then stop");
        assert!(!report.more, "no point running again until the model is back");
        assert_eq!(model.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
        let phrase = TurnIntent { class: PromptClass::Neither, source: IntentSource::Phrase };
        let labels = stored(pg, &sid).await;
        assert_eq!(labels.iter().filter(|l| **l == Some(phrase)).count(), BATCH as usize);
    }

    #[tokio::test]
    async fn a_run_out_of_time_says_there_is_more() {
        let ctx = crate::tasks::test_support::make_ctx().await;
        let pg = ctx.pg();
        let _sid = seed(pg, 25).await;
        let model = Fixed { label: Some("neither"), verdict: "no", calls: 0.into() };
        let report = label_pending(pg, &model, || true).await.unwrap();
        assert!(report.more, "candidates remain: queue another run");
        assert_eq!(report.written, 0);
    }
}

/// The measurement harness (#447 §4.1): the classifier exactly as the daemon runs
/// it — the seeded `intent` chain through the gateway, the committed prompts —
/// over a labelled set that lives OUTSIDE the repo (real prompts; the repo is
/// public). Ignored by default:
///   `INTENT_SET=<sample.json> INTENT_OUT=<labels.json> GATEWAY_LOADER_TEST_URL=<seeded db> \
///    cargo test -p senseid --bin senseid intent_labels_a_set -- --ignored`
/// The set is `[{ "id", "user", "prev" }]`; the output `{ id: label | null }`, scored
/// by the local harness against its gold labels.
#[cfg(test)]
mod measure {
    use super::super::prompt_classify::{GatewayIntentModel, IntentCandidate, classify_intent};

    /// The pass itself, end to end, for `INTENT_BATCHES` batches (default 2):
    /// candidates from the database, the seeded chain, labels written back. Run
    /// against the live database it writes exactly what the daemon would.
    ///   `GATEWAY_LOADER_TEST_URL=<db> cargo test -p senseid --bin senseid \
    ///    intent_pass_runs -- --ignored --nocapture`
    #[tokio::test]
    #[ignore]
    async fn intent_pass_runs() {
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(tracing::Level::WARN)
            .try_init();
        let url = std::env::var("GATEWAY_LOADER_TEST_URL").expect("GATEWAY_LOADER_TEST_URL");
        let pg = crate::db::pg_store::PgStore::connect(&url).await.expect("connect");
        let cfg = crate::api::gateway_config_loader::load_gateway_config(&pg)
            .await
            .expect("load")
            .expect("the database seeds chains");
        let gateway = std::sync::Arc::new(gateway::FacadeBuilder::new(cfg).build().await.gateway);
        let model = GatewayIntentModel { gateway };
        let batches: u32 =
            std::env::var("INTENT_BATCHES").ok().and_then(|b| b.parse().ok()).unwrap_or(2);
        let ticks = std::sync::atomic::AtomicU32::new(0);
        let report = super::label_pending(&pg, &model, || {
            ticks.fetch_add(1, std::sync::atomic::Ordering::SeqCst) >= batches
        })
        .await
        .expect("the pass runs");
        println!("intent pass: {report:?}");
        assert!(report.by_model > 0, "the model labelled nothing: {report:?}");
    }

    #[tokio::test]
    #[ignore]
    async fn intent_labels_a_set() {
        // The pass reports a failed call through tracing; show it here.
        let _ = tracing_subscriber::fmt()
            .with_test_writer()
            .with_max_level(tracing::Level::WARN)
            .try_init();
        let set: Vec<serde_json::Value> = serde_json::from_str(
            &std::fs::read_to_string(std::env::var("INTENT_SET").expect("INTENT_SET")).unwrap(),
        )
        .unwrap();
        let url = std::env::var("GATEWAY_LOADER_TEST_URL").expect("GATEWAY_LOADER_TEST_URL");
        let pg = crate::db::pg_store::PgStore::connect(&url).await.expect("connect");
        let cfg = crate::api::gateway_config_loader::load_gateway_config(&pg)
            .await
            .expect("load")
            .expect("the database seeds chains");
        assert!(cfg.chains.contains_key("intent"), "the intent chain is seeded");
        let gateway = std::sync::Arc::new(gateway::FacadeBuilder::new(cfg).build().await.gateway);
        let model = GatewayIntentModel { gateway };
        let items: Vec<IntentCandidate> = set
            .iter()
            .map(|it| IntentCandidate {
                prompt: it["user"].as_str().unwrap_or(""),
                prev_reply: it["prev"].as_str().unwrap_or(""),
            })
            .collect();
        let labels = classify_intent(&model, &items).await;
        let out: serde_json::Map<String, serde_json::Value> = set
            .iter()
            .zip(&labels)
            .map(|(it, l)| {
                let label = l.map_or(serde_json::Value::Null, |c| c.as_str().into());
                (it["id"].as_str().unwrap_or("").to_string(), label)
            })
            .collect();
        std::fs::write(
            std::env::var("INTENT_OUT").expect("INTENT_OUT"),
            serde_json::to_string_pretty(&out).unwrap(),
        )
        .unwrap();
        let unknown = labels.iter().filter(|l| l.is_none()).count();
        println!("intent: {} labelled, {unknown} with no answer", labels.len() - unknown);
    }
}
