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
    if let Ok(home) = std::env::var("HOME")
        && !home.is_empty()
    {
        dirs.push(format!("{home}/.qlty/bin"));
    }
    dirs
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

#[cfg(test)]
mod tests {
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
        if let Ok(home) = std::env::var("HOME")
            && !home.is_empty()
        {
            let dirs = user_local_bin_dirs();
            assert!(
                dirs.iter().any(|d| d == &format!("{home}/.qlty/bin")),
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
}
