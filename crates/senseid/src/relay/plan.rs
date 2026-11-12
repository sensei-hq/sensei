//! Sprint-run plans (queue cutover, task 2.3): the JSON block a planning lane
//! puts in its Task 0 `report_md`, parsed and validated with no database in
//! sight. The store unpacks a validated plan into `relay.tasks` (task 2.5);
//! the reviewer's approval is refused with `relay_plan_invalid` when the plan
//! does not pass here, so a plan that cannot unpack never reaches the queue.
//!
//! The contract (design §7.2): `{"tasks": [{key, title, phase, owned_paths,
//! acceptance, deps, resources?, repo_key?}]}` — keys unique, every dep a
//! key, no cycles, every task titled with at least one acceptance line, no
//! task in the `planning` phase (a plan never nests a plan). Unknown keys
//! are refused so a planner's typo is a clear error, not a silently dropped
//! field.

use serde::Deserialize;

use crate::relay::RelayTaskPhase;

/// One task of a plan, as the planner wrote it. Keys are the planner's own
/// handles (`t1`); the store maps them to uuids at unpack.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanTask {
    pub key: String,
    pub title: String,
    pub phase: RelayTaskPhase,
    #[serde(default)]
    pub owned_paths: Vec<String>,
    #[serde(default)]
    pub acceptance: Vec<String>,
    #[serde(default)]
    pub deps: Vec<String>,
    #[serde(default)]
    pub resources: Vec<String>,
    /// Overrides the run's repository for this task; absent means the run's.
    #[serde(default)]
    pub repo_key: Option<String>,
}

/// The plan: a list of tasks forming a DAG over `deps`.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PlanSpec {
    pub tasks: Vec<PlanTask>,
}

/// Why a plan cannot unpack. Each names the task it is about so the
/// reviewer's `changes_requested` can quote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    /// No fenced ```json block, and the body is not JSON either.
    NoJsonBlock,
    /// The block is not the plan shape (serde's message, which names the key
    /// or the value at fault).
    Json(String),
    /// `tasks` is empty.
    Empty,
    /// Two tasks share a key.
    DuplicateKey(String),
    /// A dep names no task key.
    UnknownDep { task: String, dep: String },
    /// The deps loop; the keys on the loop, in plan order.
    Cycle(Vec<String>),
    /// A task with a blank title.
    EmptyTitle(String),
    /// A task with no acceptance line.
    NoAcceptance(String),
    /// A task in the `planning` phase: a plan never nests a plan.
    BadPhase(String),
}

impl PlanError {
    /// One line for the 409 body: what is wrong and which task.
    pub fn message(&self) -> String {
        match self {
            PlanError::NoJsonBlock => "the report carries no ```json plan block".into(),
            PlanError::Json(detail) => format!("the plan block is not the plan shape: {detail}"),
            PlanError::Empty => "the plan has no tasks".into(),
            PlanError::DuplicateKey(key) => format!("task key {key} appears twice"),
            PlanError::UnknownDep { task, dep } => {
                format!("task {task} depends on {dep}, which names no task")
            }
            PlanError::Cycle(keys) => format!("the deps loop: {}", keys.join(" -> ")),
            PlanError::EmptyTitle(key) => format!("task {key} has no title"),
            PlanError::NoAcceptance(key) => format!("task {key} has no acceptance line"),
            PlanError::BadPhase(key) => format!("task {key} is in the planning phase; a plan never nests a plan"),
        }
    }
}

/// The plan in `report_md`: the last fenced ```json block wins; a body
/// with no fence is parsed whole. Parsing only — see [`validate_plan`].
pub fn parse_plan(report_md: &str) -> Result<PlanSpec, PlanError> {
    let _ = report_md;
    unimplemented!("task 2.3: parse_plan")
}

/// Keys unique, deps resolve, no cycles, every task titled with at least
/// one acceptance line, no task in the planning phase.
pub fn validate_plan(spec: &PlanSpec) -> Result<(), PlanError> {
    let _ = spec;
    unimplemented!("task 2.3: validate_plan")
}

