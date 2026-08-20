//! 語 · Insight-copy pipeline — mentor-voice card text.
//!
//! Human-facing insight strings are the mentor voice of the product. Static
//! templates hit their ceiling immediately ("40 tools dormant" reads as noise
//! the second time). This module routes structured `facts` through a small
//! local model (embedded gemma4 via the sensei gateway `insight-copy` chain)
//! and returns copy that reads like a mentor noticed something specific.
//!
//! Design: the model owns the *sentence*, the code owns the *action*. Every
//! call site carries a deterministic `fallback`; the model is optional. Copy
//! is persisted in `sensei.insight_copy` keyed on `(kind, facts_hash)` so the
//! wire path never blocks on inference in steady state — same facts reuse the
//! same copy until eviction.
//!
//! Wire vs. warm split (the load-bearing rule): the request path calls
//! [`copy_or_warm`], which does a cache read ONLY and, on a miss, returns the
//! caller's `fallback` immediately while a detached background task
//! ([`spawn_warm`] → [`generate_and_cache`]) generates and persists the model
//! copy for the *next* load. Inference never runs on a request's critical path
//! — `tokio::time::timeout` cannot preempt the in-process embedded (blocking)
//! model, so the only safe place to await it is off-wire.
//!
//! Pure helpers (`facts_hash`, `build_prompt`, `parse_and_validate`) are
//! unit-tested without a DB or gateway; [`generate_and_cache`] threads the
//! store + gateway and is graceful — it never errors, returning `Some(copy)`
//! only when the model produced valid copy that was cached.
//!
//! Spec: `docs/spec/pipeline/insight-copy.md`.

use crate::db::pg_store::PgStore;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// The sensei voice charter — fed verbatim as the system message on every
/// prompt so all generated copy shares one voice. (Spec: "Voice charter".)
const VOICE_CHARTER: &str = "You are Sensei, a quiet mentor watching a developer work with AI \
coding assistants. You notice things and say them once, plainly. \
You are specific about what you saw and what you would change. \
You never use marketing language, exclamations, emojis, or the \
word \"leverage.\" You never call the user \"you\" more than once \
per message. Sentence case. Lowercase \"sensei\" and \"ollama.\" \
Short — the reader is glancing, not reading.";

/// Runaway guard for one background generation. Generous by design: warming
/// runs OFF the wire (in [`generate_and_cache`], via [`spawn_warm`]), so this
/// bound is on no request's critical path. It exists only so a wedged model
/// can't pin a background task forever — NOT as a latency budget (a `tokio`
/// timeout cannot preempt the in-process embedded blocking model anyway).
const WARM_TIMEOUT_MS: u64 = 8_000;
/// After a transport/timeout failure, skip the model entirely for this long so
/// a down ollama costs no repeated warm attempts. A validation miss does NOT
/// count as a failure here — the model is up, it just returned copy we won't ship.
const FAIL_BACKOFF_MS: u64 = 60_000;
/// Token budget for one short JSON object (`{ "title", "detail" }`).
const MAX_TOKENS: u32 = 120;
/// Low temperature — voice consistency over variety.
const TEMPERATURE: f32 = 0.3;

/// Marketing words the mentor voice never uses. A single occurrence (case-
/// insensitive substring) rejects the copy → the caller falls back. `game-chang`
/// catches "game-changing" / "game-changer".
const BANNED_WORDS: &[&str] = &[
    "leverage", "seamless", "seamlessly", "effortless", "effortlessly",
    "robust", "powerful", "supercharge", "unlock", "game-chang",
];

/// Third-person references to the reader. The mentor speaks *to* the developer,
/// not *about* them — "the developer provides…" reads as a system report and
/// breaks the personal voice. Shared by `build_prompt` (as an instruction) and
/// `voice_ok` (as a guard) so the two never drift. Lowercase (matched against a
/// lowercased string).
const THIRD_PERSON_MARKERS: &[&str] = &["the developer", "the user"];

/// Module-level availability breaker. `0` = model believed up; otherwise the
/// `now_ms()` of the last transport/timeout failure. Reset to `0` on any
/// successful gateway response (a validation miss does NOT trip it — the model
/// is up, it just returned copy we won't ship).
static LAST_FAIL_MS: AtomicU64 = AtomicU64::new(0);

