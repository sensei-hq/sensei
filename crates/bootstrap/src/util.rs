//! Cross-platform binary discovery.
//!
//! This module exposes a single helper — `which_binary` — used by every
//! sensei consumer (CLI, daemon, MCP, Tauri sidecar). It looks up a binary
//! on PATH, then falls back to well-known package-manager directories so
//! Tauri's macOS .app bundle (which has a narrow process PATH) can still
//! find Homebrew-installed binaries.

use std::process::Command;

/// Directories searched when `which`/`where` lookup misses. Homebrew is
/// sensei's single source of truth — `make install-{dev,release}` and the
/// `claude plugin install sensei` flow both land binaries in the brew
/// prefix. `~/.local/bin` is intentionally NOT scanned: pre-brew install
/// scripts dropped binaries there, and continuing to look in it would
/// keep a stale copy alive after the user upgrades via brew.
const EXTRA_BIN_DIRS: &[&str] = &[
    "/opt/homebrew/bin",              // macOS Apple Silicon Homebrew
    "/usr/local/bin",                 // macOS Intel Homebrew, common Linux
    "/home/linuxbrew/.linuxbrew/bin", // Linuxbrew on dedicated user
];

/// Homebrew "opt" prefixes whose subdirectories hold keg-only formula
/// binaries. Versioned PostgreSQL formulas (`postgresql@17`,
/// `postgresql@16`, …) install keg-only by default — their bins land at
/// `<opt_prefix>/postgresql@<N>/bin` and are *not* symlinked into
/// `/opt/homebrew/bin` unless the user runs `brew link --force`. So
/// even though `pg_isready` happens to be linked on some installs (via
/// an older non-versioned `postgresql` formula or a manual link),
/// `psql` / `createdb` from a keg-only versioned install stay invisible
/// to `which`. Scanning the opt prefix's `postgresql*` subdirs catches
/// them.
const EXTRA_BIN_OPT_PREFIXES: &[&str] = &[
    "/opt/homebrew/opt", // macOS Apple Silicon Homebrew
    "/usr/local/opt",    // macOS Intel Homebrew
];

/// Subdir name prefixes inside an `opt` directory that we'll scan for
/// a missing binary. Restricted to postgres for now — adding more
/// keg-only formulas (e.g. `openssl@3`) is just a matter of listing
/// them here.
const EXTRA_BIN_OPT_SUBDIR_PREFIXES: &[&str] = &["postgresql@", "postgresql"];

/// Per-user tool-install `bin` dirs that live OUTSIDE PATH — resolved at runtime
/// because they hang off `$HOME` / an install-var. `qlty`'s installer lands the
/// binary at `$QLTY_INSTALL/bin` (default `~/.qlty/bin`), which a launchd-managed
/// daemon's minimal PATH (`/opt/homebrew/bin:/usr/local/bin:/usr/bin:/bin`) does
/// NOT include — so `which qlty` misses in the daemon even though it's installed.
/// Listed here so [`which_binary`] finds them (the same reason the Homebrew "opt"
/// dirs are scanned for keg-only postgres).
fn user_local_bin_dirs() -> Vec<String> {
    let mut dirs = Vec::new();
    // `$QLTY_INSTALL/bin` wins (an explicit non-default install), else `~/.qlty/bin`.
    if let Ok(qlty_install) = std::env::var("QLTY_INSTALL")
        && !qlty_install.is_empty()
    {
        dirs.push(format!("{qlty_install}/bin"));
    }
    if let Some(home) = crate::config::try_home_dir() {
        dirs.push(home.join(".qlty").join("bin").to_string_lossy().into_owned());
    }
    dirs
}

/// Why a POSIX shell could not be found — Windows only: Unix always has `sh`.
pub const NO_POSIX_SHELL: &str = "no POSIX shell: install Git for Windows (winget install Git.Git), \
     which provides the sh.exe sensei runs shell commands with";

