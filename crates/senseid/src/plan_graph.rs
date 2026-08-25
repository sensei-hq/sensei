//! The authored plan graph a `register_plan` run carries — the phase→task
//! structure with per-task `agent`/`model`/`spec_ref` + lifecycle state.
//!
//! This is the PURE core of the phase-1 automated-run planner→executor bridge
//! (design/automated-run.md, AR-2): the daemon stores this graph as jsonb on
//! `activity.runs.plan_graph`, and [`plan_to_segments`] projects it into the
//! relay outline the phone/console render. [`set_task_state`] is the write-back
//! `update_task_status` applies. No DB, no clock, no env — unit-testable.
//!
//! **Nesting** is carried as `parent_seq` (the phase's `seq`), not `parent_id`: the
//! Worker segments upsert keys on `(session_id, seq)` and assigns `id` server-side,
//! so a child can't reference a not-yet-assigned parent id in one publish. The daemon
//! authors each task with its phase's `seq`; the Worker resolves `parent_id` from it
//! after the upsert, so Phase/Step nesting survives. All per-task labels
//! (`agent`/`model`/`spec_ref`) still cross — zero-knowledge (D10): labels, never
//! spec bodies.

use dojo_protocol::relay::{RelaySegment, SegmentState};
use serde::{Deserialize, Serialize};

/// The authored plan: an ordered list of phases, each an ordered list of tasks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanGraph {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal: Option<String>,
    #[serde(default)]
    pub phases: Vec<PlanPhase>,
}

/// One phase — a titled group of tasks.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanPhase {
    pub title: String,
    #[serde(default)]
    pub tasks: Vec<PlanTask>,
}

/// One task node — the unit of work an executor dispatches. `agent`/`model`/
/// `spec_ref` are plan-authored labels; `state` is the lifecycle (defaults to
/// `pending` on authoring). `deps` are task ids this task waits on (the executor
/// derives the ready set from them).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanTask {
    pub id: String,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spec_ref: Option<String>,
    /// One-line, diff-free phone summary (optional).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(default = "default_pending")]
    pub state: SegmentState,
    #[serde(default)]
    pub deps: Vec<String>,
}

fn default_pending() -> SegmentState {
    SegmentState::Pending
}

impl PlanGraph {
    /// Total task count across all phases.
    pub fn task_count(&self) -> usize {
        self.phases.iter().map(|p| p.tasks.len()).sum()
    }

    /// Whether a task id exists in the graph — the validity check
    /// `update_task_status` uses before a state write-back.
    pub fn has_task(&self, task_id: &str) -> bool {
        self.phases.iter().any(|p| p.tasks.iter().any(|t| t.id == task_id))
    }
}

/// Project the authored graph into a relay outline: one segment per phase (its
/// state rolled up from its tasks), each followed by its task segments in order.
/// `seq` runs monotonically across the whole list so the Worker's `(session_id,
/// seq)` upsert is stable across re-publishes (the graph order is fixed at
/// authoring). Server ids are `None`; tasks carry `parent_seq` = their phase's
/// `seq` so the Worker can resolve `parent_id` (Phase/Step nesting) post-upsert.
pub fn plan_to_segments(graph: &PlanGraph) -> Vec<RelaySegment> {
    let mut segs = Vec::with_capacity(graph.phases.len() + graph.task_count());
    let mut seq: i32 = 0;
    for phase in &graph.phases {
        let phase_seq = seq;
        segs.push(RelaySegment {
            id: None,
            parent_id: None,
            parent_seq: None,
            seq: phase_seq,
            title: phase.title.clone(),
            summary: None,
            detail: None,
            state: phase_rollup(&phase.tasks),
            is_gate: false,
            gate_severity: None,
            response_verdict: None,
            response_note: None,
            agent: None,
            model: None,
            spec_ref: None,
        });
        seq += 1;
        for task in &phase.tasks {
            segs.push(RelaySegment {
                id: None,
                parent_id: None,
                parent_seq: Some(phase_seq),
                seq,
                title: task.title.clone(),
                summary: task.summary.clone(),
                detail: None,
                state: task.state,
                is_gate: false,
                gate_severity: None,
                response_verdict: None,
                response_note: None,
                agent: task.agent.clone(),
                model: task.model.clone(),
                spec_ref: task.spec_ref.clone(),
            });
            seq += 1;
        }
    }
    segs
}

