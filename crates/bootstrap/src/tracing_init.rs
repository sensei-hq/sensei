//! Shared tracing-subscriber initialisers for sensei.
//!
//! Two transports run the bootstrap pipeline (CLI `doctor`, Tauri sidecar)
//! and each wants the same `sensei_bootstrap` events surfaced — just to
//! different sinks. This module owns the subscriber setup so both call
//! sites stay one-liners.
//!
//! Both initialisers use `try_init()` so a process that already has a
//! subscriber installed (e.g. a daemon embedding the library) is left
//! alone. Callers don't need to check.

use tracing_subscriber::EnvFilter;

/// `RUST_LOG` when it is set and valid, `default_filter` otherwise.
///
/// One implementation for all three initialisers so the override rule cannot
/// drift between them. `try_from_default_env` errors both when the variable is
/// unset and when it is unparseable; both mean "use the caller's default".
fn filter_or(default_filter: &str) -> EnvFilter {
    EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter))
}

/// Stdout subscriber. ANSI on, compact format. Reads `RUST_LOG` from the
/// env; falls back to `default_filter` (e.g. `"sensei_bootstrap=warn"`)
/// when unset.
///
/// Use case: the `sensei doctor` CLI, where the structured `HealthEvent`
/// timeline is the primary signal and library tracing is opt-in. Targets are
/// suppressed because that timeline, not the emitting module, is what the
/// reader is following — see [`install_daemon_console`] for the opposite case.
pub fn install_console(default_filter: &str) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter_or(default_filter))
        .with_target(false)
        .with_level(true)
        .compact()
        .try_init();
}

/// Stdout subscriber for a long-running service, with the emitting module KEPT.
///
/// Differs from [`install_console`] only in showing the target. A daemon's log
/// is read after the fact by someone asking which subsystem failed, and 752 of
/// the daemon's tracing calls do not name themselves in the message — without
/// the target those lines cannot be attributed to a module at all.
pub fn install_daemon_console(default_filter: &str) {
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter_or(default_filter))
        .with_target(true)
        .with_level(true)
        .compact()
        .try_init();
}

/// Where a daemon's rolling trace log lives, and how many days of it are kept.
///
/// Returns `(directory, file_name_prefix)`. Files land as
/// `<prefix>.YYYY-MM-DD`, which is the layout [`tracing_appender`] rolls and
/// prunes; keeping the naming in one place is what lets a reader find yesterday's
/// log without knowing the appender's conventions.
pub fn daemon_log_dir() -> std::path::PathBuf {
    crate::home_dir().join(crate::config::SenseiConfig::from_env().dir_suffix).join("logs")
}

/// Rolling-file subscriber for a long-running service: the daemon's real log.
///
/// ROTATION IS THE POINT. `install_daemon_console` writes to stdout, which under
/// `brew services` is captured by launchd into a single file that nothing ever
/// truncates. Measured 2026-09-30: **20 GB**, at the ordinary `senseid=info`
/// filter — so this is not a debug-verbosity accident, it is the steady state.
/// `tracing_appender` rolls daily and keeps `retention_days` files, so the
/// history is bounded by construction rather than by someone noticing.
///
/// STDOUT IS NARROWED TO WARN at the same time, and that pairing is load-bearing:
/// adding a rolling file while leaving stdout at `info` would bound the new log
/// and leave the launchd-captured one growing exactly as before. Warnings still
/// reach `brew services log`; the detail moves to [`daemon_log_dir`].
///
/// WHAT THIS DOES NOT BOUND, said plainly: bytes. Retention here counts FILES,
/// not size, so a day that emits 20 GB still writes a 20 GB file — daily
/// rotation merely stops it accumulating for ever. A byte budget needs either a
/// size-rotating appender or a pruner over the directory; until then the real
/// control is the filter.
///
/// Returns the appender guard, which the caller must hold for the process's
/// lifetime — dropping it stops the writer thread and silently ends logging.
#[must_use = "dropping the guard stops the log writer thread"]
pub fn install_daemon_rolling(
    dir: impl AsRef<std::path::Path>,
    default_filter: &str,
    retention_days: usize,
) -> Option<tracing_appender::non_blocking::WorkerGuard> {
    use tracing_subscriber::Layer;
    use tracing_subscriber::layer::SubscriberExt;
    use tracing_subscriber::util::SubscriberInitExt;

    // Failure to log is never fatal for the host process — the daemon still runs,
    // it just runs quietly. Same contract as `install_file`.
    std::fs::create_dir_all(dir.as_ref()).ok()?;
    let appender = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::DAILY)
        .filename_prefix(crate::config::SENSEID_BIN)
        .filename_suffix("log")
        .max_log_files(retention_days)
        .build(dir.as_ref())
        .ok()?;
    let (writer, guard) = tracing_appender::non_blocking(appender);

    let file = tracing_subscriber::fmt::layer()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(true)
        .with_level(true)
        .compact()
        .with_filter(filter_or(default_filter));

    // THE CONSOLE LAYER EXISTS ONLY FOR A HUMAN AT A TERMINAL.
    //
    // Under `brew services` stdout is not a terminal — it is a file launchd
    // opened, and nothing rotates it. Writing there as well as to the rolling
    // file duplicates every line into the one sink with no retention, which is
    // how the 20 GB was reached in the first place. Measured before this gate:
    // 59 MB/day of warnings, every one of them already in the rolling file.
    //
    // Run `senseid` in a terminal and stdout IS a tty, a person is watching, and
    // the console layer comes back at the same filter as the file. So the rule is
    // not "quieter in production" — it is "one sink per reader".
    let console = std::io::IsTerminal::is_terminal(&std::io::stdout()).then(|| {
        tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_level(true)
            .compact()
            .with_filter(filter_or(default_filter))
    });

    let installed = tracing_subscriber::registry().with(file).with(console).try_init().is_ok();
    installed.then_some(guard)
}

