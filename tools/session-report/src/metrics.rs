//! Turning sessions into the handful of numbers worth showing someone.
//!
//! Every metric here is derived from a signal that is actually present in the
//! transcript. Nothing is estimated, and anything the data cannot answer is
//! `None` rather than a plausible-looking zero — a report that quietly reports
//! "0 failures" for a session with no completion events would be worse than
//! saying nothing.

use crate::model::Session;
use std::collections::HashMap;

pub fn pct(n: usize, d: usize) -> Option<f64> {
    (d > 0).then(|| 100.0 * n as f64 / d as f64)
}

/// Percentile over an already-collected sample. Nearest-rank, which needs no
/// interpolation and cannot invent a value that was never observed.
pub fn percentile(sorted: &[i64], p: f64) -> Option<i64> {
    if sorted.is_empty() {
        return None;
    }
    let rank = ((p / 100.0) * sorted.len() as f64).ceil().max(1.0) as usize;
    sorted.get(rank.min(sorted.len()) - 1).copied()
}

pub struct ToolStat {
    pub name: String,
    pub calls: usize,
    pub failures: usize,
}

impl ToolStat {
    pub fn failure_pct(&self) -> Option<f64> {
        pct(self.failures, self.calls)
    }
}

/// A run of the SAME tool failing back-to-back inside one session — the shape of
/// an agent stuck in a loop, which a flat failure rate hides.
pub struct FailureRun {
    pub session: String,
    pub tool: String,
    pub length: usize,
    pub at_ms: i64,
    pub event_id: String,
}

pub struct Analysis {
    pub sessions: usize,
    pub unclosed: usize,
    pub events: usize,
    pub skipped_lines: usize,
    pub first_ms: i64,
    pub last_ms: i64,
    pub active_days: usize,
    pub prompts: usize,
    pub turns: usize,
    pub tool_calls: usize,
    pub tool_failures: usize,
    pub tool_unreported: usize,
    pub turn_ms_sorted: Vec<i64>,
    pub tools: Vec<ToolStat>,
    pub failure_runs: Vec<FailureRun>,
    pub permission_events: usize,
    pub lines_added: i64,
    pub lines_removed: i64,
    pub files_modified: usize,
    pub premium_requests: i64,
    pub input_tokens: i64,
    pub cache_read_tokens: i64,
    pub api_duration_ms: i64,
    /// Span from first to last event, summed. Includes idle.
    pub wall_ms: i64,
    /// Time with activity, idle gaps removed — see Session::active_ms.
    pub active_ms: i64,
    /// Sessions that reported shutdown totals. The cost and code-change figures
    /// only cover these — stated so nobody reads a partial sum as a full one.
    pub sessions_with_totals: usize,
}

impl Analysis {
    pub fn tool_failure_pct(&self) -> Option<f64> {
        pct(self.tool_failures, self.tool_calls)
    }
    /// Share of input tokens served from cache. High is good: it means context
    /// is being reused rather than resent.
    pub fn cache_reuse_pct(&self) -> Option<f64> {
        let total = self.input_tokens + self.cache_read_tokens;
        (total > 0).then(|| 100.0 * self.cache_read_tokens as f64 / total as f64)
    }
    pub fn tools_per_prompt(&self) -> Option<f64> {
        (self.prompts > 0).then(|| self.tool_calls as f64 / self.prompts as f64)
    }
    pub fn turns_per_prompt(&self) -> Option<f64> {
        (self.prompts > 0).then(|| self.turns as f64 / self.prompts as f64)
    }
    /// Share of ACTIVE time spent waiting on the model. Measured against active
    /// time, not span: a session left open overnight would otherwise show a
    /// model share near zero and read as "barely used the assistant".
    pub fn api_share_pct(&self) -> Option<f64> {
        (self.active_ms > 0).then(|| 100.0 * self.api_duration_ms as f64 / self.active_ms as f64)
    }
}