/// The POSIX shell to run `sh -c` commands with: `sh` on Unix; on Windows Git
/// for Windows' `sh.exe` — its install folders first (the same search as the
/// plugin's `run-hook.cmd`), then `PATH`. A miss is [`NO_POSIX_SHELL`], never a
/// silent fallback to `cmd`.
pub fn posix_shell() -> Result<std::path::PathBuf, String> {
    #[cfg(unix)]
    {
        Ok(std::path::PathBuf::from("sh"))
    }
    #[cfg(windows)]
    {
        let candidates = git_sh_candidates(
            std::env::var_os("ProgramFiles"),
            std::env::var_os("ProgramFiles(x86)"),
        );
        first_existing(&candidates, |p| p.is_file())
            .or_else(|| which_binary("sh").map(std::path::PathBuf::from))
            .ok_or_else(|| NO_POSIX_SHELL.to_string())
    }
}

/// `<Program Files>\Git\bin\sh.exe` for each Program Files folder that is set.
#[cfg_attr(unix, allow(dead_code))]
fn git_sh_candidates(
    program_files: Option<std::ffi::OsString>,
    program_files_x86: Option<std::ffi::OsString>,
) -> Vec<std::path::PathBuf> {
    [program_files, program_files_x86]
        .into_iter()
        .flatten()
        .filter(|d| !d.is_empty())
        .map(|d| std::path::PathBuf::from(d).join("Git").join("bin").join("sh.exe"))
        .collect()
}

#[cfg_attr(unix, allow(dead_code))]
fn first_existing(
    candidates: &[std::path::PathBuf],
    exists: impl Fn(&std::path::Path) -> bool,
) -> Option<std::path::PathBuf> {
    candidates.iter().find(|p| exists(p)).cloned()
}

/// The directory holding `exe`, then its `bin` subdirectory: the installed
/// app sits in the first and sensei's binaries in the second, and a binary in
/// `bin` finds its siblings in the first.
#[cfg_attr(unix, allow(dead_code))]
fn exe_relative_dirs(exe: &std::path::Path) -> Vec<std::path::PathBuf> {
    exe.parent().map(|dir| vec![dir.to_path_buf(), dir.join("bin")]).unwrap_or_default()
}