/// Which insight card this copy is for. `as_str` is the stable snake_case key
/// used in both the `facts_hash` and the `sensei.insight_copy.kind` column;
/// `task_line` is the per-kind `<task>` instruction for the prompt.
// Variants are wired to producers incrementally. The tool-health six + Today
// (HeroKoanMature + InsightRecurringPattern) + Learnings-triage memory kinds +
// SessionRetrospective (per-session narrative, off the analyzer enrichment tick)
// are LIVE — routed by real producers. The remaining kinds (pattern-promoted,
// drift, early-koan, insight-adopted/drift, ftr-lift/regression) still have no
// producer, so `#[allow(dead_code)]` sits on those individual variants until wired.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InsightKind {
    // Health signals
    ToolWarn,
    ToolOpportunity,
    ToolDormant,
    ToolWorkhorse,
    ToolsDormantSummary,
    ToolsWorkhorseSummary,
    // Learnings triage
    MemoryProposedAdopt,
    MemoryProposedReview,
    #[allow(dead_code)]
    PatternPromoted,
    #[allow(dead_code)]
    DriftDetected,
    // Today koan
    #[allow(dead_code)]
    HeroKoanEarly,
    HeroKoanMature,
    InsightRecurringPattern,
    #[allow(dead_code)]
    InsightAdopted,
    #[allow(dead_code)]
    InsightDrift,
    // Impact
    #[allow(dead_code)]
    FtrLift,
    #[allow(dead_code)]
    FtrRegression,
    // Sessions
    SessionRetrospective,
    // Metric drill-down — one "why this session moved this metric" line per
    // (session, metric), grounded in the session row + the metric's meaning.
    SessionMetricObservation,
    // Code graph
    CommunityDescription,
    // Project metrics narrative (the metrics screen)
    MetricNarrativeHeadline,
    MetricSignalInsight,
    // Per-datapoint metric explainer — one "why this day's value is what it is"
    // line per (project, metric, day) at DAILY grain. Generated at COMPUTE time
    // alongside the value and keyed on facts that INCLUDE the value, so an
    // unchanged value re-hits the cache (no model call) and a changed value
    // misses and regenerates.
    MetricDayExplainer,
}

impl InsightKind {
    /// Stable snake_case key — part of the `facts_hash` and the DB `kind`
    /// column. Never change an existing mapping (it would orphan cached rows).
    pub fn as_str(&self) -> &'static str {
        match self {
            InsightKind::ToolWarn => "tool_warn",
            InsightKind::ToolOpportunity => "tool_opportunity",
            InsightKind::ToolDormant => "tool_dormant",
            InsightKind::ToolWorkhorse => "tool_workhorse",
            InsightKind::ToolsDormantSummary => "tools_dormant_summary",
            InsightKind::ToolsWorkhorseSummary => "tools_workhorse_summary",
            InsightKind::MemoryProposedAdopt => "memory_proposed_adopt",
            InsightKind::MemoryProposedReview => "memory_proposed_review",
            InsightKind::PatternPromoted => "pattern_promoted",
            InsightKind::DriftDetected => "drift_detected",
            InsightKind::HeroKoanEarly => "hero_koan_early",
            InsightKind::HeroKoanMature => "hero_koan_mature",
            InsightKind::InsightRecurringPattern => "insight_recurring_pattern",
            InsightKind::InsightAdopted => "insight_adopted",
            InsightKind::InsightDrift => "insight_drift",
            InsightKind::FtrLift => "ftr_lift",
            InsightKind::FtrRegression => "ftr_regression",
            InsightKind::SessionRetrospective => "session_retrospective",
            InsightKind::SessionMetricObservation => "session_metric_observation",
            InsightKind::CommunityDescription => "community_description",
            InsightKind::MetricNarrativeHeadline => "metric_narrative_headline",
            InsightKind::MetricSignalInsight => "metric_signal_insight",
            InsightKind::MetricDayExplainer => "metric_day_explainer",
        }
    }

    /// The `<task>` instruction sentence for this kind. Opens with the same
    /// `Kind: <key>.` marker the spec shows so the model anchors on the card
    /// type, then states what to write and to include the specific facts.
    pub fn task_line(&self) -> &'static str {
        match self {
            InsightKind::ToolWarn =>
                "Kind: tool_warn. Warn the developer that one tool is failing often. Name the tool and its error rate. Suggest a fix or a replacement.",
            InsightKind::ToolOpportunity =>
                "Kind: tool_opportunity. Point out a tool the developer rarely uses but that fits their current work. Name it and say when to reach for it.",
            InsightKind::ToolDormant =>
                "Kind: tool_dormant. Tell the developer one specific tool has gone dormant. Include the tool name. Suggest what to do.",
            InsightKind::ToolWorkhorse =>
                "Kind: tool_workhorse. Note one tool the developer leans on the most. Name it and the call count. Keep it a quiet observation, not praise.",
            InsightKind::ToolsDormantSummary =>
                "Kind: tools_dormant_summary. Summarise how many tools have gone dormant. Give the count and suggest a prune or a review.",
            InsightKind::ToolsWorkhorseSummary =>
                "Kind: tools_workhorse_summary. Summarise which few tools carry most of the work. Give the count and what it implies.",
            InsightKind::MemoryProposedAdopt =>
                "Kind: memory_proposed_adopt. Describe a proposed memory worth adopting. Say what it captures and why to keep it.",
            InsightKind::MemoryProposedReview =>
                "Kind: memory_proposed_review. Describe a proposed memory that needs a human look before adoption. Say what is uncertain.",
            InsightKind::PatternPromoted =>
                "Kind: pattern_promoted. Note a recurring pattern that has been promoted to a convention. Name it and what it standardises.",
            InsightKind::DriftDetected =>
                "Kind: drift_detected. Point out where the code and its documentation have drifted apart. Name the symbol or doc and what no longer matches.",
            InsightKind::HeroKoanEarly =>
                "Kind: hero_koan_early. Write a short reflective line for a developer early in working with sensei. Ground it in one thing you observed.",
            InsightKind::HeroKoanMature =>
                "Kind: hero_koan_mature. Write a short reflective line grounded in a trend you observed. Write a complete sentence, not a phrase or a label.",
            InsightKind::InsightRecurringPattern =>
                "Kind: insight_recurring_pattern. Name the recurring pattern in the work — what repeats and what it points to. Be concrete about the repeated thing.",
            InsightKind::InsightAdopted =>
                "Kind: insight_adopted. Note that a suggested change has been adopted. Say what changed and the effect so far.",
            InsightKind::InsightDrift =>
                "Kind: insight_drift. Note that a past habit or convention has started to slip. Say what changed and what to watch.",
            InsightKind::FtrLift =>
                "Kind: ftr_lift. Report that first-try resolution improved. Give the before and after and what likely helped.",
            InsightKind::FtrRegression =>
                "Kind: ftr_regression. Report that first-try resolution dropped. Give the before and after and where to look.",
            InsightKind::SessionRetrospective =>
                "Kind: session_retrospective. Summarise what this coding session accomplished. The title is a short headline of the main work; the detail states the outcome and any corrections. Ground both in the facts. Write plainly, sentence case.",
            InsightKind::SessionMetricObservation =>
                "Kind: session_metric_observation. These are software-engineering signals about a code repository — its files, sessions, and tools — never business or customer metrics (e.g. \"churn\" here means code churn, not customers leaving). Read the given `meaning` field for what this metric measures. In one plain line, say what THIS coding session contributed to THIS metric, grounded strictly in the given facts (outcome, first-try, corrections, turns, task, summary) — never invent a number. The title is a 2-4 word label; the detail is the one-line observation.",
            InsightKind::CommunityDescription =>
                "Kind: community_description. In one plain sentence, say what this cluster of code is responsible for, grounded in the given hub symbols and their kinds. Name the shared responsibility; do not list every symbol. The title is a short 2-4 word name for the cluster; the detail is the sentence.",
            InsightKind::MetricNarrativeHeadline =>
                "Kind: metric_narrative_headline. These are software-engineering signals about a code repository — its files, sessions, and tools — never business, sales, or customer metrics (e.g. \"churn\" here means code churn, not customers leaving). Summarise how the project's signals moved this period, reading the facts as a whole. The title is one plain sentence naming how many signals moved and the overall direction; the detail is one sentence naming the most important shifts and what they suggest. Ground both strictly in the given facts — never invent a number.",
            InsightKind::MetricSignalInsight =>
                "Kind: metric_signal_insight. This is a software-engineering signal about a code repository — its files, sessions, and tools — never a business or customer metric (e.g. \"churn\" here means code churn, not customers leaving). Read the given `meaning` field for what it actually measures. In one or two plain sentences, say how this metric is trending for the project. When a `trend` fact is given, report its `assessment` (improving, worsening, or steady) as the overall trend over `trend.window` — this is the direction the chart shows; you may add the `recent` step as the latest move, but only if you mark it as recent, and never present that one step as the overall trend. When no `trend` is given, describe only the `recent` step and say it is recent. Use only the given numbers — never invent one, and never call the metric improving when the given `assessment` is worsening. The title is a 2-4 word label; the detail is the observation.",
            InsightKind::MetricDayExplainer =>
                "Kind: metric_day_explainer. This is a software-engineering signal about a code repository — its files, sessions, and tools — never a business or customer metric (e.g. \"churn\" here means code churn, not customers leaving). Read the given `meaning` field for what this metric measures. In one plain line, explain why THIS day's value is what it is, grounded strictly in the given numbers (value, prev_value, delta, and the day's session counts) and the `day` context — never invent a number, and never state a direction the given `delta` does not support. The title is a 2-4 word label; the detail is that one line.",
        }
    }
}