/// File subscriber writing to `path` (append). ANSI off, compact format.
/// Reads `RUST_LOG`; falls back to `default_filter` when unset. If the
/// file can't be opened, the call is a no-op — failure to log is never
/// a fatal condition for the host process.
///
/// Use case: the Tauri sidecar, which writes bootstrap traces to a known
/// log path users can `tail -f` while reproducing an install bug.
pub fn install_file(path: impl AsRef<std::path::Path>, default_filter: &str) {
    use std::fs::OpenOptions;
    let Ok(file) = OpenOptions::new().create(true).append(true).open(path) else {
        return;
    };
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(default_filter));
    let _ = tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::sync::Mutex::new(file))
        .with_ansi(false)
        .with_target(false)
        .compact()
        .try_init();
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Retention must be a FILE COUNT the appender ENFORCES, not a setting it
    /// merely accepts.
    ///
    /// Seeds older dated files under the daemon's own naming scheme, then builds
    /// the appender exactly as `install_daemon_rolling` does and makes it write.
    /// Pruning is what must remove the surplus.
    ///
    /// Written this way after the first version passed VACUOUSLY: it wrote four
    /// times inside one minute, so nothing ever rotated, only one file existed,
    /// and `kept <= retention` held whether or not retention was configured at
    /// all. Seeding the history is what makes the assertion able to fail.
    ///
    /// Breaking mutation: drop `.max_log_files(retention)` — the seeded files
    /// survive and the directory grows without bound, which is the 20 GB defect
    /// one level down.
    #[test]
    fn rotation_prunes_history_beyond_the_retained_file_count() {
        use std::io::Write;
        let dir = tempfile::tempdir().unwrap();
        let retention = 2;

        // Five days of history, in the layout the DAILY appender globs for.
        for day in 1..=5 {
            std::fs::write(dir.path().join(format!("senseid.2020-01-0{day}.log")), b"old\n")
                .unwrap();
        }
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 5, "seeded history");

        let mut appender = tracing_appender::rolling::Builder::new()
            .rotation(tracing_appender::rolling::Rotation::DAILY)
            .filename_prefix("senseid")
            .filename_suffix("log")
            .max_log_files(retention)
            .build(dir.path())
            .expect("the builder must accept the daemon's own settings");
        writeln!(appender, "today").unwrap();
        appender.flush().unwrap();

        let kept = std::fs::read_dir(dir.path()).unwrap().count();
        assert!(
            kept <= retention,
            "retention is a bound, not a suggestion: {kept} files kept for a limit of \
             {retention}"
        );
    }

    /// The daemon log directory must sit under the instance's own data dir, so a
    /// `SENSEI_INSTANCE` sandbox never writes into the real install's logs.
    ///
    /// Breaking mutation: hardcode `~/.sensei/logs` — an e2e run then prunes the
    /// user's production daemon log.
    #[test]
    fn the_daemon_log_dir_follows_the_instance() {
        let dir = super::daemon_log_dir();
        assert!(dir.ends_with("logs"), "logs live in their own directory: {dir:?}");
        let parent = dir.parent().unwrap().file_name().unwrap().to_string_lossy().to_string();
        assert!(
            parent.starts_with(".sensei"),
            "the log dir must hang off the instance data dir, got parent {parent:?}"
        );
    }

    #[test]
    fn install_console_does_not_panic() {
        install_console("sensei_bootstrap=warn");
        // try_init guarantees a second call is a no-op, not a panic.
        install_console("sensei_bootstrap=debug");
    }

    #[test]
    fn install_daemon_console_does_not_panic() {
        // Same try_init contract: the daemon installs one at startup, and a
        // second call (a test, an embedded host) must be a no-op.
        install_daemon_console("senseid=info,warn");
        install_daemon_console("senseid=debug,warn");
    }

    #[test]
    fn an_unset_rust_log_falls_back_to_the_callers_default() {
        // The daemon ships under a launch agent that sets no RUST_LOG, so this
        // fallback is the ONLY filter it ever gets. `try_from_default_env`
        // errors on an unset variable, which is what makes the fallback fire.
        if std::env::var("RUST_LOG").is_ok() {
            // A developer running with RUST_LOG set is exercising the OVERRIDE,
            // not the fallback. Skipping is honest; asserting would test theirs.
            return;
        }
        let hint = filter_or("senseid=info,warn").max_level_hint();
        assert_eq!(
            hint,
            Some(tracing_subscriber::filter::LevelFilter::INFO),
            "the default must admit INFO — under ERROR the whole sync cycle goes dark"
        );
    }

    #[test]
    fn install_file_writes_to_temp_path() {
        let tmp =
            std::env::temp_dir().join(format!("sensei-tracing-test-{}.log", std::process::id(),));
        let _ = std::fs::remove_file(&tmp);
        install_file(&tmp, "sensei_bootstrap=info");
        // try_init is idempotent; this should not panic even if a
        // subscriber from install_console was already installed.
        assert!(tmp.exists(), "log file must be created on first call");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn install_file_with_unwritable_path_is_silent() {
        // A path under / on a writable FS still tends to be writable on
        // CI; pick a clearly-bogus parent that doesn't exist. The helper
        // must swallow the open error and return.
        install_file("/this/path/should/not/exist/sensei-tracing.log", "sensei_bootstrap=warn");
    }
}
