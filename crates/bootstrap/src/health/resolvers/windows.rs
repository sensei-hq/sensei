//! The Windows prerequisites' resolvers (#505): the `sensei-pg` container,
//! winget installs (Ollama, Git for Windows), and the database setup behind a
//! credentials check. Compiled everywhere so every host runs their tests.

use crate::config::SenseiConfig;
use crate::health::checkers::windows::describe;
use crate::health::process_util::{TimedOutcome, output_with_timeout};
use crate::health::resolver::{ResolveOutcome, Resolver};
use crate::health::resolvers::DatabaseResolver;
use crate::health::types::{ComponentId, Remedy};
use crate::pg_container::{CONTAINER, Runner, State, run_args, start_args, state};
use std::time::Duration;

/// How long a started container may take to accept connections (`initdb` on
/// first run included).
const READY_WAIT: Duration = Duration::from_secs(60);

/// Creates `sensei-pg` when it is missing and starts it when it is stopped.
/// Docker itself it cannot install or start; those are remedies.
pub struct DockerPostgresResolver {
    pub run: Runner,
}

impl Resolver for DockerPostgresResolver {
    fn id(&self) -> &'static str {
        "docker_postgres"
    }
    fn resolves(&self) -> &'static [ComponentId] {
        &[ComponentId::Postgres]
    }

    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        let current = state(self.run);
        match current {
            State::Running => return ResolveOutcome::Resolved,
            State::NoDocker(_) => return ResolveOutcome::NeedsHumanAction(install_docker_remedy()),
            State::EngineDown(_) => return ResolveOutcome::NeedsHumanAction(start_docker_remedy()),
            State::Unknown(_) => {
                return ResolveOutcome::NeedsHumanAction(container_remedy(&describe(&current)));
            }
            State::Stopped => {
                let args = start_args();
                let args: Vec<&str> = args.iter().map(String::as_str).collect();
                match (self.run)(&args) {
                    Ok(r) if r.ok => {}
                    Ok(r) => return ResolveOutcome::NeedsHumanAction(container_remedy(&r.stderr)),
                    Err(e) => return ResolveOutcome::NeedsHumanAction(container_remedy(&e)),
                }
            }
            State::Missing => {
                if let Some(problem) = credentials_problem() {
                    return ResolveOutcome::NeedsHumanAction(credentials_remedy(&problem));
                }
                if let Err(e) = create_container() {
                    return ResolveOutcome::NeedsHumanAction(container_remedy(&e));
                }
            }
        }
        wait_until_ready(self.run, READY_WAIT)
    }

    fn fallback_remedy(&self) -> Remedy {
        container_remedy("Postgres is still not answering on port 5432")
    }
}

/// `docker run` with the credentials handed over through the environment
/// (`POSTGRES_USER`/`POSTGRES_PASSWORD` from `PGUSER`/`PGPASSWORD`), so the
/// password is never on a command line. Bounded generously: the first run
/// pulls the image.
fn create_container() -> Result<(), String> {
    let mut cmd = crate::util::command_for("docker")?;
    cmd.args(run_args());
    for (from, to) in [("PGUSER", "POSTGRES_USER"), ("PGPASSWORD", "POSTGRES_PASSWORD")] {
        let value = std::env::var(from).map_err(|_| format!("{from} is not set"))?;
        cmd.env(to, value);
    }
    match output_with_timeout(cmd, Duration::from_secs(15 * 60)) {
        TimedOutcome::Done(o) if o.status.success() => Ok(()),
        TimedOutcome::Done(o) => Err(String::from_utf8_lossy(&o.stderr).trim().to_string()),
        TimedOutcome::TimedOut => Err("docker run timed out after 15 minutes".into()),
        TimedOutcome::Failed(e) => Err(format!("docker: {e}")),
    }
}

/// Poll the container's own `pg_isready` until it answers or `limit` passes.
fn wait_until_ready(run: Runner, limit: Duration) -> ResolveOutcome {
    let deadline = std::time::Instant::now() + limit;
    loop {
        if (run)(&["exec", CONTAINER, "pg_isready", "--quiet"]).is_ok_and(|r| r.ok) {
            return ResolveOutcome::Resolved;
        }
        if std::time::Instant::now() >= deadline {
            return ResolveOutcome::NeedsHumanAction(container_remedy(&format!(
                "{CONTAINER} did not accept connections within {}s",
                limit.as_secs()
            )));
        }
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn credentials_problem() -> Option<String> {
    match crate::credentials::env_file() {
        Some(path) => crate::credentials::missing(&path),
        None => Some("no home directory, so ~/.sensei/.env cannot be found".into()),
    }
}

/// Postgres's `DatabaseResolver`, once the credentials it connects with exist.
pub struct CredentialedDatabaseResolver {
    pub inner: DatabaseResolver,
}

impl Resolver for CredentialedDatabaseResolver {
    fn id(&self) -> &'static str {
        self.inner.id()
    }
    fn resolves(&self) -> &'static [ComponentId] {
        self.inner.resolves()
    }
    fn resolve(&self, targets: &[ComponentId]) -> ResolveOutcome {
        match credentials_problem() {
            Some(problem) => ResolveOutcome::NeedsHumanAction(credentials_remedy(&problem)),
            None => self.inner.resolve(targets),
        }
    }
    fn fallback_remedy(&self) -> Remedy {
        self.inner.fallback_remedy()
    }
}