/// Char caps for the generated copy. Over-limit output is rejected (→ fallback)
/// rather than truncated, so a rendered card never shows an ellipsis.
#[derive(Debug, Clone, Copy)]
pub struct CopyLimits {
    pub title: usize,
    pub detail: usize,
}

impl Default for CopyLimits {
    fn default() -> Self {
        CopyLimits { title: 60, detail: 180 }
    }
}

/// The deterministic copy a call site must supply — used on cold start,
/// timeout, model failure, or when the model's output fails validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FallbackCopy {
    pub title: String,
    pub detail: String,
}

/// The visible `title` + `detail` for one insight card — the return type of
/// [`copy_or_warm`] and the `Some` payload of [`generate_and_cache`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InsightCopy {
    pub title: String,
    pub detail: String,
}

impl From<FallbackCopy> for InsightCopy {
    fn from(f: FallbackCopy) -> Self {
        InsightCopy { title: f.title, detail: f.detail }
    }
}

/// `sha256(kind.as_str() + canonical_json(facts))`, hex-encoded. Deterministic
/// and order-independent: two `facts` values differing only in key insertion
/// order hash identically (canonical JSON sorts object keys).
pub fn facts_hash(kind: InsightKind, facts: &serde_json::Value) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(kind.as_str().as_bytes());
    hasher.update(canonical_json(facts).as_bytes());
    hex::encode(hasher.finalize())
}