/// The state a phase segment shows, rolled up from its tasks: any failure wins
/// (Failed), then any block/needs-review (Blocked), then all-terminal (Done),
/// then any progress (Active), else Pending. An empty phase is Pending.
fn phase_rollup(tasks: &[PlanTask]) -> SegmentState {
    if tasks.is_empty() {
        return SegmentState::Pending;
    }
    if tasks.iter().any(|t| t.state == SegmentState::Failed) {
        return SegmentState::Failed;
    }
    if tasks.iter().any(|t| matches!(t.state, SegmentState::Blocked | SegmentState::NeedsReview)) {
        return SegmentState::Blocked;
    }
    if tasks.iter().all(|t| matches!(t.state, SegmentState::Done | SegmentState::Skipped)) {
        return SegmentState::Done;
    }
    if tasks.iter().any(|t| {
        matches!(t.state, SegmentState::Active | SegmentState::Done | SegmentState::Skipped)
    }) {
        return SegmentState::Active;
    }
    SegmentState::Pending
}

/// Set a task's state in place. Returns `true` if the task id was found (and
/// updated), `false` otherwise — the caller turns `false` into a 404 rather than
/// silently writing back an unchanged graph.
pub fn set_task_state(graph: &mut PlanGraph, task_id: &str, state: SegmentState) -> bool {
    for phase in &mut graph.phases {
        for task in &mut phase.tasks {
            if task.id == task_id {
                task.state = state;
                return true;
            }
        }
    }
    false
}

/// Progress rollup over TASK nodes (not phase segments): `(done, total)` where a
/// task counts done in a terminal state (`Done`/`Skipped`). Phase-1 uses this for
/// the run's `progress_done`/`progress_total` so the phone shows task-level
/// completion, not phase count.
pub fn task_progress(graph: &PlanGraph) -> (i32, i32) {
    let mut done = 0i32;
    let mut total = 0i32;
    for phase in &graph.phases {
        for task in &phase.tasks {
            total += 1;
            if matches!(task.state, SegmentState::Done | SegmentState::Skipped) {
                done += 1;
            }
        }
    }
    (done, total)
}

