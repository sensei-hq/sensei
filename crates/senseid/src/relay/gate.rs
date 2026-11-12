//! The deterministic merge gate (queue cutover, task 2.8): a broker task the
//! sweeper hands over after an approval that names a branch. The gate checks
//! the branch merges clean, verifies the merged tree, and merges it into the
//! base — or says exactly why not. One [`GateRunner`] per backend, in the
//! house seam style (`relay_drivers/trait_def.rs`: return-position
//! `impl Future`, static dispatch): [`GitGateRunner`] for real,
//! [`FakeGateRunner`] for tests. The outcome's write is the store's
//! (`relay_gate_outcome`); this module never touches the database.

use std::future::Future;
use std::path::PathBuf;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// The whole gate's wall-clock budget when `deterministic` names none.
pub const GATE_DEFAULT_TIMEOUT_SECS: u64 = 900;

/// `relay.tasks.deterministic` of a gate task, as minted at approval:
/// `{action: "gate", task_id, branch, base_checkout, base_branch,
/// verify_cmd?, timeout_seconds?}`. Unknown keys are refused — the shape is
/// the daemon's own, a stray key is a bug.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GateSpec {
    pub action: String,
    /// The approved agent task the gate merges.
    pub task_id: Uuid,
    /// From the task's latest full report.
    pub branch: String,
    /// The run's `config.base_checkout`: a checkout on `base_branch`.
    pub base_checkout: PathBuf,
    pub base_branch: String,
    /// The run's `config.verify_cmd`, run by `sh -c` in a temporary worktree
    /// of the merged tree; none skips the verify.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub verify_cmd: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_seconds: Option<u64>,
}

impl GateSpec {
    /// Parse a gate task's `deterministic`; the message names what is wrong.
    pub fn parse(value: &serde_json::Value) -> Result<Self, String> {
        let spec: Self = serde_json::from_value(value.clone()).map_err(|e| e.to_string())?;
        if spec.action != "gate" {
            return Err(format!("action must be gate, saw {}", spec.action));
        }
        for (name, v) in [("branch", &spec.branch), ("base_branch", &spec.base_branch)] {
            if v.trim().is_empty() {
                return Err(format!("{name} is empty"));
            }
        }
        if spec.base_checkout.as_os_str().is_empty() {
            return Err("base_checkout is empty".into());
        }
        Ok(spec)
    }

    pub fn input(&self) -> GateInput {
        GateInput {
            base_checkout: self.base_checkout.clone(),
            base_branch: self.base_branch.clone(),
            branch: self.branch.clone(),
            verify_cmd: self.verify_cmd.clone(),
            timeout: Duration::from_secs(self.timeout_seconds.unwrap_or(GATE_DEFAULT_TIMEOUT_SECS)),
        }
    }
}

/// One gate's inputs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateInput {
    pub base_checkout: PathBuf,
    pub base_branch: String,
    pub branch: String,
    pub verify_cmd: Option<String>,
    /// The budget for the whole gate, every command included.
    pub timeout: Duration,
}

/// What a gate found. `Merged` and the two refusals are the gate's verdict
/// on the branch; `Error` is the gate's own failure (the base checkout
/// unusable, git missing, the budget spent) — the person's to resolve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateOutcome {
    Merged { sha: String },
    Conflict { detail: String },
    VerifyFailed { cmd: String, exit: i32, tail: String },
    Error(String),
}

/// The gate backend seam.
pub trait GateRunner: Send + Sync {
    fn run(&self, input: &GateInput) -> impl Future<Output = GateOutcome> + Send;
}

/// The real gate over the base checkout: refuses a dirty tree or one not on
/// `base_branch`; `git merge-tree --write-tree` first (a conflict never
/// touches the tree); the verify runs in a detached temporary worktree with
/// the base merged in, always removed; then `git merge --no-ff` on the base.
pub struct GitGateRunner;

impl GateRunner for GitGateRunner {
    async fn run(&self, _input: &GateInput) -> GateOutcome {
        unimplemented!("task 2.8: the git gate")
    }
}

/// A gate that answers with a fixed outcome and remembers what it was asked.
#[cfg(test)]
pub struct FakeGateRunner {
    outcome: GateOutcome,
    pub seen: std::sync::Mutex<Vec<GateInput>>,
}