/// Serialize a JSON value with object keys sorted recursively (arrays keep
/// order). Explicit rather than relying on serde_json's default `BTreeMap`
/// Value, so the hash stays stable even if a workspace crate flips on the
/// `serde_json/preserve_order` feature via feature unification.
fn canonical_json(v: &serde_json::Value) -> String {
    let mut out = String::new();
    write_canonical(v, &mut out);
    out
}

fn write_canonical(v: &serde_json::Value, out: &mut String) {
    use serde_json::Value;
    match v {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            out.push('{');
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(k).unwrap_or_default());
                out.push(':');
                write_canonical(&map[*k], out);
            }
            out.push('}');
        }
        Value::Array(arr) => {
            out.push('[');
            for (i, e) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(e, out);
            }
            out.push(']');
        }
        // Scalars: serde_json's own rendering is already canonical.
        other => out.push_str(&serde_json::to_string(other).unwrap_or_default()),
    }
}

/// Build the `(system, user)` prompt pair for one card. System is the voice
/// charter; user carries the `<task>` / `<facts>` / `<limits>` / `<format>`
/// sections from the spec's "Prompt shape".
///
/// The `<limits>` block is deliberately explicit and model-actionable: gemma2
/// routinely blew the char budget or reached for a banned word under the old
/// soft "no marketing language" phrasing. The forbidden-words line is generated
/// FROM [`BANNED_WORDS`] so the guard and the prompt can never drift.
///
/// When `retry` is set (the second, corrective attempt in [`generate_and_cache`])
/// a short corrective instruction is appended telling the model exactly why the
/// previous reply was rejected.
fn build_prompt(kind: InsightKind, facts: &serde_json::Value, limits: CopyLimits, retry: bool) -> (String, String) {
    let facts_json = serde_json::to_string(facts).unwrap_or_else(|_| "{}".to_string());
    // DRY: the banned line is derived from BANNED_WORDS, the same list voice_ok
    // enforces. Never hand-maintain a second copy.
    let banned = BANNED_WORDS.join(", ");
    let mut user = format!(
        "<task>\n{task}\n</task>\n\n\
         <facts>\n{facts}\n</facts>\n\n\
         <limits>\n\
         title: at most {tmax} characters — one short line.\n\
         detail: at most {dmax} characters — two short sentences at most. Count the characters; do not exceed {dmax}.\n\
         Do not use any of these words: {banned}.\n\
         Do not refer to the reader in the third person ({third_person}). Write as a direct observation of what you saw.\n\
         Write \"sensei\" and \"ollama\" in lowercase. Do not mention \"Claude\". No exclamation marks. No emoji.\n\
         </limits>\n\n\
         <format>\nReturn JSON: {{ \"title\": \"...\", \"detail\": \"...\" }}\nNo prose. No preamble.\n</format>",
        task = kind.task_line(),
        facts = facts_json,
        tmax = limits.title,
        dmax = limits.detail,
        banned = banned,
        third_person = THIRD_PERSON_MARKERS.join(", "),
    );
    if retry {
        user.push_str(&format!(
            "\n\n<correction>\nYour previous reply was rejected for being too long or using a forbidden word. \
             Rewrite it: keep detail strictly under {dmax} characters and use plain words only.\n</correction>",
            dmax = limits.detail,
        ));
    }
    (VOICE_CHARTER.to_string(), user)
}

/// Parse the model reply into validated copy, or `None` (→ caller falls back).
/// Tolerates fences / prose by extracting the first `{ … }` (mirrors
/// `tasks::handlers::corrections_llm::parse_response`). Enforces non-empty
/// fields, the char limits, and the voice guard.
pub fn parse_and_validate(content: &str, limits: CopyLimits) -> Option<InsightCopy> {
    // Extract the first JSON object, tolerating surrounding fences/prose.
    let start = content.find('{')?;
    let end = content.rfind('}')?;
    if end <= start {
        return None;
    }
    let v: serde_json::Value = serde_json::from_str(&content[start..=end]).ok()?;
    let title = v.get("title").and_then(|t| t.as_str()).map(str::trim).filter(|s| !s.is_empty())?;
    let detail = v.get("detail").and_then(|t| t.as_str()).map(str::trim).filter(|s| !s.is_empty())?;

    // Char limits — over-limit is rejected, never truncated.
    if title.chars().count() > limits.title || detail.chars().count() > limits.detail {
        return None;
    }
    // Voice guard on both fields.
    if !voice_ok(title) || !voice_ok(detail) {
        return None;
    }
    Some(InsightCopy { title: title.to_string(), detail: detail.to_string() })
}

