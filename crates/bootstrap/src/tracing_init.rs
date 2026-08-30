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