/// Find a binary on PATH (and well-known directories).
///
/// Uses `which` on Unix and `where` on Windows; falls back to scanning
/// `EXTRA_BIN_DIRS`, per-user tool dirs ([`user_local_bin_dirs`], e.g.
/// `~/.qlty/bin`), and Homebrew "opt" subdirs for keg-only formulas
/// (currently postgresql variants). Returns the full path if found,
/// `None` otherwise.
pub fn which_binary(name: &str) -> Option<String> {
    #[cfg(unix)]
    let cmd = "which";
    #[cfg(windows)]
    let cmd = "where";

    if let Ok(output) = Command::new(cmd).arg(name).output()
        && output.status.success()
    {
        let path =
            String::from_utf8_lossy(&output.stdout).lines().next().unwrap_or("").trim().to_string();
        if !path.is_empty() {
            return Some(path);
        }
    }

    for dir in EXTRA_BIN_DIRS {
        let candidate = format!("{dir}/{name}");
        if std::path::Path::new(&candidate).exists() {
            return Some(candidate);
        }
    }

    // Windows: beside this executable, and in `bin` beside it — where the
    // installer puts sensei's binaries. A process the installer started still
    // has the PATH from before the install.
    #[cfg(windows)]
    if let Ok(exe) = std::env::current_exe() {
        for dir in exe_relative_dirs(&exe) {
            let candidate = dir.join(format!("{name}.exe"));
            if candidate.exists() {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
    }

    // Per-user tool installs off PATH (qlty at ~/.qlty/bin) — the daemon's launchd
    // PATH omits these, so a bare `which` misses them.
    for dir in user_local_bin_dirs() {
        let candidate = format!("{dir}/{name}");
        if std::path::Path::new(&candidate).exists() {
            return Some(candidate);
        }
    }

    // Keg-only Homebrew formulas (versioned postgres in particular).
    // We list directory entries under each opt prefix and only descend
    // into ones whose name starts with one of the documented prefixes,
    // so this stays O(few-dirs) regardless of how many formulas the
    // user has installed.
    for prefix in EXTRA_BIN_OPT_PREFIXES {
        let Ok(entries) = std::fs::read_dir(prefix) else {
            continue;
        };
        for entry in entries.flatten() {
            let entry_name = entry.file_name();
            let entry_name = entry_name.to_string_lossy();
            if !EXTRA_BIN_OPT_SUBDIR_PREFIXES.iter().any(|p| entry_name.starts_with(p)) {
                continue;
            }
            let candidate = entry.path().join("bin").join(name);
            if candidate.exists() {
                return Some(candidate.to_string_lossy().into_owned());
            }
        }
    }

    None
}

/// Build a `Command` for `name` after resolving it via [`which_binary`].
///
/// Callers MUST use this instead of `Command::new(name)` when the binary
/// may live outside the calling process's `PATH` — which is the default
/// situation in the Tauri `.app`'s Finder-launched environment (no
/// `/opt/homebrew/bin`). The resolution uses the same fallback as
/// `which_binary`, and the returned `Command` is invoked by absolute
/// path so the kernel never re-searches PATH at spawn time.
///
/// Returns `Err("<name> not installed")` if the binary isn't on PATH or
/// in any of the fallback directories. That string is end-user readable
/// — bubble it straight up to the remedy / detail field instead of
/// surfacing the raw `std::io::Error` from a failed spawn ("No such
/// file or directory (os error 2)"), which is what every prior code
/// path that used `Command::new(name)` produced.
pub fn command_for(name: &str) -> Result<Command, String> {
    let path = which_binary(name).ok_or_else(|| format!("{name} not installed"))?;
    Ok(Command::new(path))
}

/// The exit code to pass on for a child that has finished: its own code, or —
/// killed by a signal, so it has none — 128 + the signal, as a shell reports it.
/// Never 0 for a child that did not succeed: `code().unwrap_or(0)` turned a
/// killed daemon into a successful `sensei restart`.
pub fn exit_code(status: std::process::ExitStatus) -> i32 {
    if let Some(code) = status.code() {
        return code;
    }
    #[cfg(unix)]
    if let Some(signal) = std::os::unix::process::ExitStatusExt::signal(&status) {
        return 128 + signal;
    }
    1
}

/// Escape text for HTML or XML content and attribute values: `& < > " '`.
pub fn escape_markup(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(c),
        }
    }
    out
}

/// The process with this pid, freshly read, or `None` when no such process exists.
fn live_process(pid: u32) -> Option<sysinfo::System> {
    let pid = sysinfo::Pid::from_u32(pid);
    let mut sys = sysinfo::System::new();
    sys.refresh_processes(sysinfo::ProcessesToUpdate::Some(&[pid]), true);
    sys.process(pid).is_some().then_some(sys)
}

/// Whether a process with this pid exists (an unreaped zombie counts, as with `kill -0`).
pub fn pid_alive(pid: u32) -> bool {
    live_process(pid).is_some()
}

