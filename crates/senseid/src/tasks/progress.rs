//! SSE progress events for task queue.

use serde::Serialize;
use tokio::sync::broadcast;

/// Events broadcast via SSE as tasks transition states.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum TaskEvent {
    Queued {
        task_id: u64,
    },
    Started {
        task_id: u64,
        folder_path: String,
        kind: String,
        path: String,
    },
    Completed {
        task_id: u64,
        folder_path: String,
        kind: String,
    },
    Failed {
        task_id: u64,
        folder_path: String,
        kind: String,
        error: String,
    },
    /// Emitted once when a folder's file tasks are first queued, carrying BOTH
    /// denominators — because there are two, at different grains, and a bar fed
    /// the wrong one is wrong in a way nothing errors on.
    ///
    /// `files_total` is TASK-grain: the files this scan enqueued. It is the one
    /// `progress_emitter`'s numerator pairs with — `files_completed` increments
    /// once per `process_file` completion — so the bar reaches 100% only if the
    /// denominator counts the same things. On a warm re-scan that is 3, not
    /// 2,305.
    ///
    /// `files_expected` is FILE-grain: every file the folder owns, written once
    /// at the structure barrier (R14, `folders.props.expected_files`). It is
    /// what `sensei.folder_completeness` divides by, whose numerator counts
    /// files that reached a verdict rather than tasks that ran. Read back from
    /// the row the barrier wrote — never recomputed here, because a
    /// re-derived denominator can disagree with the one the rest of the system
    /// uses (08 S2).
    ///
    /// `None` means the folder has no `expected_files` yet — a folder the
    /// structure walk has not reached. That is a real state and distinct from
    /// `Some(0)`, which says "walked, owns no indexable files". Reporting 0 for
    /// the unknown case would read as an empty folder (R4).
    FolderQueued {
        folder_path: String,
        files_total: u32,
        files_expected: Option<u32>,
    },

    /// A v2 pipeline STAGE began (08 S1) — `scan_root`, `scan_repo`,
    /// `library_discovery`, `structure_write`.
    ///
    /// Its own variant rather than a reuse of [`Self::Started`], which carries a
    /// queue `task_id`. These stages are not queue tasks and have no id;
    /// synthesising one would let a consumer that keys on task ids merge a stage
    /// with a real task, and nothing would report the collision.
    StageStarted {
        stage: String,
        /// What the stage ran over — the scan directory for `scan_root`, the
        /// repo for the rest.
        path: String,
    },
    /// A stage finished successfully, with the count it produced.
    StageCompleted {
        stage: String,
        path: String,
        /// What the stage produced, in whatever it counts: roots found, files
        /// written, pages ingested, files expected at the barrier.
        items: u64,
        /// **No silent caps** (08 S5). `Some(reason)` when the stage bounded its
        /// own coverage — a top-N, a sample, a give-up-after-one-try. Silent
        /// truncation reads as "covered everything" when it did not, and the
        /// only moment that is knowable is here.
        capped: Option<String>,
    },
    /// A stage failed, with the reason. Never silence: a stage that stops
    /// emitting is indistinguishable from one that finished (08 S4).
    StageFailed {
        stage: String,
        path: String,
        error: String,
    },
}

/// Where a stage's progress goes.
///
/// Carrying the sink as a value rather than reaching for a global is what lets
/// `scan_and_write_structure` stay callable from a test, a CLI and the daemon
/// without three code paths.
///
/// **Absent is a supported state, not a degraded one.** A scan driven from a
/// test or a one-shot command has no SSE subscriber, and so does a daemon whose
/// UI is closed — the spec's failure table says to emit anyway, because a
/// broadcast with no receiver is not an error. Both cases land here as a send
/// whose result is deliberately dropped.
#[derive(Clone, Copy, Default)]
pub struct StageEvents<'a>(Option<&'a broadcast::Sender<TaskEvent>>);

impl<'a> StageEvents<'a> {
    /// Emit to `tx`.
    pub fn to(tx: &'a broadcast::Sender<TaskEvent>) -> Self {
        Self(Some(tx))
    }

    /// Emit nowhere. Named rather than `default()` at call sites, so a caller
    /// that drops progress on the floor says so.
    pub fn none() -> Self {
        Self(None)
    }

    fn send(&self, evt: TaskEvent) {
        // Dropped ON PURPOSE, in ONE place: `send` fails only when no receiver
        // is attached, which is not a failure of the scan. Ignoring it at each
        // call site would make an ignored real error look the same.
        if let Some(tx) = self.0 {
            let _ = tx.send(evt);
        }
    }

    /// Announce a stage and get back the thing that must report its outcome.
    ///
    /// A guard rather than a matching `completed` call, because a matching call
    /// is a CONVENTION and a convention is what an early `return`, a `?`, or a
    /// panic walks straight past — leaving a UI showing work that stopped
    /// minutes ago (08 S4). Here the terminal event comes from `Drop`, so the
    /// only way to start a stage and never end it is to not start it.
    pub fn begin(&self, stage: &str, path: &str) -> Stage<'a> {
        self.send(TaskEvent::StageStarted { stage: stage.into(), path: path.into() });
        Stage { events: *self, stage: stage.into(), path: path.into(), reported: false }
    }
}