/// Voice charter guard for one string. Rejects exclamations, emoji, marketing
/// words, the assistant's name ("Claude"), and mis-cased "Ollama"/"Sensei"
/// (the mentor is lowercase "sensei"; "Sensei" is allowed only as the first
/// character of the string, e.g. sentence start).
fn voice_ok(s: &str) -> bool {
    if s.contains('!') {
        return false;
    }
    if s.chars().any(|c| {
        let u = c as u32;
        u >= 0x1F000 || (0x2600..=0x27BF).contains(&u)
    }) {
        return false;
    }
    let lower = s.to_lowercase();
    if BANNED_WORDS.iter().any(|b| lower.contains(b)) {
        return false;
    }
    // Third-person self-reference breaks the personal-mentor voice ("The
    // developer provides…" reads as a report about the reader, not to them).
    if THIRD_PERSON_MARKERS.iter().any(|m| lower.contains(m)) {
        return false;
    }
    if s.contains("Claude") || s.contains("Ollama") {
        return false;
    }
    // "Sensei" only permitted as the very first character of the string.
    if s.match_indices("Sensei").any(|(i, _)| i != 0) {
        return false;
    }
    true
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// Structured logger onto `public.logs` for the off-wire warm path, so a
/// developer can inspect *why* a card fell back (the module's `tracing` lines
/// alone don't reach the DB audit log). Mirrors the `task_logger` /
/// `watchdog_logger` construction in `api::server` — the shared, already-used
/// path, not a new framework.
fn warm_logger(store: &PgStore) -> sensei_logger::Logger {
    sensei_logger::Logger::new(
        sensei_logger::LogWriter::pg(store.pool().clone()),
        sensei_logger::LogLevel::Info,
        "daemon",
        "insight_copy",
    )
}

/// Outcome of one gateway attempt inside [`generate_and_cache`].
enum WarmAttempt {
    /// Valid copy plus the model id that produced it (for the cache write).
    Ok(InsightCopy, Option<String>),
    /// Model responded but the copy failed `parse_and_validate` — the model is
    /// up, so the breaker stays clear and a retry is worthwhile.
    Rejected,
    /// Transport / timeout / provider failure — trips the 60s breaker.
    Failed(String),
}

/// One call to the `insight-copy` chain + parse/validate. `retry` selects the
/// tightened corrective prompt (see [`build_prompt`]). Time-boxed by
/// [`WARM_TIMEOUT_MS`] purely as a runaway guard (this runs off the wire).
async fn call_once(
    gateway: &gateway::Gateway,
    kind: InsightKind,
    facts: &serde_json::Value,
    limits: CopyLimits,
    retry: bool,
) -> WarmAttempt {
    use gateway::types::capability::Capability;
    use gateway::types::request::*;

    let (system, user) = build_prompt(kind, facts, limits, retry);
    let request = InferenceRequest {
        capability: Capability::TextChat,
        model: None,
        router: None,
        chain: Some("insight-copy".into()),
        payload: Payload::Chat {
            messages: vec![Message::text(MessageRole::User, user)],
            system: Some(system),
            max_tokens: Some(MAX_TOKENS),
            temperature: Some(TEMPERATURE),
            tools: Vec::new(),
        },
        budget: None,
        auth: None,
        panel: None,
        consensus: None,
        allow_fallback: true,
        credentials: std::collections::HashMap::new(),
    };

    match tokio::time::timeout(Duration::from_millis(WARM_TIMEOUT_MS), gateway.execute(&request)).await {
        Ok(Ok(resp)) if resp.success => {
            match resp.content.as_deref().and_then(|c| parse_and_validate(c, limits)) {
                Some(copy) => WarmAttempt::Ok(copy, resp.model),
                None => WarmAttempt::Rejected,
            }
        }
        Ok(Ok(_resp)) => WarmAttempt::Failed("gateway returned failure".to_string()),
        Ok(Err(e)) => WarmAttempt::Failed(e.to_string()),
        Err(_elapsed) => WarmAttempt::Failed(format!("timed out after {WARM_TIMEOUT_MS}ms")),
    }
}

/// Generate + validate + persist copy for one card — the reusable core, run
/// OFF the wire (from [`spawn_warm`], or directly from a future eager/tick
/// task). Returns `Some(copy)` only when the model produced valid copy that was
/// cached; `None` on breaker back-off, gateway failure, or a validation miss
/// that survived the one corrective retry. Never errors.
///
/// Breaker semantics: a transport/timeout/provider failure trips the 60s
/// [`LAST_FAIL_MS`] back-off (a fully-down model stops causing warm attempts);
/// a validation miss does NOT trip it (the model is up). Two attempts max —
/// the second uses the tightened corrective prompt.
pub async fn generate_and_cache(
    store: &PgStore,
    gateway: &gateway::Gateway,
    kind: InsightKind,
    facts: &serde_json::Value,
    limits: CopyLimits,
) -> Option<InsightCopy> {
    let h = facts_hash(kind, facts);

    // Availability breaker — don't hammer a model that just failed.
    let last_fail = LAST_FAIL_MS.load(Ordering::Relaxed);
    if last_fail != 0 && now_ms().saturating_sub(last_fail) < FAIL_BACKOFF_MS {
        return None;
    }

    // Two attempts: fresh, then a tightened corrective prompt on a validation miss.
    for retry in [false, true] {
        match call_once(gateway, kind, facts, limits, retry).await {
            WarmAttempt::Ok(copy, model) => {
                LAST_FAIL_MS.store(0, Ordering::Relaxed);
                // Provider not exposed on the response; model id is.
                store
                    .upsert_insight_copy(kind.as_str(), &h, &copy.title, &copy.detail, None, model.as_deref())
                    .await;
                return Some(copy);
            }
            WarmAttempt::Rejected => {
                // Model reachable — clear any prior back-off; do not trip the breaker.
                LAST_FAIL_MS.store(0, Ordering::Relaxed);
                if retry {
                    // Second miss — give up (fallback already on screen).
                    warm_logger(store).warn(
                        "insight_copy: model reply failed validation after retry — copy not cached",
                        Some(serde_json::json!({ "kind": kind.as_str() })),
                    ).await;
                    tracing::warn!(kind = kind.as_str(), "insight_copy: model reply failed validation after retry — copy not cached");
                }
                // First miss — fall through to the corrective retry.
            }
            WarmAttempt::Failed(err) => {
                LAST_FAIL_MS.store(now_ms(), Ordering::Relaxed);
                warm_logger(store).warn(
                    "insight_copy: gateway error — 60s back-off, copy not cached",
                    Some(serde_json::json!({ "kind": kind.as_str(), "error": err })),
                ).await;
                tracing::debug!(error = %err, kind = kind.as_str(), "insight_copy: gateway error — 60s back-off, copy not cached");
                return None;
            }
        }
    }
    None
}

/// Wire-path cache read ONLY — no inference, instant. Computes the `facts_hash`
/// and reads `sensei.insight_copy` (which bumps `last_used_at`). `None` on miss.
pub async fn read_cached_copy(
    store: &PgStore,
    kind: InsightKind,
    facts: &serde_json::Value,
) -> Option<InsightCopy> {
    let h = facts_hash(kind, facts);
    store
        .get_insight_copy(kind.as_str(), &h)
        .await
        .map(|(title, detail)| InsightCopy { title, detail })
}

/// In-flight dedup set: a burst of identical cache-misses fires only ONE
/// background generation. Keyed on `facts_hash`.
fn inflight() -> &'static std::sync::Mutex<std::collections::HashSet<String>> {
    static INFLIGHT: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> =
        std::sync::OnceLock::new();
    INFLIGHT.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()))
}