/// Indices into `spec.tasks` in an insertion order that puts every dep
/// before its dependants, stable by plan order among the ready. Call on a
/// validated plan (a cycle would leave tasks out).
pub fn topo_order(spec: &PlanSpec) -> Vec<usize> {
    let _ = spec;
    unimplemented!("task 2.3: topo_order")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The design's §7.2 example, verbatim.
    const EXAMPLE: &str = r#"Plan for the sprint.

```json
{
  "tasks": [
    {
      "key": "t1",
      "title": "Apply schema deltas for relay.runs and relay.tasks",
      "phase": "impl",
      "owned_paths": [
        "database/ddl/table/relay/runs.ddl",
        "database/ddl/table/relay/tasks.ddl"
      ],
      "acceptance": ["psql -f database/ddl/table/relay/runs.ddl succeeds"],
      "deps": []
    },
    {
      "key": "t2",
      "title": "Add claim_next and claim_review handlers to senseid",
      "phase": "impl",
      "owned_paths": [
        "crates/senseid/src/api/handlers/relay.rs",
        "crates/senseid/src/db/pg_store/relay.rs"
      ],
      "acceptance": ["cargo check --bin senseid succeeds"],
      "deps": ["t1"]
    }
  ]
}
```

Sized to one turn each."#;

    fn task(key: &str, deps: &[&str]) -> PlanTask {
        PlanTask {
            key: key.into(),
            title: format!("Task {key}"),
            phase: RelayTaskPhase::Impl,
            owned_paths: vec![],
            acceptance: vec!["green".into()],
            deps: deps.iter().map(|d| d.to_string()).collect(),
            resources: vec![],
            repo_key: None,
        }
    }

    #[test]
    fn parses_the_design_example_into_two_tasks_with_a_dep() {
        let spec = parse_plan(EXAMPLE).unwrap();
        assert_eq!(spec.tasks.len(), 2);
        assert_eq!(spec.tasks[0].key, "t1");
        assert_eq!(spec.tasks[0].phase, RelayTaskPhase::Impl);
        assert_eq!(spec.tasks[0].owned_paths.len(), 2);
        assert_eq!(spec.tasks[1].deps, vec!["t1".to_string()]);
        assert_eq!(spec.tasks[1].resources, Vec::<String>::new());
        assert_eq!(spec.tasks[1].repo_key, None);
        assert_eq!(validate_plan(&spec), Ok(()));
        assert_eq!(topo_order(&spec), vec![0, 1]);
    }

    #[test]
    fn the_last_json_block_wins_and_an_unfenced_body_parses_whole() {
        let two_blocks = "```json\n{\"not\": \"a plan\"}\n```\nrevised:\n```json\n{\"tasks\": [{\"key\": \"t1\", \"title\": \"T\", \"phase\": \"tests\", \"acceptance\": [\"ok\"]}]}\n```";
        let spec = parse_plan(two_blocks).unwrap();
        assert_eq!(spec.tasks[0].key, "t1");
        let bare = "{\"tasks\": [{\"key\": \"t1\", \"title\": \"T\", \"phase\": \"docs\", \"acceptance\": [\"ok\"]}]}";
        assert_eq!(parse_plan(bare).unwrap().tasks[0].phase, RelayTaskPhase::Docs);
    }

    #[test]
    fn prose_without_a_plan_is_no_json_block() {
        assert_eq!(parse_plan("I looked at the repo and it is fine."), Err(PlanError::NoJsonBlock));
        assert_eq!(parse_plan(""), Err(PlanError::NoJsonBlock));
    }

    #[test]
    fn a_malformed_or_misshapen_block_is_a_json_error_naming_the_fault() {
        let broken = "```json\n{\"tasks\": [\n```";
        assert!(matches!(parse_plan(broken), Err(PlanError::Json(_))));
        // `merge` is not a phase (Q3); an unknown key is refused, not dropped.
        let bad_phase = "```json\n{\"tasks\": [{\"key\": \"t1\", \"title\": \"T\", \"phase\": \"merge\", \"acceptance\": [\"ok\"]}]}\n```";
        match parse_plan(bad_phase) {
            Err(PlanError::Json(detail)) => assert!(detail.contains("merge"), "{detail}"),
            other => panic!("{other:?}"),
        }
        let extra = "```json\n{\"tasks\": [{\"key\": \"t1\", \"title\": \"T\", \"phase\": \"impl\", \"acceptance\": [\"ok\"], \"estimate\": 3}]}\n```";
        match parse_plan(extra) {
            Err(PlanError::Json(detail)) => assert!(detail.contains("estimate"), "{detail}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn validate_refuses_empty_duplicates_unknown_deps_and_blank_tasks() {
        assert_eq!(validate_plan(&PlanSpec { tasks: vec![] }), Err(PlanError::Empty));
        let dup = PlanSpec { tasks: vec![task("t1", &[]), task("t1", &[])] };
        assert_eq!(validate_plan(&dup), Err(PlanError::DuplicateKey("t1".into())));
        let unknown = PlanSpec { tasks: vec![task("t1", &[]), task("t2", &["t9"])] };
        assert_eq!(
            validate_plan(&unknown),
            Err(PlanError::UnknownDep { task: "t2".into(), dep: "t9".into() })
        );
        let mut blank = task("t1", &[]);
        blank.title = "   ".into();
        assert_eq!(
            validate_plan(&PlanSpec { tasks: vec![blank] }),
            Err(PlanError::EmptyTitle("t1".into()))
        );
        let mut untested = task("t1", &[]);
        untested.acceptance.clear();
        assert_eq!(
            validate_plan(&PlanSpec { tasks: vec![untested] }),
            Err(PlanError::NoAcceptance("t1".into()))
        );
        let mut nested = task("t1", &[]);
        nested.phase = RelayTaskPhase::Planning;
        assert_eq!(
            validate_plan(&PlanSpec { tasks: vec![nested] }),
            Err(PlanError::BadPhase("t1".into()))
        );
    }

    #[test]
    fn validate_refuses_cycles_self_deps_included() {
        let selfdep = PlanSpec { tasks: vec![task("t1", &["t1"])] };
        assert_eq!(validate_plan(&selfdep), Err(PlanError::Cycle(vec!["t1".into()])));
        let ring = PlanSpec {
            tasks: vec![task("a", &["c"]), task("b", &["a"]), task("c", &["b"]), task("d", &[])],
        };
        match validate_plan(&ring) {
            Err(PlanError::Cycle(keys)) => {
                assert_eq!(keys, vec!["a".to_string(), "b".into(), "c".into()]);
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn topo_order_puts_deps_first_and_keeps_plan_order_among_the_ready() {
        // Written dependants-first: the order flips; among the ready, the
        // plan's own order holds (b before d).
        let spec = PlanSpec {
            tasks: vec![task("t2", &["t1"]), task("b", &[]), task("t1", &[]), task("d", &["b"])],
        };
        assert_eq!(validate_plan(&spec), Ok(()));
        let order: Vec<&str> = topo_order(&spec).into_iter().map(|i| spec.tasks[i].key.as_str()).collect();
        assert_eq!(order, vec!["b", "t1", "t2", "d"]);
    }

    #[test]
    fn messages_name_the_task_for_the_reviewer() {
        assert!(PlanError::DuplicateKey("t1".into()).message().contains("t1"));
        assert!(
            PlanError::UnknownDep { task: "t2".into(), dep: "t9".into() }
                .message()
                .contains("t9")
        );
        assert!(PlanError::Cycle(vec!["a".into(), "b".into()]).message().contains("a -> b"));
        assert!(PlanError::NoJsonBlock.message().contains("json"));
    }
}
