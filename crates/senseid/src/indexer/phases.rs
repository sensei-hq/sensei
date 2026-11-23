//! Per-phase wall time for one file's indexing, persisted to
//! `task_executions.phases_ms` on completion — so an 82-second 9KB file
//! attributes itself instead of needing a profiler.
//!
//! A task-local, not a parameter: the phases span three layers (the handler
//! measures world-load and compile-wait, `index_file` measures parse and
//! resolve, `index_and_persist` measures the write) and threading a recorder
//! through every signature would churn each one. Handlers and pipeline fns
//! record through [`PhaseSplits::record`]; outside a task scope recording is
//! a silent no-op, so unit tests calling handlers directly never panic.
//!
//! Absent phases serialize missing, never zero: a phase the file never
//! reached (no compiler for its language, an early return) is not a phase
//! that took no time.

/// Per-phase wall milliseconds. Field set is the contract `phases_ms`
/// documents in the DDL comment — add a field here and there together.
#[derive(Debug, Default, serde::Serialize)]
pub struct PhaseSplits {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parse_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub world_load_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compile_wait_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolve_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub write_ms: Option<i64>,
}

impl PhaseSplits {
    pub fn is_empty(&self) -> bool {
        self.parse_ms.is_none()
            && self.world_load_ms.is_none()
            && self.compile_wait_ms.is_none()
            && self.resolve_ms.is_none()
            && self.write_ms.is_none()
    }

    /// Record splits from anywhere in the running task.
    pub fn record(f: impl FnOnce(&mut PhaseSplits)) {
        let _ = PHASE_SPLITS.try_with(|p| f(&mut p.borrow_mut()));
    }

    /// Take the current task's splits as JSON, if any were recorded.
    pub fn take_json() -> Option<serde_json::Value> {
        PHASE_SPLITS
            .try_with(|p| {
                let splits = p.borrow();
                (!splits.is_empty()).then(|| serde_json::to_value(&*splits).ok())
            })
            .ok()
            .flatten()
            .flatten()
    }
}

tokio::task_local! {
    static PHASE_SPLITS: std::cell::RefCell<PhaseSplits>;
}

/// Run `fut` with a fresh phase outbox installed; what the task records is
/// readable via [`PhaseSplits::take_json`] until the scope ends.
pub async fn scope<Fut>(fut: Fut) -> Fut::Output
where
    Fut: std::future::Future,
{
    PHASE_SPLITS.scope(std::cell::RefCell::new(PhaseSplits::default()), fut).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn recorded_splits_read_back_until_the_scope_ends() {
        let got = scope(async {
            PhaseSplits::record(|p| p.parse_ms = Some(3));
            PhaseSplits::take_json()
        })
        .await;
        assert_eq!(got.as_ref().and_then(|v| v.get("parse_ms")), Some(&serde_json::json!(3)));
        assert!(got.as_ref().and_then(|v| v.get("write_ms")).is_none(), "absent reads missing");
        assert!(PhaseSplits::take_json().is_none(), "nothing outlives the scope");
    }

    #[test]
    fn recording_outside_a_scope_is_a_silent_no_op() {
        PhaseSplits::record(|p| p.parse_ms = Some(1));
        assert!(PhaseSplits::take_json().is_none());
    }
}
