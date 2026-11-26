//! Database credentials from `~/.sensei/.env` (#505, design D3).
//!
//! The file holds the standard libpq names `PGUSER` and `PGPASSWORD`. They are
//! put into the process environment at startup — never into a connection URL —
//! because every client sensei uses reads them from there: `psql`, `createdb`
//! and `pg_isready` (libpq), and sqlx (the daemon's pool and dbd-core's
//! deploy) when the URL carries no credentials. So a password never appears in
//! a URL, a log line or an error.
//!
//! Real environment variables win over the file. The file is optional on
//! macOS and Linux, where a local Postgres trusts the OS user; on Windows,
//! where Postgres is the `sensei-pg` container, the database check requires
//! both keys and names the file when either is missing. Nothing is generated
//! or defaulted.

use std::path::{Path, PathBuf};

pub const PGUSER: &str = "PGUSER";
pub const PGPASSWORD: &str = "PGPASSWORD";

/// The keys read from the file; any other line is ignored.
pub const KEYS: [&str; 2] = [PGUSER, PGPASSWORD];

/// `~/.sensei/.env` — the real install's, whatever `SENSEI_INSTANCE` says:
/// instances share one Postgres server, so they share its credentials.
pub fn env_file() -> Option<PathBuf> {
    crate::config::try_home_dir().map(|home| home.join(".sensei").join(".env"))
}

/// The file's `KEYS`, or `Ok(None)` when there is no file. An error names the
/// file and the line, never a value.
pub fn read(path: &Path) -> Result<Option<Vec<(String, String)>>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => parse(&text).map(Some).map_err(|e| format!("{}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("{}: {e}", path.display())),
    }
}

/// `KEY=VALUE` lines: blank lines and `#` comments are skipped, a leading
/// `export ` is allowed, and a value wrapped in matching quotes is unwrapped.
/// Values are literal — no `$VAR` expansion, which would corrupt a password.
fn parse(text: &str) -> Result<Vec<(String, String)>, String> {
    let mut pairs = Vec::new();
    for (i, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let line = line.strip_prefix("export ").unwrap_or(line);
        let Some((key, value)) = line.split_once('=') else {
            return Err(format!("line {} is not KEY=VALUE", i + 1));
        };
        let key = key.trim();
        if !KEYS.contains(&key) {
            continue;
        }
        pairs.push((key.to_string(), unquote(value.trim()).to_string()));
    }
    Ok(pairs)
}

fn unquote(v: &str) -> &str {
    for q in ['"', '\''] {
        if v.len() >= 2 && v.starts_with(q) && v.ends_with(q) {
            return &v[1..v.len() - 1];
        }
    }
    v
}

/// The pairs to set: those whose key the environment does not already hold.
/// A later line for the same key replaces an earlier one, as in a shell.
fn to_apply(pairs: Vec<(String, String)>, is_set: impl Fn(&str) -> bool) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for (k, v) in pairs.into_iter().filter(|(k, _)| !is_set(k)) {
        out.retain(|(seen, _)| *seen != k);
        out.push((k, v));
    }
    out
}

fn env_is_set(key: &str) -> bool {
    std::env::var_os(key).is_some_and(|v| !v.is_empty())
}

/// Put the file's credentials into this process's environment, under the
/// variables it does not already have. Without a file this does nothing.
///
/// # Safety
/// Mutates the process environment: call it at startup, before any other
/// thread could read the environment.
pub unsafe fn load_into_env() -> Result<(), String> {
    let Some(path) = env_file() else { return Ok(()) };
    let Some(pairs) = read(&path)? else { return Ok(()) };
    for (k, v) in to_apply(pairs, env_is_set) {
        // SAFETY: the caller's contract — no other thread is running.
        unsafe { std::env::set_var(&k, v) };
    }
    Ok(())
}

/// Why the database credentials are not usable, when they are not: the file
/// could not be read, or `PGUSER`/`PGPASSWORD` is set neither in the
/// environment nor in the file. Names the file and the keys, never a value.
pub fn missing(path: &Path) -> Option<String> {
    problem(path, read(path), env_is_set)
}

