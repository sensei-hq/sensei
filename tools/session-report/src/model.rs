//! What we read out of a Copilot CLI `events.jsonl`, and nothing more.
//!
//! The event names here were taken from real transcripts, not from documentation
//! — see `docs/2026-08-26-copilot-adapter-review.md`. The names the ingestion
//! adapter currently looks for (`tool_use`, `tool_result`) do not occur in any of
//! the 77,139 events sampled.

use std::collections::HashMap;

/// One tool invocation, start paired with completion.
#[derive(Debug, Clone)]
pub struct ToolCall {
    pub name: String,
    pub started_ms: i64,
    /// `None` when the session ended before the tool reported back — a real
    /// state, not a zero. Counted separately from a failure.
    pub ended_ms: Option<i64>,
    pub success: Option<bool>,
    /// The event id, so any claim in the report can be traced to a line.
    pub event_id: String,
}

impl ToolCall {
    pub fn failed(&self) -> bool {
        self.success == Some(false)
    }
}

/// One assistant turn, bounded by `assistant.turn_start` / `assistant.turn_end`.
#[derive(Debug, Clone)]
pub struct Turn {
    pub id: String,
    pub started_ms: i64,
    pub ended_ms: Option<i64>,
    pub model: Option<String>,
}

impl Turn {
    pub fn duration_ms(&self) -> Option<i64> {
        self.ended_ms.map(|e| e - self.started_ms)
    }
}

/// Totals the CLI reports at shutdown. Every field is `Option` because a session
/// killed with the lock file still in place never writes one — that is a real
/// gap, and reporting 0 tokens for it would be a lie.
/// One model's share of a session, from `session.shutdown.modelMetrics`.
#[derive(Debug, Clone, Default)]
pub struct ModelUse {
    pub requests: i64,
    /// Premium requests consumed. Only premium models charge any — most models
    /// report 0, so this is what actually draws down the plan allowance.
    pub premium: i64,
    pub output_tokens: i64,
    /// Billing "AI units", in nano. Reported alongside premium requests.
    pub nano_aiu: i64,
}

#[derive(Debug, Clone, Default)]
pub struct Totals {
    pub premium_requests: Option<i64>,
    pub api_duration_ms: Option<i64>,
    pub lines_added: Option<i64>,
    pub lines_removed: Option<i64>,
    pub files_modified: Option<usize>,
    pub input_tokens: Option<i64>,
    pub output_tokens: Option<i64>,
    pub cache_read_tokens: Option<i64>,
    pub cache_write_tokens: Option<i64>,
    pub reasoning_tokens: Option<i64>,
    pub nano_aiu: Option<i64>,
    pub by_model: HashMap<String, ModelUse>,
}

#[derive(Debug, Clone)]
pub struct Session {
    pub id: String,
    pub cwd: Option<String>,
    pub first_ms: i64,
    pub last_ms: i64,
    /// How many prompts the human wrote. A COUNT, not the prompts themselves —
    /// these are other people's transcripts and the report has no reason to hold
    /// their text in memory.
    pub prompts: usize,
    pub turns: Vec<Turn>,
    pub tools: Vec<ToolCall>,
    pub totals: Totals,
    pub models: HashMap<String, usize>,
    /// `session.permissions_changed` — each one is a point where the agent
    /// stopped and waited for the human.
    pub permission_events: usize,
    pub event_count: usize,
    /// Every event timestamp, ascending. Kept so ACTIVE time can be measured as
    /// the sum of gaps below an idle cutoff — a session left open overnight
    /// spans days of wall clock that nobody was working through.
    pub activity_ms: Vec<i64>,
    /// Sub-agent transcripts folded into this session. A delegated agent runs
    /// inside its parent's session but writes its own file, so the parent's
    /// transcript shows only the hand-off — all the work it did would otherwise
    /// be invisible.
    pub delegated: usize,
    /// True when the session directory still holds an `inuse.*.lock`, i.e. it
    /// was never cleanly closed. Those sessions have no shutdown totals.
    pub unclosed: bool,
}

/// Gaps longer than this are treated as "walked away", not work. Copilot turns
/// are seconds to a couple of minutes; ten minutes of silence is a break.
pub const IDLE_CUTOFF_MS: i64 = 10 * 60 * 1000;

impl Session {
    /// First to last event — includes idle time, so it is the session's SPAN,
    /// not its effort. Shown as such.
    pub fn wall_ms(&self) -> i64 {
        self.last_ms - self.first_ms
    }

    /// Time with something actually happening: consecutive-event gaps, with
    /// anything past the idle cutoff dropped.
    pub fn active_ms(&self) -> i64 {
        self.activity_ms
            .windows(2)
            .map(|w| w[1] - w[0])
            .filter(|d| *d >= 0 && *d < IDLE_CUTOFF_MS)
            .sum()
    }
    pub fn short_id(&self) -> &str {
        self.id.get(..8).unwrap_or(&self.id)
    }
}
