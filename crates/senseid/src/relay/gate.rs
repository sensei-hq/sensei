//! The deterministic merge gate (queue cutover, task 2.8): a broker task the
//! sweeper hands over after an approval that names a branch. The gate checks
//! the branch merges clean, verifies the merged tree, and merges it into the
//! base — or says exactly why not. One [`GateRunner`] per backend, in the
//! house seam style (`relay_drivers/trait_def.rs`: return-position
//! `impl Future`, static dispatch): [`GitGateRunner`] for real,
//! [`FakeGateRunner`] for tests. The outcome's write is the store's
//! (`relay_gate_outcome`); this module never touches the database.

use std::future::Future;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::agent_spawn::{AgentCommand, AgentOutput, run_agent};

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
    async fn run(&self, input: &GateInput) -> GateOutcome {
        match git_gate(input).await {
            Ok(outcome) => outcome,
            Err(why) => GateOutcome::Error(why),
        }
    }
}

/// How much of a command's output a verdict carries.
const TAIL_CHARS: usize = 2000;
/// Cleanup commands (worktree removal, merge abort) never spend the gate's
/// budget: a spent budget still leaves the base clean.
const CLEANUP_TIMEOUT: Duration = Duration::from_secs(30);

/// The gate's one budget: every command takes what is left of it.
struct Budget {
    deadline: tokio::time::Instant,
}

impl Budget {
    fn new(timeout: Duration) -> Self {
        Self { deadline: tokio::time::Instant::now() + timeout }
    }

    fn remaining(&self, what: &str) -> Result<Duration, String> {
        let left = self.deadline.saturating_duration_since(tokio::time::Instant::now());
        if left.is_zero() {
            return Err(format!("gate timed out before {what}"));
        }
        Ok(left)
    }
}

/// The last `TAIL_CHARS` of what a command said, stderr after stdout.
fn tail(out: &AgentOutput) -> String {
    let text = format!("{}\n{}", out.stdout.trim_end(), out.stderr.trim_end());
    let text = text.trim();
    let start = text.len().saturating_sub(TAIL_CHARS);
    let start = text.char_indices().map(|(i, _)| i).find(|i| *i >= start).unwrap_or(0);
    text[start..].to_string()
}

/// Run `program args` in `dir` with `timeout`; a timeout is an error naming
/// the command, a spawn failure too. The exit code is the caller's to read.
async fn run_in(
    program: &str,
    args: &[&str],
    dir: &Path,
    timeout: Duration,
) -> Result<AgentOutput, String> {
    let shown = format!(
        "{} {}",
        Path::new(program).file_name().map_or(program, |n| n.to_str().unwrap_or(program)),
        args.join(" ")
    );
    let cmd = AgentCommand::new(program, args.iter().map(|a| a.to_string()).collect(), timeout)
        .with_cwd(dir);
    let out = run_agent(&cmd).await.map_err(|e| format!("{shown}: {e:?}"))?;
    if out.timed_out {
        return Err(format!("gate timed out during `{shown}`"));
    }
    Ok(out)
}

/// `run_in` for a command that must succeed: its trimmed stdout, else the
/// failure with the tail.
async fn must(
    program: &str,
    args: &[&str],
    dir: &Path,
    timeout: Duration,
) -> Result<String, String> {
    let out = run_in(program, args, dir, timeout).await?;
    match out.exit_code {
        Some(0) => Ok(out.stdout.trim().to_string()),
        code => Err(format!("`git {}` failed ({code:?}): {}", args.join(" "), tail(&out))),
    }
}