#[cfg(test)]
impl FakeGateRunner {
    pub fn new(outcome: GateOutcome) -> Self {
        Self { outcome, seen: std::sync::Mutex::new(Vec::new()) }
    }
}

#[cfg(test)]
impl GateRunner for FakeGateRunner {
    async fn run(&self, input: &GateInput) -> GateOutcome {
        self.seen.lock().unwrap().push(input.clone());
        self.outcome.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::path::Path;
    use std::process::Command;

    fn spec_json() -> serde_json::Value {
        json!({
            "action": "gate",
            "task_id": "01938931-0000-7000-8000-0000000000f9",
            "branch": "lane/x",
            "base_checkout": "/tmp/base",
            "base_branch": "develop",
            "verify_cmd": "make test-fast",
        })
    }

    #[test]
    fn gate_spec_parses_and_refuses_bad_shapes() {
        let spec = GateSpec::parse(&spec_json()).unwrap();
        assert_eq!(spec.branch, "lane/x");
        assert_eq!(spec.base_branch, "develop");
        assert_eq!(spec.verify_cmd.as_deref(), Some("make test-fast"));
        let input = spec.input();
        assert_eq!(input.timeout, Duration::from_secs(GATE_DEFAULT_TIMEOUT_SECS));
        assert_eq!(input.base_checkout, PathBuf::from("/tmp/base"));
        // The shape is the daemon's own: a stray key, a wrong action, an
        // empty branch or checkout are bugs, named.
        let mut stray = spec_json();
        stray["manifest"] = json!("x");
        assert!(GateSpec::parse(&stray).unwrap_err().contains("manifest"));
        let mut action = spec_json();
        action["action"] = json!("lint");
        assert!(GateSpec::parse(&action).unwrap_err().contains("action"));
        let mut branch = spec_json();
        branch["branch"] = json!("  ");
        assert!(GateSpec::parse(&branch).unwrap_err().contains("branch"));
        let mut checkout = spec_json();
        checkout["base_checkout"] = json!("");
        assert!(GateSpec::parse(&checkout).unwrap_err().contains("base_checkout"));
        let mut secs = spec_json();
        secs["timeout_seconds"] = json!(30);
        assert_eq!(GateSpec::parse(&secs).unwrap().input().timeout, Duration::from_secs(30));
        // Round trip: what the store writes, the gate reads.
        let back: serde_json::Value = serde_json::to_value(&spec).unwrap();
        assert_eq!(GateSpec::parse(&back).unwrap(), spec);
    }

    #[tokio::test]
    async fn fake_gate_returns_its_outcome_and_records_the_input() {
        let fake = FakeGateRunner::new(GateOutcome::Merged { sha: "abc".into() });
        let input = GateSpec::parse(&spec_json()).unwrap().input();
        assert_eq!(fake.run(&input).await, GateOutcome::Merged { sha: "abc".into() });
        assert_eq!(fake.seen.lock().unwrap().as_slice(), &[input]);
    }

    // ── the real gate, on a throwaway repository ──

    fn git(dir: &Path, args: &[&str]) -> String {
        let out = Command::new("git").args(args).current_dir(dir).output().unwrap();
        assert!(
            out.status.success(),
            "git {args:?}: {}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        String::from_utf8_lossy(&out.stdout).trim().to_string()
    }

    fn write(dir: &Path, name: &str, text: &str) {
        std::fs::write(dir.join(name), text).unwrap();
    }

    /// A repository with `main` (README) and `lane/x` (adds feature.txt);
    /// `conflict` also edits README's one line on both sides.
    fn repo(conflict: bool) -> tempfile::TempDir {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();
        git(dir, &["init", "-q", "-b", "main"]);
        git(dir, &["config", "user.email", "gate@test"]);
        git(dir, &["config", "user.name", "gate"]);
        write(dir, "README", "one\n");
        git(dir, &["add", "."]);
        git(dir, &["commit", "-qm", "init"]);
        git(dir, &["checkout", "-qb", "lane/x"]);
        write(dir, "feature.txt", "feature\n");
        if conflict {
            write(dir, "README", "branch\n");
        }
        git(dir, &["add", "."]);
        git(dir, &["commit", "-qm", "feature"]);
        git(dir, &["checkout", "-q", "main"]);
        if conflict {
            write(dir, "README", "main\n");
            git(dir, &["commit", "-qam", "main moves"]);
        }
        tmp
    }

    fn input(dir: &Path, verify_cmd: Option<&str>) -> GateInput {
        GateInput {
            base_checkout: dir.to_path_buf(),
            base_branch: "main".into(),
            branch: "lane/x".into(),
            verify_cmd: verify_cmd.map(str::to_owned),
            timeout: Duration::from_secs(60),
        }
    }

    fn worktrees(dir: &Path) -> usize {
        git(dir, &["worktree", "list"]).lines().count()
    }

    #[tokio::test]
    async fn git_gate_merges_a_clean_branch() {
        let tmp = repo(false);
        let dir = tmp.path();
        // The verify sees the merged tree (the branch's file is there).
        let out = GitGateRunner.run(&input(dir, Some("test -f feature.txt"))).await;
        let GateOutcome::Merged { sha } = out else { panic!("{out:?}") };
        assert_eq!(sha, git(dir, &["rev-parse", "main"]));
        assert!(
            git(dir, &["log", "-1", "--format=%P", "main"]).split(' ').count() == 2,
            "a --no-ff merge"
        );
        assert!(dir.join("feature.txt").exists());
        assert_eq!(worktrees(dir), 1, "the verify worktree is gone");
        assert_eq!(git(dir, &["status", "--porcelain"]), "");
    }

    #[tokio::test]
    async fn git_gate_reports_a_conflict() {
        let tmp = repo(true);
        let dir = tmp.path();
        let before = git(dir, &["rev-parse", "main"]);
        let out = GitGateRunner.run(&input(dir, Some("true"))).await;
        let GateOutcome::Conflict { detail } = out else { panic!("{out:?}") };
        assert!(detail.contains("README"), "{detail}");
        assert_eq!(git(dir, &["rev-parse", "main"]), before, "the base is untouched");
        assert_eq!(git(dir, &["status", "--porcelain"]), "");
        assert_eq!(worktrees(dir), 1);
    }

    #[tokio::test]
    async fn git_gate_reports_a_failed_verify() {
        let tmp = repo(false);
        let dir = tmp.path();
        let before = git(dir, &["rev-parse", "main"]);
        let cmd = "echo boom >&2; exit 3";
        let out = GitGateRunner.run(&input(dir, Some(cmd))).await;
        let GateOutcome::VerifyFailed { cmd: seen, exit, tail } = out else { panic!("{out:?}") };
        assert_eq!(seen, cmd);
        assert_eq!(exit, 3);
        assert!(tail.contains("boom"), "{tail}");
        assert_eq!(git(dir, &["rev-parse", "main"]), before, "nothing merged");
        assert_eq!(worktrees(dir), 1, "the verify worktree is gone");
        // No verify: a clean branch merges without one.
        let out = GitGateRunner.run(&input(dir, None)).await;
        assert!(matches!(out, GateOutcome::Merged { .. }), "{out:?}");
    }

    #[tokio::test]
    async fn git_gate_refuses_a_dirty_or_wrong_base() {
        let tmp = repo(false);
        let dir = tmp.path();
        write(dir, "README", "edited\n");
        let out = GitGateRunner.run(&input(dir, None)).await;
        let GateOutcome::Error(why) = out else { panic!("{out:?}") };
        assert!(why.contains("dirty"), "{why}");
        git(dir, &["checkout", "-q", "--", "README"]);
        git(dir, &["checkout", "-qb", "elsewhere"]);
        let out = GitGateRunner.run(&input(dir, None)).await;
        let GateOutcome::Error(why) = out else { panic!("{out:?}") };
        assert!(why.contains("main"), "{why}");
        // A missing branch is an error too, not a conflict.
        git(dir, &["checkout", "-q", "main"]);
        let mut missing = input(dir, None);
        missing.branch = "lane/nope".into();
        let out = GitGateRunner.run(&missing).await;
        assert!(matches!(out, GateOutcome::Error(_)), "{out:?}");
        // The budget is the gate's: spent, it says so.
        let mut spent = input(dir, Some("sleep 5"));
        spent.timeout = Duration::from_millis(200);
        let out = GitGateRunner.run(&spent).await;
        let GateOutcome::Error(why) = out else { panic!("{out:?}") };
        assert!(why.contains("time"), "{why}");
        assert_eq!(worktrees(dir), 1);
    }
}
