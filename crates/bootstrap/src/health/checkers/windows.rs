//! The Windows prerequisites' checkers (#505): the `sensei-pg` container, the
//! database credentials in `~/.sensei/.env`, and Git for Windows' `sh`.
//! Compiled everywhere so every host runs their tests.

use crate::health::checker::{CheckOutcome, Checker};
use crate::pg_container::{CONTAINER, Runner, State, state};
use std::path::PathBuf;

/// The Docker CLI, its engine, and the `sensei-pg` container running.
pub struct DockerPostgresChecker {
    pub run: Runner,
}

impl Checker for DockerPostgresChecker {
    fn check(&self) -> CheckOutcome {
        match state(self.run) {
            State::Running => CheckOutcome::ready_no_version(),
            other => CheckOutcome::failed(describe(&other)),
        }
    }
}

/// What a state that is not `Running` means to a person.
pub(crate) fn describe(state: &State) -> String {
    match state {
        State::Running => format!("container {CONTAINER} is running"),
        State::NoDocker(e) => format!("Docker is not installed ({e})"),
        State::EngineDown(e) => format!("the Docker engine is not running ({e})"),
        State::Missing => format!("container {CONTAINER} does not exist"),
        State::Stopped => format!("container {CONTAINER} is stopped"),
        State::Unknown(e) => format!("docker could not report on {CONTAINER}: {e}"),
    }
}

/// `PGUSER` and `PGPASSWORD`, from the environment or `~/.sensei/.env`.
pub struct CredentialsChecker {
    pub env_file: Option<PathBuf>,
}

impl Checker for CredentialsChecker {
    fn check(&self) -> CheckOutcome {
        let Some(path) = &self.env_file else {
            return CheckOutcome::failed("no home directory, so ~/.sensei/.env cannot be found");
        };
        match crate::credentials::missing(path) {
            None => CheckOutcome::ready_no_version(),
            Some(problem) => CheckOutcome::failed(problem),
        }
    }
}

/// The POSIX `sh` sensei's hooks run under: Git for Windows' on Windows.
pub struct PosixShellChecker;

impl Checker for PosixShellChecker {
    fn check(&self) -> CheckOutcome {
        match crate::util::posix_shell() {
            Ok(_) => CheckOutcome::ready_no_version(),
            Err(e) => CheckOutcome::failed(e),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health::types::ComponentStatus;
    use crate::pg_container::Run;

    fn ok(stdout: &str) -> Result<Run, String> {
        Ok(Run { ok: true, stdout: stdout.into(), stderr: String::new() })
    }

    #[test]
    fn a_running_container_is_ready() {
        let c = DockerPostgresChecker {
            run: |a| if a[0] == "inspect" { ok("true") } else { ok("27") },
        };
        assert!(matches!(c.check().status, ComponentStatus::Ready));
    }

    #[test]
    fn a_stopped_container_fails_naming_it() {
        let c = DockerPostgresChecker {
            run: |a| if a[0] == "inspect" { ok("false") } else { ok("27") },
        };
        let o = c.check();
        assert!(matches!(o.status, ComponentStatus::Failed));
        assert_eq!(o.detail.as_deref(), Some("container sensei-pg is stopped"));
    }

    #[test]
    fn docker_down_fails_on_docker_not_the_container() {
        let c = DockerPostgresChecker {
            run: |a| match a[0] {
                "info" => {
                    Ok(Run { ok: false, stdout: String::new(), stderr: "pipe not found".into() })
                }
                "inspect" => ok("true"),
                _ => ok("27"),
            },
        };
        let o = c.check();
        assert!(matches!(o.status, ComponentStatus::Failed));
        assert!(o.detail.unwrap().starts_with("the Docker engine is not running"));
    }

    #[test]
    fn no_docker_fails_naming_docker() {
        let c = DockerPostgresChecker { run: |_| Err("docker not installed".into()) };
        assert_eq!(
            c.check().detail.as_deref(),
            Some("Docker is not installed (docker not installed)")
        );
    }

    #[test]
    fn credentials_in_the_file_are_ready() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env");
        std::fs::write(&path, "PGUSER=sensei\nPGPASSWORD=s3cret\n").unwrap();
        let o = CredentialsChecker { env_file: Some(path) }.check();
        assert!(matches!(o.status, ComponentStatus::Ready), "{:?}", o.detail);
    }

    #[test]
    fn a_malformed_file_fails_without_showing_its_contents() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env");
        std::fs::write(&path, "PGUSER=sensei\ns3cret\n").unwrap();
        let o = CredentialsChecker { env_file: Some(path) }.check();
        assert!(matches!(o.status, ComponentStatus::Failed));
        let detail = o.detail.unwrap();
        assert!(detail.contains("line 2"), "{detail}");
        assert!(!detail.contains("s3cret"), "{detail}");
    }

    #[test]
    fn no_home_directory_fails() {
        let o = CredentialsChecker { env_file: None }.check();
        assert!(matches!(o.status, ComponentStatus::Failed));
    }

    #[cfg(unix)]
    #[test]
    fn sh_is_found_on_unix() {
        assert!(matches!(PosixShellChecker.check().status, ComponentStatus::Ready));
    }
}