/// Claim the in-flight slot for `h`. Returns `true` if this caller now owns it
/// (and must eventually [`release_inflight`]); `false` if another warm task
/// already holds it (the caller should NOT spawn a duplicate). A poisoned mutex
/// is recovered rather than panicking the daemon.
fn claim_inflight(h: &str) -> bool {
    let mut set = inflight().lock().unwrap_or_else(|e| e.into_inner());
    // HashSet::insert returns true iff the value was newly inserted.
    set.insert(h.to_string())
}

/// Release the in-flight slot for `h` once the background generation finishes.
fn release_inflight(h: &str) {
    inflight().lock().unwrap_or_else(|e| e.into_inner()).remove(h);
}

/// Fire-and-forget background generation for one card. No-op if the breaker is
/// tripped or another warm task already owns this `facts_hash`. Owned args so
/// the detached task can `move` them.
fn spawn_warm(
    store: PgStore,
    gateway: std::sync::Arc<gateway::Gateway>,
    kind: InsightKind,
    facts: serde_json::Value,
    limits: CopyLimits,
) {
    let h = facts_hash(kind, &facts);

    // Don't pile up doomed tasks while the model is in back-off.
    let last_fail = LAST_FAIL_MS.load(Ordering::Relaxed);
    if last_fail != 0 && now_ms().saturating_sub(last_fail) < FAIL_BACKOFF_MS {
        return;
    }

    // Dedup: only the first caller for this hash spawns.
    if !claim_inflight(&h) {
        return;
    }

    tokio::spawn(async move {
        let _ = generate_and_cache(&store, &gateway, kind, &facts, limits).await;
        release_inflight(&h);
    });
}

/// Wire-path copy: cache hit → the persisted model copy; miss → `fallback`
/// returned immediately AND a detached background task is spawned to generate
/// + cache the copy for the next load. The wire NEVER blocks on inference.
pub async fn copy_or_warm(
    store: &PgStore,
    gateway: &std::sync::Arc<gateway::Gateway>,
    kind: InsightKind,
    facts: &serde_json::Value,
    limits: CopyLimits,
    fallback: FallbackCopy,
) -> InsightCopy {
    if let Some(c) = read_cached_copy(store, kind, facts).await {
        return c;
    }
    spawn_warm(store.clone(), gateway.clone(), kind, facts.clone(), limits);
    fallback.into()
}