/// One running stage. Reports its outcome exactly once — explicitly if the
/// stage says so, and from `Drop` if it does not.
pub struct Stage<'a> {
    events: StageEvents<'a>,
    stage: String,
    path: String,
    reported: bool,
}

impl Stage<'_> {
    pub fn completed(mut self, items: u64) {
        self.reported = true;
        self.events.send(TaskEvent::StageCompleted {
            stage: std::mem::take(&mut self.stage),
            path: std::mem::take(&mut self.path),
            items,
            capped: None,
        });
    }

    /// A stage that bounded its own coverage says what it left out (08 S5).
    /// Silent truncation reads as "covered everything" when it did not, and
    /// this is the only moment that is knowable.
    pub fn completed_capped(mut self, items: u64, capped: &str) {
        self.reported = true;
        self.events.send(TaskEvent::StageCompleted {
            stage: std::mem::take(&mut self.stage),
            path: std::mem::take(&mut self.path),
            items,
            capped: Some(capped.into()),
        });
    }

    pub fn failed(mut self, error: &str) {
        self.reported = true;
        self.events.send(TaskEvent::StageFailed {
            stage: std::mem::take(&mut self.stage),
            path: std::mem::take(&mut self.path),
            error: error.into(),
        });
    }
}

impl Drop for Stage<'_> {
    fn drop(&mut self) {
        if self.reported {
            return;
        }
        // The reason says what is KNOWN — the stage ended without deciding —
        // and not a guess at why. A drop reaches here from a panic, a `?`, and
        // an early return alike, and this code cannot tell them apart; naming
        // one of them would be stating something it does not know (R4).
        self.events.send(TaskEvent::StageFailed {
            stage: std::mem::take(&mut self.stage),
            path: std::mem::take(&mut self.path),
            error: "the stage ended with no verdict — it panicked or returned early".into(),
        });
    }
}

/// Accumulated progress for a single repo.
#[derive(Debug, Clone, Default, Serialize)]
pub struct RepoProgress {
    pub total: u32,
    pub pending: u32,
    pub running: u32,
    pub completed: u32,
    pub failed: u32,
    pub current_file: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A stage that starts must reach a terminal event even if it PANICS
    /// (08 S4, and its failure table: "a stage panics -> `Failed` must still be
    /// emitted, or the UI shows a task running forever").
    ///
    /// A paired `started`/`completed` call is a convention, and a convention is
    /// exactly what an early `return` or a panic walks straight past. The guard
    /// makes the pairing structural: the terminal event is emitted by `Drop`, so
    /// the only way to avoid one is to not start the stage at all.
    #[test]
    fn a_stage_that_panics_still_reaches_a_terminal_event() {
        let (tx, mut rx) = broadcast::channel(8);
        let events = StageEvents::to(&tx);

        let hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _stage = events.begin("scan_repo", "/x");
            panic!("the parse wedged");
        }));
        std::panic::set_hook(hook);
        assert!(outcome.is_err(), "the fixture must actually panic, or it proves nothing");

        let mut terminal = None;
        while let Ok(evt) = rx.try_recv() {
            if let TaskEvent::StageFailed { stage, error, .. } = evt {
                terminal = Some((stage, error));
            }
        }
        let (stage, error) = terminal.expect("a panicking stage still emits a terminal event");
        assert_eq!(stage, "scan_repo");
        assert!(
            error.contains("no verdict"),
            "and the reason says the stage ended without deciding, rather than inventing one: \
             {error}"
        );
    }

    /// The guard emits exactly ONE terminal event, and it is the real one.
    ///
    /// Without this, a stage that completed would ALSO get a `StageFailed` from
    /// `Drop` — a failure reported for work that succeeded, which is worse than
    /// no event at all because a consumer cannot tell it from a real failure.
    #[test]
    fn a_completed_stage_emits_one_terminal_event_and_it_is_not_a_failure() {
        let (tx, mut rx) = broadcast::channel(8);
        let events = StageEvents::to(&tx);
        {
            let stage = events.begin("scan_root", "/x");
            stage.completed(7);
        }

        let mut terminals = Vec::new();
        while let Ok(evt) = rx.try_recv() {
            match evt {
                TaskEvent::StageCompleted { items, capped, .. } => {
                    terminals.push(format!("completed:{items}:{capped:?}"))
                }
                TaskEvent::StageFailed { error, .. } => terminals.push(format!("failed:{error}")),
                _ => {}
            }
        }
        assert_eq!(
            terminals,
            vec!["completed:7:None".to_string()],
            "one terminal, and it is the \
             one the stage reported"
        );
    }

    #[test]
    fn repo_progress_default() {
        let p = RepoProgress::default();
        assert_eq!(p.total, 0);
        assert_eq!(p.running, 0);
        assert!(p.current_file.is_none());
    }

    #[test]
    fn task_event_serializes() {
        let evt = TaskEvent::Completed {
            task_id: 1,
            folder_path: "/code/app".into(),
            kind: "process_file".into(),
        };
        let json = serde_json::to_string(&evt).unwrap();
        assert!(json.contains("\"event\":\"completed\""));
        assert!(json.contains("\"folder_path\":\"/code/app\""));
    }
}
