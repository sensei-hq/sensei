//! The daemon's service manager (#505, design D2).
//!
//! macOS and Linux: Homebrew services, which hands the daemon to launchd or
//! systemd with keep-alive. Windows: a per-user Task Scheduler task that starts
//! the daemon when the user signs in. A Windows Service is not used: it runs
//! under a service account, whose Credential Manager entries and `~/.sensei`
//! are not the ones the CLI and the app see.
//!
//! The Windows task runs `senseid start`, which spawns the daemon without a
//! console window and exits, so the task itself finishes at once.

use crate::config::SenseiConfig;
#[cfg(unix)]
use crate::health::resolvers::brew_helpers::{BrewError, brew_services_start};
use std::fmt;

/// Name of the Windows logon task. At the root of the Task Scheduler library:
/// a standard user may register a task there, but creating a folder needs an
/// administrator.
pub const WINDOWS_TASK_NAME: &str = "sensei-senseid";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ServiceError {
    /// The service manager isn't here: no `brew` on PATH, or no `senseid` to register.
    Unavailable(String),
    /// The service manager ran and refused.
    Failed(String),
}

impl fmt::Display for ServiceError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ServiceError::Unavailable(m) | ServiceError::Failed(m) => f.write_str(m),
        }
    }
}

/// What a successful [`stop`] has done.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stopped {
    /// The daemon is stopped and will not start again on its own (`brew services stop`).
    Daemon,
    /// Only the autostart is gone (the Windows logon task); the caller still
    /// stops the running daemon, with `senseid stop`.
    AutostartOnly,
}

/// Start the daemon under the service manager, registering it first where that is needed.
pub fn start(port: u16) -> Result<(), ServiceError> {
    platform::start(port)
}

/// Stop the daemon's service. See [`Stopped`] for what is left to the caller.
pub fn stop() -> Result<Stopped, ServiceError> {
    platform::stop()
}

/// Restart the daemon under the service manager.
pub fn restart(port: u16) -> Result<(), ServiceError> {
    platform::restart(port)
}

/// The command a person runs to start the daemon by hand, for messages and remedies.
pub fn start_hint() -> String {
    platform::start_hint()
}

#[cfg(unix)]
mod platform {
    use super::*;
    use crate::util::which_binary;
    use std::process::Command;

    fn service() -> &'static str {
        SenseiConfig::from_env().brew_service_name()
    }

    /// The port is the formula's: its `service do` block starts the daemon.
    pub fn start(_port: u16) -> Result<(), ServiceError> {
        brew_services_start(service()).map_err(|e| match e {
            BrewError::BrewNotFound => ServiceError::Unavailable("brew is not on PATH".into()),
            other => ServiceError::Failed(format!("brew services start: {other:?}")),
        })
    }

    pub fn stop() -> Result<Stopped, ServiceError> {
        brew_services(&["stop", service()]).map(|()| Stopped::Daemon)
    }

    pub fn restart(_port: u16) -> Result<(), ServiceError> {
        brew_services(&["restart", service()])
    }

    pub fn start_hint() -> String {
        format!("brew services start {}", service())
    }

    fn brew_services(args: &[&str]) -> Result<(), ServiceError> {
        let brew = which_binary("brew")
            .ok_or_else(|| ServiceError::Unavailable("brew is not on PATH".into()))?;
        let out =
            Command::new(brew).arg("services").args(args).output().map_err(|e| {
                ServiceError::Failed(format!("brew services {}: {e}", args.join(" ")))
            })?;
        if out.status.success() {
            return Ok(());
        }
        Err(ServiceError::Failed(format!(
            "brew services {} exited {}: {}",
            args.join(" "),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        )))
    }
}

#[cfg(windows)]
mod platform {
    use super::*;
    use crate::util::which_binary;
    use std::process::Command;