/// Validate graph integrity before registration (AR-2 depth gate + trust
/// boundary): task ids are non-empty and unique, every `dep` references a real
/// task (no dangling refs, no self-dep), and the dependency edges form a DAG
/// (no cycles). Returns the first problem found, so `register_plan` turns it into
/// a clean 400 instead of persisting a broken graph an executor can't schedule.
pub fn validate(graph: &PlanGraph) -> Result<(), String> {
    use std::collections::HashMap;

    // 1. Non-empty, unique task ids. `indeg` doubles as the id set + the Kahn's
    //    in-degree counter (number of a task's own deps).
    let mut indeg: HashMap<&str, usize> = HashMap::new();
    for phase in &graph.phases {
        for task in &phase.tasks {
            if task.id.trim().is_empty() {
                return Err("a task has an empty id".into());
            }
            if indeg.insert(task.id.as_str(), 0).is_some() {
                return Err(format!("duplicate task id: {}", task.id));
            }
        }
    }

    // 2. Deps resolve (no dangling, no self-dep); build the dependents index and
    //    per-task in-degrees for the cycle check.
    let mut dependents: HashMap<&str, Vec<&str>> = HashMap::new();
    for phase in &graph.phases {
        for task in &phase.tasks {
            for dep in &task.deps {
                if dep == &task.id {
                    return Err(format!("task {} depends on itself", task.id));
                }
                if !indeg.contains_key(dep.as_str()) {
                    return Err(format!("task {} depends on unknown task {}", task.id, dep));
                }
                *indeg.get_mut(task.id.as_str()).expect("id present") += 1;
                dependents.entry(dep.as_str()).or_default().push(task.id.as_str());
            }
        }
    }

    // 3. DAG check (Kahn's): start from tasks with no deps, and each time a task
    //    is settled, relax the tasks that depend on it. If any task never reaches
    //    in-degree 0, its deps form a cycle.
    let mut queue: Vec<&str> = indeg.iter().filter(|&(_, &d)| d == 0).map(|(&k, _)| k).collect();
    let mut processed = 0usize;
    while let Some(node) = queue.pop() {
        processed += 1;
        if let Some(deps) = dependents.get(node) {
            for &t in deps {
                let e = indeg.get_mut(t).expect("dependent present");
                *e -= 1;
                if *e == 0 {
                    queue.push(t);
                }
            }
        }
    }
    if processed != indeg.len() {
        return Err("dependency cycle detected".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> PlanGraph {
        serde_json::from_value(json!({
            "goal": "ship register_plan",
            "phases": [
                { "title": "Schema", "tasks": [
                    { "id": "t1", "title": "add columns", "agent": "general-purpose", "model": "sonnet", "spec_ref": "docs/plan/x/tasks/t1.md", "state": "done" },
                    { "id": "t2", "title": "wire type", "model": "haiku" }
                ]},
                { "title": "Daemon", "tasks": [
                    { "id": "t3", "title": "register_plan", "agent": "general-purpose", "model": "opus", "deps": ["t1", "t2"] }
                ]}
            ]
        })).unwrap()
    }

    #[test]
    fn deserializes_with_pending_default_and_counts() {
        let g = sample();
        assert_eq!(g.task_count(), 3);
        assert!(g.has_task("t2"));
        assert!(!g.has_task("nope"));
        // t2 authored without a state → defaults to pending.
        assert_eq!(g.phases[0].tasks[1].state, SegmentState::Pending);
        assert_eq!(g.phases[0].tasks[0].state, SegmentState::Done);
        assert_eq!(g.phases[1].tasks[0].deps, vec!["t1", "t2"]);
    }

    #[test]
    fn projects_phase_then_tasks_with_parent_seq_and_labels() {
        let segs = plan_to_segments(&sample());
        // Schema(phase) + t1 + t2 + Daemon(phase) + t3 = 5 segments.
        assert_eq!(segs.len(), 5);
        // parent_id stays None (the Worker assigns it on upsert); nesting rides parent_seq.
        assert!(segs.iter().all(|s| s.parent_id.is_none()), "parent_id is Worker-assigned");
        assert_eq!(segs[0].parent_seq, None, "a phase has no parent");
        assert_eq!(segs[3].parent_seq, None, "a phase has no parent");
        assert_eq!(segs[1].parent_seq, Some(0), "t1 nests under Schema (seq 0)");
        assert_eq!(segs[2].parent_seq, Some(0), "t2 nests under Schema (seq 0)");
        assert_eq!(segs[4].parent_seq, Some(3), "t3 nests under Daemon (seq 3)");
        let seqs: Vec<i32> = segs.iter().map(|s| s.seq).collect();
        assert_eq!(seqs, vec![0, 1, 2, 3, 4], "monotonic seq across phases+tasks");

        assert_eq!(segs[0].title, "Schema");
        assert_eq!(segs[0].agent, None, "a phase carries no agent/model");
        // t1 rode its labels across.
        assert_eq!(segs[1].title, "add columns");
        assert_eq!(segs[1].agent.as_deref(), Some("general-purpose"));
        assert_eq!(segs[1].model.as_deref(), Some("sonnet"));
        assert_eq!(segs[1].spec_ref.as_deref(), Some("docs/plan/x/tasks/t1.md"));
        assert_eq!(segs[1].state, SegmentState::Done);
        assert_eq!(segs[2].state, SegmentState::Pending, "t2 default pending");
    }

    #[test]
    fn phase_state_rolls_up_from_tasks() {
        // Schema: t1 done, t2 pending → some progress but not all done → Active.
        let g = sample();
        assert_eq!(phase_rollup(&g.phases[0].tasks), SegmentState::Active);
        // Daemon: t3 pending → Pending.
        assert_eq!(phase_rollup(&g.phases[1].tasks), SegmentState::Pending);
        // Empty phase → Pending.
        assert_eq!(phase_rollup(&[]), SegmentState::Pending);
    }

    #[test]
    fn failure_and_block_dominate_the_rollup() {
        let failed = vec![
            PlanTask {
                id: "a".into(),
                title: "a".into(),
                agent: None,
                model: None,
                spec_ref: None,
                summary: None,
                state: SegmentState::Done,
                deps: vec![],
            },
            PlanTask {
                id: "b".into(),
                title: "b".into(),
                agent: None,
                model: None,
                spec_ref: None,
                summary: None,
                state: SegmentState::Failed,
                deps: vec![],
            },
        ];
        assert_eq!(phase_rollup(&failed), SegmentState::Failed);
        let blocked = vec![
            PlanTask {
                id: "a".into(),
                title: "a".into(),
                agent: None,
                model: None,
                spec_ref: None,
                summary: None,
                state: SegmentState::Done,
                deps: vec![],
            },
            PlanTask {
                id: "b".into(),
                title: "b".into(),
                agent: None,
                model: None,
                spec_ref: None,
                summary: None,
                state: SegmentState::Blocked,
                deps: vec![],
            },
        ];
        assert_eq!(phase_rollup(&blocked), SegmentState::Blocked);
        // All terminal (done + skipped) → Done.
        let alldone = vec![
            PlanTask {
                id: "a".into(),
                title: "a".into(),
                agent: None,
                model: None,
                spec_ref: None,
                summary: None,
                state: SegmentState::Done,
                deps: vec![],
            },
            PlanTask {
                id: "b".into(),
                title: "b".into(),
                agent: None,
                model: None,
                spec_ref: None,
                summary: None,
                state: SegmentState::Skipped,
                deps: vec![],
            },
        ];
        assert_eq!(phase_rollup(&alldone), SegmentState::Done);
    }

    #[test]
    fn set_task_state_finds_and_updates_or_reports_missing() {
        let mut g = sample();
        assert!(set_task_state(&mut g, "t3", SegmentState::Active));
        assert_eq!(g.phases[1].tasks[0].state, SegmentState::Active);
        // Unknown id → false, graph unchanged.
        assert!(!set_task_state(&mut g, "ghost", SegmentState::Done));
    }

    #[test]
    fn task_progress_counts_terminal_over_total() {
        let mut g = sample();
        assert_eq!(task_progress(&g), (1, 3), "t1 done of 3");
        set_task_state(&mut g, "t2", SegmentState::Skipped);
        set_task_state(&mut g, "t3", SegmentState::Done);
        assert_eq!(task_progress(&g), (3, 3), "all terminal");
    }

    #[test]
    fn round_trips_through_json_with_state() {
        let g = sample();
        let js = serde_json::to_string(&g).unwrap();
        let back: PlanGraph = serde_json::from_str(&js).unwrap();
        assert_eq!(back, g);
    }

    #[test]
    fn validate_accepts_a_dag() {
        assert!(validate(&sample()).is_ok(), "the sample graph is a valid DAG");
    }

    #[test]
    fn validate_rejects_duplicate_ids() {
        let g: PlanGraph = serde_json::from_value(json!({
            "phases": [{ "title": "P", "tasks": [
                { "id": "t1", "title": "a" }, { "id": "t1", "title": "b" }
            ]}]
        }))
        .unwrap();
        assert!(validate(&g).unwrap_err().contains("duplicate"));
    }

    #[test]
    fn validate_rejects_dangling_and_self_deps() {
        let dangling: PlanGraph = serde_json::from_value(json!({
            "phases": [{ "title": "P", "tasks": [
                { "id": "t1", "title": "a", "deps": ["ghost"] }
            ]}]
        }))
        .unwrap();
        assert!(validate(&dangling).unwrap_err().contains("unknown task"));
        let selfdep: PlanGraph = serde_json::from_value(json!({
            "phases": [{ "title": "P", "tasks": [
                { "id": "t1", "title": "a", "deps": ["t1"] }
            ]}]
        }))
        .unwrap();
        assert!(validate(&selfdep).unwrap_err().contains("itself"));
    }

    #[test]
    fn validate_rejects_a_cycle() {
        let g: PlanGraph = serde_json::from_value(json!({
            "phases": [{ "title": "P", "tasks": [
                { "id": "t1", "title": "a", "deps": ["t2"] },
                { "id": "t2", "title": "b", "deps": ["t1"] }
            ]}]
        }))
        .unwrap();
        assert!(validate(&g).unwrap_err().contains("cycle"));
    }
}