/// Installs one winget package, for one component.
pub struct WingetInstallResolver {
    pub id: &'static str,
    pub component: &'static [ComponentId],
    pub package: &'static str,
    pub name: &'static str,
}

impl Resolver for WingetInstallResolver {
    fn id(&self) -> &'static str {
        self.id
    }
    fn resolves(&self) -> &'static [ComponentId] {
        self.component
    }
    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        winget_install(self.package, self.name)
    }
    fn fallback_remedy(&self) -> Remedy {
        winget_remedy(self.package, self.name, "it is installed but not yet answering")
    }
}

/// The sensei component on Windows: installs Git for Windows when its `sh` is
/// what is missing; the sensei binaries themselves come from the release.
pub struct SenseiWindowsResolver;

impl Resolver for SenseiWindowsResolver {
    fn id(&self) -> &'static str {
        "sensei_windows"
    }
    fn resolves(&self) -> &'static [ComponentId] {
        &[ComponentId::Sensei]
    }
    fn resolve(&self, _targets: &[ComponentId]) -> ResolveOutcome {
        if crate::util::posix_shell().is_err() {
            return winget_install(GIT_PACKAGE, "Git for Windows");
        }
        ResolveOutcome::NeedsHumanAction(self.fallback_remedy())
    }
    fn fallback_remedy(&self) -> Remedy {
        let cfg = SenseiConfig::from_env();
        Remedy {
            message: format!(
                "The sensei binaries ({}, {}, {}) are not on PATH. Install them from the Windows release and add its folder to PATH.",
                cfg.sensei_binary(),
                cfg.senseid_binary(),
                cfg.sensei_mcp_binary()
            ),
            script: "start https://github.com/sensei-hq/sensei/releases/latest".into(),
            url: Some("https://github.com/sensei-hq/sensei/releases/latest".into()),
        }
    }
}

pub const GIT_PACKAGE: &str = "Git.Git";
pub const OLLAMA_PACKAGE: &str = "Ollama.Ollama";

fn winget_args(package: &str) -> Vec<String> {
    ["install", "--id", package, "-e", "--accept-source-agreements", "--accept-package-agreements"]
        .map(String::from)
        .into()
}

fn winget_install(package: &str, name: &str) -> ResolveOutcome {
    let mut cmd = match crate::util::command_for("winget") {
        Ok(c) => c,
        Err(e) => return ResolveOutcome::NeedsHumanAction(winget_remedy(package, name, &e)),
    };
    cmd.args(winget_args(package));
    match output_with_timeout(cmd, Duration::from_secs(15 * 60)) {
        TimedOutcome::Done(o) if o.status.success() => ResolveOutcome::Resolved,
        TimedOutcome::Done(o) => ResolveOutcome::NeedsHumanAction(winget_remedy(
            package,
            name,
            &format!("winget exited {}: {}", o.status, String::from_utf8_lossy(&o.stdout).trim()),
        )),
        TimedOutcome::TimedOut => ResolveOutcome::NeedsHumanAction(winget_remedy(
            package,
            name,
            "winget did not finish within 15 minutes",
        )),
        TimedOutcome::Failed(e) => {
            ResolveOutcome::NeedsHumanAction(winget_remedy(package, name, &e.to_string()))
        }
    }
}

fn winget_remedy(package: &str, name: &str, detail: &str) -> Remedy {
    Remedy {
        message: format!(
            "Couldn't install {name} automatically ({detail}). Run the script below, then `sensei doctor` again."
        ),
        script: format!("winget {}", winget_args(package).join(" ")),
        url: None,
    }
}

fn install_docker_remedy() -> Remedy {
    Remedy {
        message: "Postgres runs in Docker on Windows. Install Docker Desktop (Rancher Desktop or Podman also work), start it, then run `sensei doctor --fix` again.".into(),
        script: "winget install --id Docker.DockerDesktop -e".into(),
        url: Some("https://docs.docker.com/desktop/setup/install/windows-install/".into()),
    }
}