fn problem(
    path: &Path,
    file: Result<Option<Vec<(String, String)>>, String>,
    is_set: impl Fn(&str) -> bool,
) -> Option<String> {
    let pairs = match file {
        Err(e) => return Some(format!("cannot read the database credentials: {e}")),
        Ok(pairs) => pairs.unwrap_or_default(),
    };
    let absent: Vec<&str> = KEYS
        .into_iter()
        .filter(|k| !is_set(k) && !pairs.iter().any(|(pk, v)| pk == k && !v.is_empty()))
        .collect();
    if absent.is_empty() {
        return None;
    }
    Some(format!(
        "database credentials missing: set {} in {} (the user and password of the sensei-pg container)",
        absent.join(" and "),
        path.display()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairs(v: &[(&str, &str)]) -> Vec<(String, String)> {
        v.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
    }

    #[test]
    fn parses_the_two_keys_and_ignores_the_rest() {
        let text = "# sensei\n\nPGUSER=sensei\nOTHER=x\nexport PGPASSWORD=\"p@ss word\"\n";
        assert_eq!(
            parse(text).unwrap(),
            pairs(&[("PGUSER", "sensei"), ("PGPASSWORD", "p@ss word")])
        );
    }

    #[test]
    fn values_are_literal() {
        let text = "PGPASSWORD=a$HOME'b=c\nPGUSER='x y'";
        assert_eq!(parse(text).unwrap(), pairs(&[("PGPASSWORD", "a$HOME'b=c"), ("PGUSER", "x y")]));
    }

    #[test]
    fn a_malformed_line_is_named_by_number_without_its_text() {
        let err = parse("PGUSER=a\nhunter2\n").unwrap_err();
        assert_eq!(err, "line 2 is not KEY=VALUE");
    }

    #[test]
    fn a_missing_file_is_none_and_a_present_one_is_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(".env");
        assert_eq!(read(&path).unwrap(), None);
        std::fs::write(&path, "PGUSER=sensei\n").unwrap();
        assert_eq!(read(&path).unwrap(), Some(pairs(&[("PGUSER", "sensei")])));
    }

    #[test]
    fn the_environment_wins_over_the_file() {
        let file = pairs(&[("PGUSER", "from-file"), ("PGPASSWORD", "secret")]);
        let applied = to_apply(file, |k| k == "PGUSER");
        assert_eq!(applied, pairs(&[("PGPASSWORD", "secret")]));
    }

    #[test]
    fn a_later_line_replaces_an_earlier_one() {
        let file = pairs(&[("PGUSER", "a"), ("PGUSER", "b")]);
        assert_eq!(to_apply(file, |_| false), pairs(&[("PGUSER", "b")]));
    }

    #[test]
    fn no_problem_when_both_keys_are_set_somewhere() {
        let path = Path::new("/h/.sensei/.env");
        let file = Ok(Some(pairs(&[("PGPASSWORD", "secret")])));
        assert_eq!(problem(path, file, |k| k == "PGUSER"), None);
    }

    #[test]
    fn no_file_names_the_file_and_both_keys() {
        let msg = problem(Path::new("/h/.sensei/.env"), Ok(None), |_| false).unwrap();
        assert!(msg.contains("PGUSER and PGPASSWORD"), "{msg}");
        assert!(msg.contains("/h/.sensei/.env"), "{msg}");
    }

    #[test]
    fn an_empty_password_counts_as_missing() {
        let file = Ok(Some(pairs(&[("PGUSER", "sensei"), ("PGPASSWORD", "")])));
        let msg = problem(Path::new("/h/.env"), file, |_| false).unwrap();
        assert!(msg.contains("set PGPASSWORD in"), "{msg}");
    }

    #[test]
    fn an_unreadable_file_is_a_problem_and_shows_no_value() {
        let msg =
            problem(Path::new("/h/.env"), Err("/h/.env: line 3 is not KEY=VALUE".into()), |_| true)
                .unwrap();
        assert!(msg.contains("line 3"), "{msg}");
    }
}