/// Fire-and-forget warm for callers that read the cache themselves and render
/// their OWN deterministic copy on a miss (so they need no returned fallback).
/// The caller does `read_cached_copy` → on `None`, `warm(...)` to populate the
/// cache for the next load. No-op if the breaker is tripped or a warm for this
/// `facts_hash` is already in flight. Inference stays off the wire (detached).
pub fn warm(
    store: &PgStore,
    gateway: &std::sync::Arc<gateway::Gateway>,
    kind: InsightKind,
    facts: &serde_json::Value,
    limits: CopyLimits,
) {
    spawn_warm(store.clone(), gateway.clone(), kind, facts.clone(), limits);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn facts_hash_is_order_independent() {
        let a = json!({ "short": "get-callers", "days_since_last_use": 42, "total_calls": 3 });
        let b = json!({ "total_calls": 3, "days_since_last_use": 42, "short": "get-callers" });
        assert_eq!(
            facts_hash(InsightKind::ToolDormant, &a),
            facts_hash(InsightKind::ToolDormant, &b),
            "key insertion order must not change the hash"
        );
    }

    #[test]
    fn facts_hash_nested_order_independent() {
        let a = json!({ "tool": { "name": "search", "calls": 3 }, "days": 7 });
        let b = json!({ "days": 7, "tool": { "calls": 3, "name": "search" } });
        assert_eq!(facts_hash(InsightKind::ToolWarn, &a), facts_hash(InsightKind::ToolWarn, &b));
    }

    #[test]
    fn facts_hash_changes_with_facts() {
        let a = json!({ "short": "get-callers", "days_since_last_use": 42 });
        let b = json!({ "short": "get-callers", "days_since_last_use": 41 });
        assert_ne!(facts_hash(InsightKind::ToolDormant, &a), facts_hash(InsightKind::ToolDormant, &b));
    }

    #[test]
    fn facts_hash_changes_with_kind() {
        let f = json!({ "short": "get-callers" });
        assert_ne!(facts_hash(InsightKind::ToolDormant, &f), facts_hash(InsightKind::ToolWarn, &f));
    }

    #[test]
    fn parse_accepts_clean_json() {
        let c = r#"{"title":"get-callers has gone quiet","detail":"unused for 42 days after 3 calls; open it on the next graph question or drop it from the palette."}"#;
        let got = parse_and_validate(c, CopyLimits::default()).unwrap();
        assert_eq!(got.title, "get-callers has gone quiet");
        assert!(got.detail.starts_with("unused for 42 days"));
    }

    #[test]
    fn parse_tolerates_code_fences() {
        let c = "```json\n{\"title\":\"a quiet tool\",\"detail\":\"short and plain detail line\"}\n```";
        let got = parse_and_validate(c, CopyLimits::default()).unwrap();
        assert_eq!(got.title, "a quiet tool");
    }

    #[test]
    fn parse_rejects_exclamation() {
        let c = r#"{"title":"do it now!","detail":"a plain detail line"}"#;
        assert!(parse_and_validate(c, CopyLimits::default()).is_none());
    }

    #[test]
    fn parse_rejects_banned_word() {
        let c = r#"{"title":"leverage your dormant tools","detail":"a plain detail line"}"#;
        assert!(parse_and_validate(c, CopyLimits::default()).is_none());
    }

    #[test]
    fn parse_rejects_over_limit_title() {
        let long = "x".repeat(61);
        let c = format!(r#"{{"title":"{long}","detail":"a plain detail line"}}"#);
        assert!(parse_and_validate(&c, CopyLimits::default()).is_none());
    }

    #[test]
    fn parse_rejects_over_limit_detail() {
        let long = "y".repeat(181);
        let c = format!(r#"{{"title":"a quiet tool","detail":"{long}"}}"#);
        assert!(parse_and_validate(&c, CopyLimits::default()).is_none());
    }

    #[test]
    fn parse_rejects_claude() {
        let c = r#"{"title":"ask Claude to fix it","detail":"a plain detail line"}"#;
        assert!(parse_and_validate(c, CopyLimits::default()).is_none());
    }

    #[test]
    fn parse_rejects_ollama_word() {
        let c = r#"{"title":"a quiet tool","detail":"start Ollama and try the call again"}"#;
        assert!(parse_and_validate(c, CopyLimits::default()).is_none());
    }

    #[test]
    fn parse_rejects_empty_fields() {
        assert!(parse_and_validate(r#"{"title":"","detail":"x"}"#, CopyLimits::default()).is_none());
        assert!(parse_and_validate(r#"{"title":"x","detail":"  "}"#, CopyLimits::default()).is_none());
        assert!(parse_and_validate("not json", CopyLimits::default()).is_none());
    }

    #[test]
    fn parse_rejects_third_person_reference() {
        // The mentor speaks to the reader, not about them. (Persona review.)
        let c = r#"{"title":"a recurring pattern","detail":"The developer provides the same prompt each time."}"#;
        assert!(parse_and_validate(c, CopyLimits::default()).is_none(), "\"the developer\" rejected");
        let c2 = r#"{"title":"a recurring pattern","detail":"the user keeps correcting the same thing."}"#;
        assert!(parse_and_validate(c2, CopyLimits::default()).is_none(), "\"the user\" rejected");
        // Second-person / subject-less copy passes.
        let ok = r#"{"title":"a recurring pattern","detail":"the same prompt structure repeats across corrections."}"#;
        assert!(parse_and_validate(ok, CopyLimits::default()).is_some(), "direct observation passes");
    }

    #[test]
    fn parse_allows_sensei_only_at_start() {
        // Capital "Sensei" at the very start is fine (sentence start).
        let ok = r#"{"title":"Sensei noticed a quiet tool","detail":"a plain detail line"}"#;
        assert!(parse_and_validate(ok, CopyLimits::default()).is_some());
        // Mid-sentence "Sensei" is rejected.
        let bad = r#"{"title":"a quiet tool","detail":"let Sensei handle the review for you"}"#;
        assert!(parse_and_validate(bad, CopyLimits::default()).is_none());
    }

    #[test]
    fn as_str_stable_keys() {
        assert_eq!(InsightKind::ToolDormant.as_str(), "tool_dormant");
        assert_eq!(InsightKind::HeroKoanEarly.as_str(), "hero_koan_early");
        assert_eq!(InsightKind::FtrRegression.as_str(), "ftr_regression");
        // A NEW stable key — never reuse an existing one (a change orphans cache rows).
        assert_eq!(InsightKind::SessionMetricObservation.as_str(), "session_metric_observation");
        // The per-datapoint explainer's NEW stable key.
        assert_eq!(InsightKind::MetricDayExplainer.as_str(), "metric_day_explainer");
    }

    #[test]
    fn metric_day_explainer_task_line_is_grounded() {
        let t = InsightKind::MetricDayExplainer.task_line();
        assert!(t.starts_with("Kind: metric_day_explainer."), "anchors on the card key");
        // The line must steer the model to read the meaning, stay software-eng, explain
        // THIS day's value, and never invent a number — the per-datapoint grounding contract.
        assert!(t.contains("meaning"), "reads the metric's meaning");
        assert!(t.contains("software-engineering"), "software-eng signals, not business/customer");
        assert!(t.contains("never invent a number"), "no fabricated numbers");
        assert!(t.contains("THIS day's value"), "explains this day's value");
        assert!(t.contains("title is a 2-4 word label"), "title shape stated");
    }

    #[test]
    fn session_metric_observation_task_line_is_grounded() {
        let t = InsightKind::SessionMetricObservation.task_line();
        assert!(t.starts_with("Kind: session_metric_observation."), "anchors on the card key");
        // The line must steer the model to read the meaning, stay software-eng, and
        // not invent a number — the drill-down grounding contract.
        assert!(t.contains("meaning"), "reads the metric's meaning");
        assert!(t.contains("software-engineering"), "software-eng signals, not business/customer");
        assert!(t.contains("never invent a number"), "no fabricated numbers");
        assert!(t.contains("title is a 2-4 word label"), "title shape stated");
    }

    #[test]
    fn build_prompt_has_all_sections() {
        let facts = json!({ "short": "get-callers", "days_since_last_use": 42 });
        let (system, user) = build_prompt(InsightKind::ToolDormant, &facts, CopyLimits::default(), false);
        assert!(system.contains("quiet mentor"));
        assert!(user.contains("<task>") && user.contains("Kind: tool_dormant."));
        assert!(user.contains("<facts>") && user.contains("get-callers"));
        assert!(user.contains("title: at most 60 characters"));
        assert!(user.contains("detail: at most 180 characters"));
        assert!(user.contains("Return JSON:") && user.contains("No prose. No preamble."));
        // A fresh prompt carries no corrective block.
        assert!(!user.contains("<correction>"));
    }

    #[test]
    fn build_prompt_contains_banned_words_and_budget_from_source() {
        let facts = json!({ "short": "get-callers", "days_since_last_use": 42 });
        let (_system, user) = build_prompt(InsightKind::ToolDormant, &facts, CopyLimits::default(), false);
        // The char-budget instruction must be present and model-actionable.
        assert!(user.contains("Count the characters; do not exceed 180."));
        // Every banned word must appear in the prompt, derived FROM BANNED_WORDS
        // (single source of truth — never a hand-maintained second copy).
        assert!(!BANNED_WORDS.is_empty());
        for w in BANNED_WORDS {
            assert!(user.contains(w), "prompt must mention banned word {w:?}");
        }
        // The third-person markers are instructed in-prompt (from the same
        // source list voice_ok enforces) so the model self-corrects.
        assert!(!THIRD_PERSON_MARKERS.is_empty());
        for m in THIRD_PERSON_MARKERS {
            assert!(user.contains(m), "prompt must forbid third-person marker {m:?}");
        }
    }

    #[test]
    fn retry_prompt_contains_corrective_instruction() {
        let facts = json!({ "short": "get-callers" });
        let (_system, base) = build_prompt(InsightKind::ToolDormant, &facts, CopyLimits::default(), false);
        let (_system, retry) = build_prompt(InsightKind::ToolDormant, &facts, CopyLimits::default(), true);
        assert!(!base.contains("previous reply was rejected"));
        assert!(retry.contains("<correction>"));
        assert!(retry.contains("previous reply was rejected"));
        assert!(retry.contains("keep detail strictly under 180 characters"));
    }

    #[test]
    fn claim_inflight_dedups() {
        // Unique hash so the shared global set can't collide across tests.
        let h = "test_claim_inflight_dedups_unique_hash";
        // Make sure we start clean regardless of prior runs.
        release_inflight(h);
        // First claim owns the slot.
        assert!(claim_inflight(h), "first claim should own the slot");
        // Second claim for the same hash must NOT double-insert.
        assert!(!claim_inflight(h), "second claim for same hash must be refused");
        // After release, the slot is claimable again.
        release_inflight(h);
        assert!(claim_inflight(h), "slot claimable again after release");
        release_inflight(h);
    }
}