async fn git_gate(input: &GateInput) -> Result<GateOutcome, String> {
    let git = sensei_bootstrap::util::which_binary("git").ok_or("git not installed")?;
    let budget = Budget::new(input.timeout);
    let base = input.base_checkout.as_path();
    if !base.is_dir() {
        return Err(format!("base checkout {} is not a directory", base.display()));
    }
    // The base must be exactly what the run names, and untouched: the gate
    // merges into it, so a stray edit or another branch is never merged over.
    let status =
        must(&git, &["status", "--porcelain"], base, budget.remaining("git status")?).await?;
    if !status.is_empty() {
        return Err(format!("base checkout {} is dirty", base.display()));
    }
    let head = must(
        &git,
        &["rev-parse", "--abbrev-ref", "HEAD"],
        base,
        budget.remaining("git rev-parse")?,
    )
    .await?;
    if head != input.base_branch {
        return Err(format!("base checkout is on {head}, not {}", input.base_branch));
    }
    let branch_ref = format!("refs/heads/{}", input.branch);
    let known = run_in(
        &git,
        &["rev-parse", "--verify", "--quiet", &branch_ref],
        base,
        budget.remaining("git rev-parse")?,
    )
    .await?;
    if known.exit_code != Some(0) {
        return Err(format!("branch {} not found in {}", input.branch, base.display()));
    }
    // A conflict is known before anything is touched (git ≥ 2.38).
    let probe = run_in(
        &git,
        &["merge-tree", "--write-tree", &input.base_branch, &input.branch],
        base,
        budget.remaining("git merge-tree")?,
    )
    .await?;
    match probe.exit_code {
        Some(0) => {}
        Some(1) => return Ok(GateOutcome::Conflict { detail: tail(&probe) }),
        code => return Err(format!("`git merge-tree` failed ({code:?}): {}", tail(&probe))),
    }
    if let Some(verify_cmd) = input.verify_cmd.as_deref()
        && let Some(failed) = verify_merged_tree(&git, input, verify_cmd, &budget).await?
    {
        return Ok(failed);
    }
    let merge = run_in(
        &git,
        &["merge", "--no-ff", "--no-edit", &input.branch],
        base,
        budget.remaining("git merge")?,
    )
    .await?;
    if merge.exit_code != Some(0) {
        // Leave the base as found: a half merge is worse than none.
        let _ = run_in(&git, &["merge", "--abort"], base, CLEANUP_TIMEOUT).await;
        return Err(format!("`git merge --no-ff {}` failed: {}", input.branch, tail(&merge)));
    }
    let sha = must(&git, &["rev-parse", "HEAD"], base, budget.remaining("git rev-parse")?).await?;
    Ok(GateOutcome::Merged { sha })
}

/// Run the verify on the merged tree in a detached temporary worktree of the
/// branch with the base merged in — the base checkout itself stays as it is.
/// The worktree is removed whatever happens. `Ok(None)` is a pass.
async fn verify_merged_tree(
    git: &str,
    input: &GateInput,
    verify_cmd: &str,
    budget: &Budget,
) -> Result<Option<GateOutcome>, String> {
    let base = input.base_checkout.as_path();
    // A fresh path under the system temp dir: `git worktree add` creates
    // it, the cleanup below removes it.
    let worktree = std::env::temp_dir().join(format!("sensei-gate-{}", Uuid::now_v7().simple()));
    let worktree_str = worktree.to_string_lossy().into_owned();
    let verdict = async {
        must(
            git,
            &["worktree", "add", "--detach", &worktree_str, &input.branch],
            base,
            budget.remaining("git worktree add")?,
        )
        .await?;
        let merged = run_in(
            git,
            &["merge", "--no-edit", &input.base_branch],
            &worktree,
            budget.remaining("git merge (verify)")?,
        )
        .await?;
        if merged.exit_code != Some(0) {
            return Err(format!(
                "merging {} into the verify worktree failed: {}",
                input.base_branch,
                tail(&merged)
            ));
        }
        let sh = sensei_bootstrap::util::which_binary("sh").ok_or("sh not installed")?;
        let out =
            run_in(&sh, &["-c", verify_cmd], &worktree, budget.remaining("the verify")?).await?;
        match out.exit_code {
            Some(0) => Ok(None),
            Some(exit) => Ok(Some(GateOutcome::VerifyFailed {
                cmd: verify_cmd.to_string(),
                exit,
                tail: tail(&out),
            })),
            None => Err("the verify was killed before it exited".into()),
        }
    }
    .await;
    // Cleanup never spends the budget and never masks the verdict.
    let removed =
        run_in(git, &["worktree", "remove", "--force", &worktree_str], base, CLEANUP_TIMEOUT).await;
    if let Err(why) = removed {
        tracing::warn!(worktree = %worktree_str, %why, "relay gate: verify worktree not removed");
    }
    if worktree.exists()
        && let Err(why) = std::fs::remove_dir_all(&worktree)
    {
        tracing::warn!(worktree = %worktree_str, %why, "relay gate: verify dir not removed");
    }
    let _ = run_in(git, &["worktree", "prune"], base, CLEANUP_TIMEOUT).await;
    verdict
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
