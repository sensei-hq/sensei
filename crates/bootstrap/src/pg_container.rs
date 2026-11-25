//! The `sensei-pg` Postgres container on Windows (#505, design D3).
//!
//! On Windows, Postgres is the `pgvector/pgvector:pg17` image in a container
//! named `sensei-pg`, published on 5432 with a named data volume. The
//! Postgres command-line tools the database setup uses (`psql`, `createdb`,
//! `pg_isready`) run inside it with `docker exec`, so the host needs none of
//! them; [`pg_tool`] is the one place that decides. The daemon connects to the
//! published port and never starts or stops the container — `sensei doctor
//! --fix` does.

use crate::health::process_util::{TimedOutcome, output_with_timeout};
use std::process::Command;
use std::time::Duration;

pub const CONTAINER: &str = "sensei-pg";
pub const IMAGE: &str = "pgvector/pgvector:pg17";
pub const VOLUME: &str = "sensei-pgdata";

/// A Postgres client tool, ready for its arguments: the host's own on macOS
/// and Linux, the container's on Windows. The credentials reach the
/// container's tool through `docker exec -e`, which forwards their values from
/// this process's environment without putting them on a command line.
pub fn pg_tool(tool: &str) -> Result<Command, String> {
    #[cfg(windows)]
    {
        let mut cmd = crate::util::command_for("docker")?;
        cmd.args(exec_args(tool));
        Ok(cmd)
    }
    #[cfg(not(windows))]
    crate::util::command_for(tool)
}

#[cfg_attr(not(windows), allow(dead_code))]
fn exec_args(tool: &str) -> Vec<String> {
    let mut args = vec!["exec".to_string()];
    for key in crate::credentials::KEYS {
        args.extend(["-e".to_string(), key.to_string()]);
    }
    args.extend([CONTAINER.to_string(), tool.to_string()]);
    args
}

/// `docker run` for a new container. The credentials are named, not valued:
/// `-e NAME` takes the value from the environment `docker` runs with.
pub fn run_args() -> Vec<String> {
    let mut args: Vec<String> =
        ["run", "-d", "--name", CONTAINER, "--restart", "unless-stopped"].map(String::from).into();
    args.extend(["-p".into(), format!("{port}:{port}", port = crate::config::POSTGRES_PORT)]);
    args.extend(["-v".into(), format!("{VOLUME}:/var/lib/postgresql/data")]);
    args.extend(["-e", "POSTGRES_USER", "-e", "POSTGRES_PASSWORD"].map(String::from));
    args.push(IMAGE.into());
    args
}

pub fn start_args() -> Vec<String> {
    ["start", CONTAINER].map(String::from).into()
}

/// One `docker` invocation's result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Run {
    pub ok: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Runs `docker` with these arguments. `Err` when it could not run at all.
pub type Runner = fn(&[&str]) -> Result<Run, String>;

/// The real [`Runner`], bounded so a wedged engine cannot hang a health check.
pub fn docker(args: &[&str]) -> Result<Run, String> {
    let mut cmd = crate::util::command_for("docker")?;
    cmd.args(args);
    match output_with_timeout(cmd, Duration::from_secs(15)) {
        TimedOutcome::Done(o) => Ok(Run {
            ok: o.status.success(),
            stdout: String::from_utf8_lossy(&o.stdout).trim().to_string(),
            stderr: String::from_utf8_lossy(&o.stderr).trim().to_string(),
        }),
        TimedOutcome::TimedOut => Err(format!("docker {} timed out", args.join(" "))),
        TimedOutcome::Failed(e) => Err(format!("docker: {e}")),
    }
}

/// Where the container stands, checked in order: the CLI, the engine, the container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// No `docker` to run.
    NoDocker(String),
    /// The CLI is there but its engine does not answer.
    EngineDown(String),
    Missing,
    Stopped,
    Running,
    /// Docker answered with something none of the above explains.
    Unknown(String),
}

pub fn state(run: Runner) -> State {
    if let Err(e) = run(&["--version"]) {
        return State::NoDocker(e);
    }
    match run(&["info", "--format", "{{.ServerVersion}}"]) {
        Ok(r) if r.ok => {}
        Ok(r) => return State::EngineDown(r.stderr),
        Err(e) => return State::EngineDown(e),
    }
    match run(&["inspect", "--format", "{{.State.Running}}", CONTAINER]) {
        Ok(r) if r.ok && r.stdout == "true" => State::Running,
        Ok(r) if r.ok => State::Stopped,
        Ok(r) if r.stderr.contains("No such") => State::Missing,
        Ok(r) => State::Unknown(r.stderr),
        Err(e) => State::Unknown(e),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok(stdout: &str) -> Result<Run, String> {
        Ok(Run { ok: true, stdout: stdout.into(), stderr: String::new() })
    }
    fn fail(stderr: &str) -> Result<Run, String> {
        Ok(Run { ok: false, stdout: String::new(), stderr: stderr.into() })
    }

    #[test]
    fn no_docker_cli() {
        assert_eq!(
            state(|_| Err("docker not installed".into())),
            State::NoDocker("docker not installed".into())
        );
    }

    #[test]
    fn engine_not_running() {
        let s = state(|a| {
            if a[0] == "info" { fail("Cannot connect to the Docker daemon") } else { ok("") }
        });
        assert_eq!(s, State::EngineDown("Cannot connect to the Docker daemon".into()));
    }

    #[test]
    fn container_missing() {
        let s = state(|a| match a[0] {
            "inspect" => fail("Error: No such object: sensei-pg"),
            _ => ok("27.0"),
        });
        assert_eq!(s, State::Missing);
    }

    #[test]
    fn container_stopped() {
        assert_eq!(
            state(|a| if a[0] == "inspect" { ok("false") } else { ok("27.0") }),
            State::Stopped
        );
    }

    #[test]
    fn container_running() {
        assert_eq!(
            state(|a| if a[0] == "inspect" { ok("true") } else { ok("27.0") }),
            State::Running
        );
    }

    #[test]
    fn an_unexplained_inspect_failure_is_not_taken_for_missing() {
        let s = state(|a| if a[0] == "inspect" { fail("permission denied") } else { ok("27.0") });
        assert_eq!(s, State::Unknown("permission denied".into()));
    }

    #[test]
    fn inspect_asks_about_the_sensei_container() {
        let s = state(|a| match a {
            ["inspect", "--format", "{{.State.Running}}", "sensei-pg"] => ok("true"),
            ["inspect", ..] => fail("wrong container"),
            _ => ok("27.0"),
        });
        assert_eq!(s, State::Running);
    }

    #[test]
    fn exec_forwards_the_credentials_by_name() {
        assert_eq!(
            exec_args("psql"),
            ["exec", "-e", "PGUSER", "-e", "PGPASSWORD", "sensei-pg", "psql"]
        );
    }

    #[test]
    fn run_publishes_5432_with_a_named_volume_and_names_the_credentials() {
        assert_eq!(
            run_args(),
            [
                "run",
                "-d",
                "--name",
                "sensei-pg",
                "--restart",
                "unless-stopped",
                "-p",
                "5432:5432",
                "-v",
                "sensei-pgdata:/var/lib/postgresql/data",
                "-e",
                "POSTGRES_USER",
                "-e",
                "POSTGRES_PASSWORD",
                "pgvector/pgvector:pg17"
            ]
        );
        assert!(!run_args().iter().any(|a| a.contains('=')), "no value on the command line");
    }

    #[test]
    fn start_starts_the_sensei_container() {
        assert_eq!(start_args(), ["start", "sensei-pg"]);
    }
}