    pub fn start(port: u16) -> Result<(), ServiceError> {
        let bin = SenseiConfig::from_env().senseid_binary();
        let exe = which_binary(bin)
            .ok_or_else(|| ServiceError::Unavailable(format!("{bin} is not on PATH")))?;
        let user = logon_user(
            std::env::var("USERDOMAIN").ok().as_deref(),
            std::env::var("USERNAME").ok().as_deref(),
        )
        .ok_or_else(|| ServiceError::Failed("USERNAME is not set".into()))?;

        let xml_path = std::env::temp_dir().join(format!("{WINDOWS_TASK_NAME}.xml"));
        std::fs::write(&xml_path, utf16_with_bom(&task_xml(&exe, port, &user)))
            .map_err(|e| ServiceError::Failed(format!("writing {}: {e}", xml_path.display())))?;
        let created = schtasks(&create_args(&xml_path.to_string_lossy()));
        if let Err(e) = std::fs::remove_file(&xml_path) {
            tracing::warn!(error = %e, path = %xml_path.display(), "could not remove the task definition");
        }
        created?;
        schtasks(&run_args())
    }

    pub fn stop() -> Result<Stopped, ServiceError> {
        if registered()? {
            schtasks(&delete_args())?;
        }
        Ok(Stopped::AutostartOnly)
    }

    /// Stop the running daemon (a daemon that was not running is fine here),
    /// then start it through the task again.
    pub fn restart(port: u16) -> Result<(), ServiceError> {
        let bin = SenseiConfig::from_env().senseid_binary();
        let exe = which_binary(bin)
            .ok_or_else(|| ServiceError::Unavailable(format!("{bin} is not on PATH")))?;
        Command::new(exe)
            .arg("stop")
            .status()
            .map_err(|e| ServiceError::Failed(format!("{bin} stop: {e}")))?;
        std::thread::sleep(std::time::Duration::from_millis(500));
        start(port)
    }

    pub fn start_hint() -> String {
        "sensei start".into()
    }

    fn registered() -> Result<bool, ServiceError> {
        Command::new("schtasks")
            .args(query_args())
            .output()
            .map(|o| o.status.success())
            .map_err(|e| ServiceError::Unavailable(format!("schtasks: {e}")))
    }

    fn schtasks(args: &[String]) -> Result<(), ServiceError> {
        let out = Command::new("schtasks")
            .args(args)
            .output()
            .map_err(|e| ServiceError::Unavailable(format!("schtasks: {e}")))?;
        if out.status.success() {
            return Ok(());
        }
        Err(ServiceError::Failed(format!(
            "schtasks {} exited {}: {}",
            args.first().map(String::as_str).unwrap_or(""),
            out.status,
            String::from_utf8_lossy(&out.stderr).trim()
        )))
    }
}

// ── The Windows task, as pure functions so every host tests them ─────────────

/// `DOMAIN\user`, or the bare user when no domain is set. `None` without a user:
/// the logon trigger must name the account it belongs to.
#[cfg_attr(unix, allow(dead_code))]
fn logon_user(domain: Option<&str>, user: Option<&str>) -> Option<String> {
    let user = user.map(str::trim).filter(|u| !u.is_empty())?;
    Some(match domain.map(str::trim).filter(|d| !d.is_empty()) {
        Some(d) => format!("{d}\\{user}"),
        None => user.to_string(),
    })
}

/// The task definition. The trigger and principal name the signing-in user, so
/// a standard user may register it and it runs with that user's token (their
/// Credential Manager, their `~/.sensei`). `ExecutionTimeLimit` is unlimited:
/// the default (72 hours) ends whatever the task is still running.
#[cfg_attr(unix, allow(dead_code))]
fn task_xml(exe: &str, port: u16, user: &str) -> String {
    use crate::util::escape_markup as esc;
    let (exe, user) = (esc(exe), esc(user));
    format!(
        r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.2" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo>
    <Description>Starts the sensei daemon (senseid) when you sign in.</Description>
  </RegistrationInfo>
  <Triggers>
    <LogonTrigger>
      <Enabled>true</Enabled>
      <UserId>{user}</UserId>
    </LogonTrigger>
  </Triggers>
  <Principals>
    <Principal id="Author">
      <UserId>{user}</UserId>
      <LogonType>InteractiveToken</LogonType>
      <RunLevel>LeastPrivilege</RunLevel>
    </Principal>
  </Principals>
  <Settings>
    <MultipleInstancesPolicy>IgnoreNew</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Enabled>true</Enabled>
  </Settings>
  <Actions Context="Author">
    <Exec>
      <Command>{exe}</Command>
      <Arguments>start --port {port}</Arguments>
    </Exec>
  </Actions>
</Task>
"#
    )
}

