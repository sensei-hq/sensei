//! D-CHECKER — run a repo's checker-backed rules and record pass/fail verdicts.
//!
//! A rule with `verification = 'checker'` names a canonical command verb in
//! `checker_ref` (lint | test | build). For a given repo the daemon maps that
//! verb to the repo's manifest-discovered command (`sensei.project_commands`),
//! runs it in the repo directory, and records the outcome in
//! `sensei.rule_check_runs`. This turns an otherwise advisory rule into an
//! enforceable signal.
//!
//! v0 runs ONLY manifest-declared commands — never an arbitrary string carried
//! by the rule — so an adopted pack cannot smuggle a command to execute. Regex
//! and skill checker kinds, and CI/hook triggers, are deferred.

use std::path::Path;
use std::time::Duration;

use crate::db::pg_store::PgStore;

/// How long a single checker may run before it's killed and recorded as `error`.
const CHECKER_TIMEOUT: Duration = Duration::from_secs(300);
/// Keep at most this many bytes of combined output for triage.
const OUTPUT_TAIL_BYTES: usize = 4000;

/// The outcome of one checker run.
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct CheckRun {
    pub rule_statement: String,
    pub checker_ref: String,
    pub command: String,
    /// `pass` | `fail` | `skipped` | `error`.
    pub verdict: String,
    pub exit_code: Option<i32>,
}

/// Map a process exit code to a verdict: `Some(0)` = pass, any other code =
/// fail. `None` means no exit code was captured (the caller only reaches here
/// with a real status; timeouts/spawn failures are recorded as `error` directly).
pub fn verdict_from_exit(code: Option<i32>) -> &'static str {
    match code {
        Some(0) => "pass",
        Some(_) => "fail",
        None => "error",
    }
}

/// Keep the last `OUTPUT_TAIL_BYTES` of combined output (on a char boundary) so a
/// verdict carries triage context without storing megabytes.
fn tail(s: &str) -> String {
    if s.len() <= OUTPUT_TAIL_BYTES {
        return s.to_string();
    }
    let cut = s.len() - OUTPUT_TAIL_BYTES;
    let start = (cut..s.len()).find(|&i| s.is_char_boundary(i)).unwrap_or(s.len());
    format!("…{}", &s[start..])
}

/// Run one command line (whitespace-split into argv) in `cwd` with a timeout.
/// Returns `(verdict, exit_code, output_tail)`.
async fn run_command(command: &str, cwd: &Path) -> (&'static str, Option<i32>, String) {
    // ponytail: naive whitespace split — fine for the canonical `cargo clippy` /
    // `npm test` shape; swap for shell-words if discovered commands ever carry
    // quotes or pipes.
    let mut parts = command.split_whitespace();
    let Some(program) = parts.next() else {
        return ("error", None, "empty command".to_string());
    };
    let args: Vec<&str> = parts.collect();

    let mut cmd = tokio::process::Command::new(program);
    cmd.args(&args).current_dir(cwd);

    match tokio::time::timeout(CHECKER_TIMEOUT, cmd.output()).await {
        Ok(Ok(out)) => {
            let mut combined = String::from_utf8_lossy(&out.stdout).into_owned();
            combined.push_str(&String::from_utf8_lossy(&out.stderr));
            (verdict_from_exit(out.status.code()), out.status.code(), tail(&combined))
        }
        Ok(Err(e)) => ("error", None, format!("spawn failed: {e}")),
        Err(_) => ("error", None, format!("timed out after {}s", CHECKER_TIMEOUT.as_secs())),
    }
}

/// Resolve a repo's adopted checker-backed rules, run each against the repo, and
/// record every outcome in `rule_check_runs`. A rule whose `checker_ref` verb has
/// no discovered command in this repo is recorded as `skipped` (not run). `cwd`
/// is the repo's absolute path.
pub async fn run_checkers(
    pg: &PgStore,
    folder_id: &uuid::Uuid,
    cwd: &Path,
) -> Result<Vec<CheckRun>, String> {
    let rules = pg.resolve_local_checker_rules(folder_id).await?;
    let mut results = Vec::with_capacity(rules.len());
    for (rule_statement, checker_ref) in rules {
        let run = match pg.project_command_for(folder_id, &checker_ref).await? {
            Some(command) => {
                let (verdict, exit_code, out) = run_command(&command, cwd).await;
                pg.insert_check_run(
                    folder_id,
                    &rule_statement,
                    &checker_ref,
                    &command,
                    verdict,
                    exit_code,
                    &out,
                )
                .await?;
                CheckRun {
                    rule_statement,
                    checker_ref,
                    command,
                    verdict: verdict.to_string(),
                    exit_code,
                }
            }
            None => {
                pg.insert_check_run(
                    folder_id,
                    &rule_statement,
                    &checker_ref,
                    "",
                    "skipped",
                    None,
                    "no command discovered for this repo",
                )
                .await?;
                CheckRun {
                    rule_statement,
                    checker_ref,
                    command: String::new(),
                    verdict: "skipped".to_string(),
                    exit_code: None,
                }
            }
        };
        results.push(run);
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verdict_maps_exit_code() {
        assert_eq!(verdict_from_exit(Some(0)), "pass");
        assert_eq!(verdict_from_exit(Some(1)), "fail");
        assert_eq!(verdict_from_exit(Some(101)), "fail");
        assert_eq!(verdict_from_exit(None), "error");
    }

    #[test]
    fn tail_keeps_last_bytes_on_char_boundary() {
        assert_eq!(tail("hello"), "hello");
        let long = "a".repeat(OUTPUT_TAIL_BYTES + 500);
        let t = tail(&long);
        assert!(t.starts_with('…'), "truncated output is marked with an ellipsis");
        assert!(t.len() <= OUTPUT_TAIL_BYTES + 4, "tail is bounded");
    }

    #[tokio::test]
    async fn run_command_captures_pass_fail_and_spawn_error() {
        let cwd = std::path::Path::new("/tmp");
        let (v, code, _) = run_command("true", cwd).await;
        assert_eq!(v, "pass");
        assert_eq!(code, Some(0));

        let (v, _, _) = run_command("false", cwd).await;
        assert_eq!(v, "fail");

        let (v, _, out) = run_command("definitely-not-a-real-binary-xyz", cwd).await;
        assert_eq!(v, "error");
        assert!(out.contains("spawn failed"));
    }
}