/// Ask the process with this pid to end. On Unix that is SIGTERM, which a
/// daemon answers with a graceful shutdown. Windows has no such request for a
/// process without a window, so there it is terminated outright.
pub fn terminate(pid: u32) -> Result<(), String> {
    let sys = live_process(pid).ok_or_else(|| format!("no process with pid {pid}"))?;
    let process = sys
        .process(sysinfo::Pid::from_u32(pid))
        .ok_or_else(|| format!("no process with pid {pid}"))?;
    #[cfg(unix)]
    let sent = process.kill_with(sysinfo::Signal::Term).unwrap_or(false);
    #[cfg(windows)]
    let sent = process.kill();
    if sent { Ok(()) } else { Err(format!("could not end pid {pid}")) }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    #[test]
    fn git_sh_candidates_cover_both_program_files() {
        let c = git_sh_candidates(
            Some(r"C:\Program Files".into()),
            Some(r"C:\Program Files (x86)".into()),
        );
        assert_eq!(
            c,
            vec![
                PathBuf::from(r"C:\Program Files").join("Git").join("bin").join("sh.exe"),
                PathBuf::from(r"C:\Program Files (x86)").join("Git").join("bin").join("sh.exe"),
            ]
        );
    }

    #[test]
    fn git_sh_candidates_skip_unset_dirs() {
        assert!(git_sh_candidates(None, None).is_empty());
    }

    #[test]
    fn first_existing_picks_the_first_present_candidate() {
        let c = [PathBuf::from("/a/sh.exe"), PathBuf::from("/b/sh.exe")];
        let found = first_existing(&c, |p: &Path| p == Path::new("/b/sh.exe"));
        assert_eq!(found, Some(PathBuf::from("/b/sh.exe")));
        assert_eq!(first_existing(&c, |_: &Path| false), None);
    }

    #[cfg(unix)]
    #[test]
    fn posix_shell_is_sh_on_unix() {
        assert_eq!(posix_shell().unwrap(), PathBuf::from("sh"));
    }

    #[test]
    fn no_shell_error_names_git_for_windows() {
        assert!(NO_POSIX_SHELL.contains("Git for Windows"));
    }

    /// A child killed by a signal has no exit code, and `code().unwrap_or(0)`
    /// reported it as SUCCESS: `sensei restart` exited 0 for a daemon that died.
    /// The shell's convention instead: 128 + the signal.
    #[cfg(unix)]
    #[test]
    fn a_child_killed_by_a_signal_is_not_a_success() {
        let run =
            |script: &str| std::process::Command::new("sh").args(["-c", script]).status().unwrap();
        assert_eq!(exit_code(run("exit 0")), 0);
        assert_eq!(exit_code(run("exit 3")), 3);
        assert_eq!(exit_code(run("kill -9 $$")), 137);
        assert_eq!(exit_code(run("kill -15 $$")), 143);
    }

    use super::*;

    #[test]
    fn which_binary_returns_none_for_missing() {
        assert!(which_binary("definitely_not_a_real_binary_xyz_77").is_none());
    }

    #[test]
    fn which_binary_finds_ls_on_unix() {
        #[cfg(unix)]
        {
            let p = which_binary("ls").expect("ls must exist on unix");
            assert!(p.ends_with("/ls") || p.ends_with("/bin/ls"));
        }
    }

    /// Smoke test for the keg-only Homebrew fallback. Only runs when
    /// `/opt/homebrew/opt/postgresql*/bin/psql` actually exists on the
    /// host — otherwise we'd be testing absence rather than the
    /// fallback path. Skipping silently keeps CI green on non-macOS
    /// boxes and on machines that lack a keg-only postgres install.
    #[test]
    fn which_binary_falls_back_to_homebrew_opt_for_keg_only_postgres() {
        // Locate a real keg-only psql, if any, to gate the assertion.
        let mut keg_only_psql: Option<String> = None;
        for prefix in EXTRA_BIN_OPT_PREFIXES {
            let Ok(entries) = std::fs::read_dir(prefix) else { continue };
            for entry in entries.flatten() {
                let name = entry.file_name();
                let name = name.to_string_lossy().into_owned();
                if EXTRA_BIN_OPT_SUBDIR_PREFIXES.iter().any(|p| name.starts_with(p)) {
                    let candidate = entry.path().join("bin").join("psql");
                    if candidate.exists() {
                        keg_only_psql = Some(candidate.to_string_lossy().into_owned());
                        break;
                    }
                }
            }
            if keg_only_psql.is_some() {
                break;
            }
        }
        let Some(expected) = keg_only_psql else { return };

        // `which_binary("psql")` may resolve to a symlinked
        // /opt/homebrew/bin/psql instead of the opt-dir one when both
        // exist (PATH lookup wins). The assertion is just that
        // *something* is found — i.e., the fallback doesn't fail to
        // surface psql when only the keg-only copy exists.
        let resolved = which_binary("psql").unwrap_or_else(|| {
            panic!(
                "which_binary(\"psql\") returned None even though a keg-only \
                 install exists at {expected}",
            )
        });
        // Sanity: resolution lands on a real psql file.
        assert!(
            std::path::Path::new(&resolved).exists(),
            "resolved path should exist on disk: {resolved}",
        );
    }

    #[test]
    fn user_local_bin_dirs_includes_qlty_home_dir() {
        // The `qlty` install dir (`~/.qlty/bin`) must be a resolution candidate — this
        // is the dir the daemon's launchd PATH omits, which made every qlty scan miss.
        // HOME is always set in the test environment; assert without mutating env.
        if let Some(home) = crate::config::try_home_dir() {
            let want = home.join(".qlty").join("bin").to_string_lossy().into_owned();
            let dirs = user_local_bin_dirs();
            assert!(
                dirs.iter().any(|d| d == &want),
                "user_local_bin_dirs must include ~/.qlty/bin (got {dirs:?})",
            );
        }
    }

    /// Smoke test for the qlty fallback: when `~/.qlty/bin/qlty` actually exists on the
    /// host, `which_binary("qlty")` MUST resolve it (via PATH or the ~/.qlty/bin
    /// fallback) — the resolution the daemon relies on. Gated on the file existing so
    /// CI/machines without qlty stay green (testing the fallback, not absence).
    #[test]
    fn which_binary_resolves_qlty_when_installed_in_user_local_dir() {
        let Ok(home) = std::env::var("HOME") else { return };
        let expected = format!("{home}/.qlty/bin/qlty");
        if !std::path::Path::new(&expected).exists() {
            return; // qlty not installed at the default location on this host — skip.
        }
        let resolved = which_binary("qlty")
            .expect("which_binary(\"qlty\") must resolve when ~/.qlty/bin/qlty exists");
        assert!(
            std::path::Path::new(&resolved).exists(),
            "resolved qlty path should exist on disk: {resolved}",
        );
    }

    #[test]
    fn escape_markup_escapes_the_five_metacharacters() {
        assert_eq!(super::escape_markup(r#"a&b<c>d"e'f"#), "a&amp;b&lt;c&gt;d&quot;e&#39;f");
        assert_eq!(super::escape_markup(r"C:\Program Files\sensei"), r"C:\Program Files\sensei");
    }

    #[test]
    fn this_process_is_alive() {
        assert!(super::pid_alive(std::process::id()));
    }

    /// A child that runs until it is ended, on either platform.
    fn long_running_child() -> std::process::Child {
        #[cfg(unix)]
        let mut cmd = std::process::Command::new("sleep");
        #[cfg(unix)]
        cmd.arg("30");
        #[cfg(windows)]
        let mut cmd = std::process::Command::new("ping");
        #[cfg(windows)]
        cmd.args(["-n", "30", "127.0.0.1"]);
        cmd.stdout(std::process::Stdio::null()).spawn().expect("spawn a long-running child")
    }

    #[test]
    fn terminate_ends_a_running_process_and_it_is_then_not_alive() {
        let mut child = long_running_child();
        let pid = child.id();
        assert!(super::pid_alive(pid));
        super::terminate(pid).expect("terminate a live child");
        let status = child.wait().expect("reap the child");
        assert!(!status.success(), "an ended child does not report success");
        assert!(!super::pid_alive(pid), "a reaped child is gone");
    }

    #[test]
    fn terminate_names_a_pid_with_no_process() {
        let mut child = long_running_child();
        let pid = child.id();
        child.kill().expect("kill");
        child.wait().expect("reap");
        let err = super::terminate(pid).expect_err("nothing left to end");
        assert!(err.contains(&pid.to_string()), "{err}");
    }

    #[test]
    fn exe_relative_dirs_are_the_exes_own_and_its_bin() {
        let exe = Path::new("/apps/Sensei/sensei-desktop.exe");
        assert_eq!(
            super::exe_relative_dirs(exe),
            [PathBuf::from("/apps/Sensei"), PathBuf::from("/apps/Sensei/bin")]
        );
    }
}