fn start_docker_remedy() -> Remedy {
    Remedy {
        message: "Docker is installed but its engine is not running. Start Docker Desktop, wait until it reports the engine running, then run `sensei doctor --fix` again.".into(),
        script: "docker desktop start".into(),
        url: None,
    }
}

fn credentials_remedy(problem: &str) -> Remedy {
    Remedy {
        message: format!(
            "{problem}. Put two lines in the file — PGUSER=<a user name> and PGPASSWORD=<a password you choose> — then run `sensei doctor --fix` again. The container is created with these, and sensei connects with them."
        ),
        script: r#"New-Item -ItemType Directory -Force "$HOME\.sensei" | Out-Null; notepad "$HOME\.sensei\.env""#.into(),
        url: None,
    }
}

fn container_remedy(detail: &str) -> Remedy {
    Remedy {
        message: format!(
            "The {CONTAINER} Postgres container needs attention: {detail}. Its log usually says why."
        ),
        script: format!("docker logs --tail 50 {CONTAINER}"),
        url: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pg_container::Run;

    fn ok(stdout: &str) -> Result<Run, String> {
        Ok(Run { ok: true, stdout: stdout.into(), stderr: String::new() })
    }
    fn fail(stderr: &str) -> Result<Run, String> {
        Ok(Run { ok: false, stdout: String::new(), stderr: stderr.into() })
    }
    fn human(o: ResolveOutcome) -> Remedy {
        match o {
            ResolveOutcome::NeedsHumanAction(r) => r,
            ResolveOutcome::Resolved => panic!("expected a remedy"),
        }
    }

    #[test]
    fn a_running_container_needs_nothing() {
        let r = DockerPostgresResolver {
            run: |a| if a[0] == "inspect" { ok("true") } else { ok("27") },
        };
        assert!(matches!(r.resolve(&[]), ResolveOutcome::Resolved));
    }

    #[test]
    fn a_stopped_container_is_started_and_waited_for() {
        let r = DockerPostgresResolver {
            run: |a| match a {
                ["inspect", ..] => ok("false"),
                ["start", "sensei-pg"] => ok("sensei-pg"),
                ["exec", "sensei-pg", "pg_isready", ..] => ok(""),
                ["--version"] | ["info", ..] => ok("27"),
                other => panic!("unexpected docker {other:?}"),
            },
        };
        assert!(matches!(r.resolve(&[]), ResolveOutcome::Resolved));
    }

    #[test]
    fn a_failed_start_is_a_remedy_carrying_dockers_reason() {
        let r = DockerPostgresResolver {
            run: |a| match a[0] {
                "inspect" => ok("false"),
                "start" => fail("port is already allocated"),
                _ => ok("27"),
            },
        };
        let remedy = human(r.resolve(&[]));
        assert!(remedy.message.contains("port is already allocated"), "{}", remedy.message);
    }

    #[test]
    fn no_docker_asks_for_docker_desktop() {
        let r = DockerPostgresResolver { run: |_| Err("docker not installed".into()) };
        assert!(human(r.resolve(&[])).script.contains("Docker.DockerDesktop"));
    }

    #[test]
    fn a_stopped_engine_asks_to_start_docker() {
        let r = DockerPostgresResolver {
            run: |a| if a[0] == "info" { fail("no engine") } else { ok("27") },
        };
        assert_eq!(human(r.resolve(&[])).script, "docker desktop start");
    }

    #[test]
    fn a_container_that_never_answers_is_a_remedy() {
        let o = wait_until_ready(|_| fail("no response"), Duration::ZERO);
        assert!(human(o).message.contains("did not accept connections"));
    }

    #[test]
    fn winget_installs_by_exact_id_without_prompts() {
        assert_eq!(
            winget_args("Ollama.Ollama"),
            [
                "install",
                "--id",
                "Ollama.Ollama",
                "-e",
                "--accept-source-agreements",
                "--accept-package-agreements"
            ]
        );
        assert!(
            winget_remedy(GIT_PACKAGE, "Git for Windows", "x")
                .script
                .starts_with("winget install --id Git.Git -e")
        );
    }

    #[test]
    fn the_credentials_remedy_names_both_keys_and_holds_no_password() {
        let r = credentials_remedy(
            "database credentials missing: set PGPASSWORD in C:\\Users\\a\\.sensei\\.env",
        );
        assert!(r.message.contains("PGUSER=") && r.message.contains("PGPASSWORD="));
        assert!(r.script.contains(r"\.sensei\.env"));
        assert!(!r.script.contains("PGPASSWORD"), "the script opens the file; it sets no value");
    }
}