/// `schtasks /Create /XML` reads the encoding the declaration names; UTF-16 is
/// the one it always accepts.
#[cfg_attr(unix, allow(dead_code))]
fn utf16_with_bom(s: &str) -> Vec<u8> {
    std::iter::once(0xFEFF_u16).chain(s.encode_utf16()).flat_map(u16::to_le_bytes).collect()
}

#[cfg_attr(unix, allow(dead_code))]
fn create_args(xml_path: &str) -> Vec<String> {
    ["/Create", "/TN", WINDOWS_TASK_NAME, "/XML", xml_path, "/F"].map(String::from).to_vec()
}

#[cfg_attr(unix, allow(dead_code))]
fn run_args() -> Vec<String> {
    ["/Run", "/TN", WINDOWS_TASK_NAME].map(String::from).to_vec()
}

#[cfg_attr(unix, allow(dead_code))]
fn query_args() -> Vec<String> {
    ["/Query", "/TN", WINDOWS_TASK_NAME].map(String::from).to_vec()
}

#[cfg_attr(unix, allow(dead_code))]
fn delete_args() -> Vec<String> {
    ["/Delete", "/TN", WINDOWS_TASK_NAME, "/F"].map(String::from).to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logon_user_joins_domain_and_user() {
        assert_eq!(logon_user(Some("WORK"), Some("ana")).as_deref(), Some("WORK\\ana"));
    }

    #[test]
    fn logon_user_without_a_domain_is_the_bare_user() {
        assert_eq!(logon_user(None, Some("ana")).as_deref(), Some("ana"));
        assert_eq!(logon_user(Some(" "), Some("ana")).as_deref(), Some("ana"));
    }

    #[test]
    fn logon_user_needs_a_user() {
        assert_eq!(logon_user(Some("WORK"), None), None);
        assert_eq!(logon_user(Some("WORK"), Some("")), None);
    }

    #[test]
    fn task_runs_senseid_start_on_the_port_as_the_user_at_their_logon() {
        let xml = task_xml(r"C:\Program Files\sensei\senseid.exe", 7744, r"WORK\ana");
        assert!(xml.contains(r"<Command>C:\Program Files\sensei\senseid.exe</Command>"), "{xml}");
        assert!(xml.contains("<Arguments>start --port 7744</Arguments>"), "{xml}");
        assert!(
            xml.contains(
                "<LogonTrigger>\n      <Enabled>true</Enabled>\n      <UserId>WORK\\ana</UserId>"
            ),
            "the trigger is the user's own logon: {xml}"
        );
        assert!(xml.contains("<LogonType>InteractiveToken</LogonType>"), "{xml}");
        assert!(xml.contains("<RunLevel>LeastPrivilege</RunLevel>"), "{xml}");
        assert!(xml.contains("<ExecutionTimeLimit>PT0S</ExecutionTimeLimit>"), "{xml}");
    }

    #[test]
    fn task_escapes_the_path_and_user() {
        let xml = task_xml(r"C:\R&D\senseid.exe", 1, "a<b");
        assert!(xml.contains(r"<Command>C:\R&amp;D\senseid.exe</Command>"), "{xml}");
        assert!(xml.contains("<UserId>a&lt;b</UserId>"), "{xml}");
        assert!(!xml.contains("R&D"), "{xml}");
    }

    #[test]
    fn task_definition_is_utf16_le_with_a_bom() {
        assert_eq!(utf16_with_bom("<a/>"), vec![0xFF, 0xFE, b'<', 0, b'a', 0, b'/', 0, b'>', 0]);
    }

    #[test]
    fn schtasks_arguments_name_the_task() {
        assert_eq!(
            create_args(r"C:\t\x.xml"),
            ["/Create", "/TN", "sensei-senseid", "/XML", r"C:\t\x.xml", "/F"]
        );
        assert_eq!(run_args(), ["/Run", "/TN", "sensei-senseid"]);
        assert_eq!(query_args(), ["/Query", "/TN", "sensei-senseid"]);
        assert_eq!(delete_args(), ["/Delete", "/TN", "sensei-senseid", "/F"]);
    }
}