pub fn analyse(sessions: &[Session], skipped_lines: usize) -> Analysis {
    let mut a = Analysis {
        sessions: sessions.len(),
        unclosed: 0,
        events: 0,
        skipped_lines,
        first_ms: i64::MAX,
        last_ms: 0,
        active_days: 0,
        prompts: 0,
        turns: 0,
        tool_calls: 0,
        tool_failures: 0,
        tool_unreported: 0,
        turn_ms_sorted: Vec::new(),
        tools: Vec::new(),
        failure_runs: Vec::new(),
        permission_events: 0,
        lines_added: 0,
        lines_removed: 0,
        files_modified: 0,
        premium_requests: 0,
        input_tokens: 0,
        cache_read_tokens: 0,
        api_duration_ms: 0,
        wall_ms: 0,
        active_ms: 0,
        sessions_with_totals: 0,
    };

    let mut by_tool: HashMap<String, (usize, usize)> = HashMap::new();
    let mut days: std::collections::HashSet<i64> = std::collections::HashSet::new();

    for s in sessions {
        a.events += s.event_count;
        a.prompts += s.prompts.len();
        a.turns += s.turns.len();
        a.permission_events += s.permission_events;
        a.wall_ms += s.wall_ms();
        a.active_ms += s.active_ms();
        if s.unclosed {
            a.unclosed += 1;
        }
        if s.first_ms > 0 {
            a.first_ms = a.first_ms.min(s.first_ms);
            a.last_ms = a.last_ms.max(s.last_ms);
            // Every day the session was actually active — not just its first and
            // last, which would count a session left open over a weekend as two.
            for t in &s.activity_ms {
                days.insert(t / 86_400_000);
            }
        }

        for t in &s.turns {
            if let Some(d) = t.duration_ms()
                && d >= 0
            {
                a.turn_ms_sorted.push(d);
            }
        }

        // Tool stats, plus consecutive-failure runs in call order.
        let mut run: Option<(String, usize, i64, String)> = None;
        for c in &s.tools {
            a.tool_calls += 1;
            let e = by_tool.entry(c.name.clone()).or_insert((0, 0));
            e.0 += 1;
            match c.success {
                Some(false) => {
                    a.tool_failures += 1;
                    e.1 += 1;
                    match &mut run {
                        Some((name, len, _, _)) if *name == c.name => *len += 1,
                        _ => {
                            run = Some((
                                c.name.clone(),
                                1,
                                c.started_ms,
                                c.event_id.clone(),
                            ))
                        }
                    }
                }
                None => {
                    a.tool_unreported += 1;
                    run = None;
                }
                Some(true) => run = None,
            }
            // A run of 3+ is worth surfacing; shorter is ordinary retrying.
            if let Some((name, len, at, ev)) = &run
                && *len >= 3
            {
                // Replace the in-progress entry so only the longest run is kept.
                a.failure_runs.retain(|f| {
                    !(f.session == s.id && f.tool == *name && f.at_ms == *at)
                });
                a.failure_runs.push(FailureRun {
                    session: s.id.clone(),
                    tool: name.clone(),
                    length: *len,
                    at_ms: *at,
                    event_id: ev.clone(),
                });
            }
        }

        let t = &s.totals;
        if t.premium_requests.is_some() || t.lines_added.is_some() {
            a.sessions_with_totals += 1;
        }
        a.lines_added += t.lines_added.unwrap_or(0);
        a.lines_removed += t.lines_removed.unwrap_or(0);
        a.files_modified += t.files_modified.unwrap_or(0);
        a.premium_requests += t.premium_requests.unwrap_or(0);
        a.input_tokens += t.input_tokens.unwrap_or(0);
        a.cache_read_tokens += t.cache_read_tokens.unwrap_or(0);
        a.api_duration_ms += t.api_duration_ms.unwrap_or(0);
    }

    if a.first_ms == i64::MAX {
        a.first_ms = 0;
    }
    a.active_days = days.len();
    a.turn_ms_sorted.sort_unstable();
    a.tools = by_tool
        .into_iter()
        .map(|(name, (calls, failures))| ToolStat { name, calls, failures })
        .collect();
    a.tools.sort_by(|x, y| y.calls.cmp(&x.calls));
    a.failure_runs.sort_by(|x, y| y.length.cmp(&x.length));
    a
}
