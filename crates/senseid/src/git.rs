//! Run `git` and keep the exit code.
//!
//! The daemon shells out to `git` in three places today and every one of them
//! answers with an `Option` — [`crate::git_identity::read_git_user`],
//! `tasks::handlers::scan::read_git_remotes` and
//! `tasks::handlers::metrics::churn::run_git`. An `Option` throws the exit code
//! away, and with it the difference between these four outcomes (all measured
//! against git 2.50.1):
//!
//! | outcome                          | what git actually does               | as `Option` |
//! |----------------------------------|--------------------------------------|-------------|
//! | the dir is not a repository      | exit 128, `fatal: not a git reposi…` | `None`      |
//! | `git` missing / not executable   | the spawn fails; git never runs      | `None`      |
//! | git is killed (OOM, SIGKILL)     | dies with NO exit code at all        | `None`      |
//! | the repository has no commits    | **exit 0, empty stdout**             | `Some("")`  |
//!
//! Reading a remote URL can live with that conflation: you either learned a URL
//! or you did not. The git-history scanner (#224) cannot. It records a
//! per-checkout cursor in `sensei.commit_scans` meaning "this checkout has been
//! walked up to these tips". If git fails transiently — upgraded underneath us,
//! OOM-killed, a network filesystem blinking — and that failure arrives at the
//! caller as "no output", the scanner writes a cursor claiming a SUCCESSFUL
//! scan of zero commits. The repository's history is then silently absent, and
//! nothing downstream can tell it apart from a repository that genuinely has no
//! commits, because an empty repository answers the tip-set walk with exactly
//! that: exit 0 and no bytes (`git log --branches --remotes --tags` in a fresh
//! `git init` — measured, not assumed).
//!
//! So this module returns `Result`, and the ONLY thing that produces an `Ok` is
//! git running to completion and exiting 0. After that, empty means empty.
//!
//! ## Bytes, not `String`
//! [`run_bytes`] is the real entry point. git's `-z` output — the form the
//! commit/numstat walk must use — separates records with NUL and emits paths as
//! RAW BYTES, which on macOS and Linux are not required to be UTF-8. Decoding
//! those lossily is not a cosmetic loss: `String::from_utf8_lossy` substitutes
//! U+FFFD, so a path that cannot be decoded becomes a *different, plausible
//! path* that gets stored as a fact and never matches the file it came from.
//! [`run_text`] therefore ERRORS on invalid UTF-8 rather than mangling it, and
//! is for git's ASCII-shaped scalar output (shas, dates, emails, ref names)
//! only. A path must never travel through it.
//!
//! ## The environment git is given
//! Three settings, each load-bearing:
//! * **stdin is `/dev/null`.** git prompts on stdin for credentials under some
//!   configurations, and inheriting the daemon's stdin would park this call for
//!   ever. (`output()` already defaults stdin to null; the explicit call is
//!   what keeps that true if this ever moves to `spawn()` + streaming.)
//! * **`GIT_TERMINAL_PROMPT=0`** closes the other half of that door: with no
//!   usable stdin git still reaches for the terminal and for
//!   `GIT_ASKPASS`/`SSH_ASKPASS` helpers. Set, an auth prompt becomes a
//!   non-zero exit — a failure we can see — instead of a hang we cannot.
//! * **`LC_ALL=C` with `LANGUAGE` emptied** pins git's messages to English.
//!   [`GitError::is_not_a_repository`] reads git's own wording; against a
//!   localized git (`ce n'est pas un dépôt git`) that classifier would quietly
//!   answer `false`, and the scanner would log an ordinary non-repository as an
//!   unexplained failure on every pass, for ever.
//!
//! ## No deadline, on purpose
//! [`std::process::Command::output`] takes no timeout, which is why
//! `read_git_remotes` has none either; callers run these inside
//! [`tokio::task::spawn_blocking`], so a slow git costs a blocking thread
//! rather than an async worker, and the task's own budget can still fire.
//!
//! That is not merely a limitation we inherited — it is the right default here.
//! A history walk over a large repository legitimately takes minutes (~121,780
//! commits across this machine). A deadline firing mid-walk hands back a
//! PARTIAL result, and a partial walk accepted as a complete one is precisely
//! the failure this module exists to prevent. If a deadline is ever added it
//! must surface as its own error variant. It must never truncate an `Ok`.

use std::path::Path;
use std::process::{Command, Stdio};

/// How much of git's stderr a [`GitError`] carries.
///
/// The TAIL, not the head: git prints warnings first and its `fatal:` line
/// last, immediately before exiting, so the end of the stream is the part that
/// says what went wrong. Bounded because this string is logged on every failed
/// pass over every repository on the machine, and an unbounded one would let a
/// repository with a pathological hook flood the daemon log.
const STDERR_TAIL_CHARS: usize = 400;

/// git's own words for "this directory is not a repository", lowercased for a
/// case-insensitive match: git has shipped both `Not a git repository` (older
/// releases) and `not a git repository` (current), and an enterprise box
/// running an old git must not be misclassified as a transient failure.
const NOT_A_REPOSITORY: &str = "not a git repository";

/// A git invocation that did not produce an answer.
///
/// Four variants because four *different things* happened, and the scanner
/// treats them differently — that separation is the whole reason this module
/// exists. Follows the house error style ([`crate::dojo::client::DojoClientError`],
/// [`crate::agent_spawn::AgentSpawnError`]): `Debug` derive, hand-written
/// `Display`, `impl std::error::Error`. `PartialEq` so tests can assert the
/// exact variant rather than string-matching a message.
///
/// There is deliberately NO variant for "git succeeded but returned nothing".
/// That is an `Ok` with empty bytes, and it is a real answer about the
/// repository rather than a failure to get one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GitError {
    /// git never ran: the binary is absent from `PATH`, is not executable, or
    /// the working directory could not be entered. Says NOTHING about the
    /// repository — the caller learned nothing and must not record a scan.
    ///
    /// `kind` is [`std::io::ErrorKind::NotFound`] for both "no git binary" and
    /// "no such directory"; the OS does not let us tell those apart from the
    /// errno alone, and `message` is what distinguishes them in a log.
    Spawn { kind: std::io::ErrorKind, message: String },
    /// git ran to completion and exited non-zero. `code` is git's own exit
    /// status (128 is its catch-all `fatal:`; 1 is "no such subcommand" and,
    /// for `git diff --exit-code`, "there were differences"), and `stderr` is
    /// the tail of what it complained about. This is a VERDICT: git looked and
    /// is telling us why it refused.
    Exit { code: i32, stderr: String },
    /// git was killed by a signal and so has no exit code (the OOM killer, a
    /// supervisor tearing the process group down, SIGPIPE). Kept apart from
    /// [`GitError::Exit`] because an absent exit code is not exit code 0, and
    /// collapsing the two is how a killed walk gets filed as an empty one.
    /// `status` is the platform's own rendering, e.g. `signal: 9 (SIGKILL)`.
    Signal { status: String, stderr: String },
    /// git exited 0 but its stdout is not valid UTF-8, and the caller asked for
    /// text ([`run_text`]). Raised instead of decoding lossily: a path mangled
    /// into U+FFFD is a fabricated path, stored as a fact that matches no file.
    /// The fix at a call site is [`run_bytes`], not a lossy decode.
    NotUtf8 { bytes: usize, valid_up_to: usize },
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GitError::Spawn { kind, message } => {
                write!(f, "git could not be started ({kind:?}): {message}")
            }
            GitError::Exit { code, stderr } => write!(f, "git exited {code}: {stderr}"),
            GitError::Signal { status, stderr } => {
                write!(f, "git was killed before it could exit ({status}): {stderr}")
            }
            GitError::NotUtf8 { bytes, valid_up_to } => {
                write!(f, "git stdout is not utf-8: {bytes} bytes, valid up to {valid_up_to}")
            }
        }
    }
}

impl std::error::Error for GitError {}

impl GitError {
    /// git's exit code, or `None` when git never ran to a code — it failed to
    /// spawn, was signalled, or exited 0 with undecodable output.
    ///
    /// `None` here means "there is no exit code", never "the exit code was 0".
    /// A caller branching on `== Some(0)` is looking at an impossible case:
    /// exit 0 is an `Ok`.
    pub fn exit_code(&self) -> Option<i32> {
        match self {
            GitError::Exit { code, .. } => Some(*code),
            _ => None,
        }
    }

    /// True only when git ran, looked, and reported that the directory is not a
    /// git repository — the one failure that is ORDINARY. A watched folder that
    /// is not a checkout produces it on every pass, so the caller should skip
    /// quietly rather than log an error.
    ///
    /// Everything else at exit 128 is NOT this and must stay loud. git uses 128
    /// for every `fatal:`, including `detected dubious ownership in repository
    /// at …` (a `safe.directory` misconfiguration that hides a REAL repository,
    /// and that is silently never scanned if it is filed as "not a repo") and
    /// `ambiguous argument …` (our own bad revision). Hence the match is on
    /// git's words, not on the code.
    ///
    /// Reads the stderr tail, which is where the `fatal:` line lands — git
    /// emits this one at startup and exits immediately, so it is never pushed
    /// out of the window by later output.
    pub fn is_not_a_repository(&self) -> bool {
        match self {
            GitError::Exit { stderr, .. } => stderr.to_ascii_lowercase().contains(NOT_A_REPOSITORY),
            // A spawn failure or a signal taught us nothing about the directory.
            // Answering `true` here would let a machine-level fault be recorded
            // as a settled fact about the repository.
            _ => false,
        }
    }
}

/// Run `git <args>` in `dir` and return its stdout as RAW BYTES.
///
/// The byte-exact entry point, and the one the commit walk must use: under `-z`
/// git separates records with NUL and writes paths verbatim, so the output is
/// neither line-structured nor guaranteed UTF-8. Returning `Vec<u8>` is what
/// lets the single shared numstat parser see the bytes git actually produced.
/// What reading git's *quoted* line output instead costs, measured on this
/// repository: 2,087 of 24,496 churn records are rename pseudo-paths stored
/// verbatim, and 30 more are C-quoted octal escapes — 8.6% of the feed holding
/// strings that match no file.
///
/// `Ok(vec![])` is a real, positive answer: git ran and had nothing to say.
/// Only that — exit 0 — produces `Ok`. Every other outcome is a [`GitError`]
/// the caller must handle before it writes anything durable.
///
/// Blocking. Call it inside [`tokio::task::spawn_blocking`]; see the module
/// docs on why there is no deadline.
pub fn run_bytes(dir: &Path, args: &[&str]) -> Result<Vec<u8>, GitError> {
    capture("git", dir, args)
}

/// Run `git <args>` in `dir` and return its stdout as text.
///
/// The convenience for git's ASCII-shaped scalar output — shas, `%aI` dates,
/// author emails, ref names, counts. **Not for paths.** It is strict rather
/// than lossy on purpose: an undecodable byte becomes [`GitError::NotUtf8`],
/// not a U+FFFD that silently turns one path into a plausible different one and
/// stores it as a fact. If a call site here ever trips that error, the answer
/// is to move it to [`run_bytes`] and parse the bytes — never to decode lossily.
pub fn run_text(dir: &Path, args: &[&str]) -> Result<String, GitError> {
    decode_utf8(run_bytes(dir, args)?)
}

/// The one place a git subprocess is actually spawned.
///
/// Parameterised on `program` purely as a testability seam: the error
/// classification below — spawn vs. exit code vs. signal — is the point of this
/// module, and driving it through `/bin/sh` stubs exercises the real code path
/// without needing a repository, a network, or a way to make `git` crash on
/// demand. Production has exactly one caller, [`run_bytes`], and it passes
/// `"git"`.
fn capture(program: &str, dir: &Path, args: &[&str]) -> Result<Vec<u8>, GitError> {
    let output = Command::new(program)
        .args(args)
        .current_dir(dir)
        // See the module docs: each of these three is load-bearing.
        .stdin(Stdio::null())
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("LC_ALL", "C")
        .env("LANGUAGE", "")
        .output()
        .map_err(|e| GitError::Spawn { kind: e.kind(), message: e.to_string() })?;

    if output.status.success() {
        return Ok(output.stdout);
    }
    let stderr = stderr_tail(&output.stderr);
    // `code()` is `None` exactly when the process was signalled. Folding that
    // into a numeric code would require inventing one, and any number we
    // invented would be indistinguishable from a code git really returned.
    match output.status.code() {
        Some(code) => Err(GitError::Exit { code, stderr }),
        None => Err(GitError::Signal { status: output.status.to_string(), stderr }),
    }
}

/// Decode git's stdout as UTF-8, or fail. Split out from [`run_text`] so the
/// refusal-to-mangle is unit-testable without a subprocess.
fn decode_utf8(bytes: Vec<u8>) -> Result<String, GitError> {
    String::from_utf8(bytes).map_err(|e| GitError::NotUtf8 {
        bytes: e.as_bytes().len(),
        valid_up_to: e.utf8_error().valid_up_to(),
    })
}

/// The last [`STDERR_TAIL_CHARS`] characters of git's stderr, trimmed, with a
/// leading `…` when something was dropped.
///
/// Lossy decoding is correct HERE and nowhere else in this module: this string
/// is a diagnostic for a human reading a log, never a fact that gets stored, so
/// a replacement character costs nothing. Truncation counts CHARACTERS rather
/// than slicing bytes, so a multi-byte sequence is never cut in half.
fn stderr_tail(stderr: &[u8]) -> String {
    let decoded = String::from_utf8_lossy(stderr);
    let text = decoded.trim();
    let total = text.chars().count();
    if total <= STDERR_TAIL_CHARS {
        return text.to_string();
    }
    let mut out = String::with_capacity(STDERR_TAIL_CHARS * 4 + 4);
    out.push('…');
    out.extend(text.chars().skip(total - STDERR_TAIL_CHARS));
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A program name no `PATH` can resolve, for the spawn-failure tests.
    const NO_SUCH_PROGRAM: &str = "sensei-no-such-program-224";

    // ── Pure: the stderr tail ────────────────────────────────────────────────

    #[test]
    fn stderr_tail_keeps_short_output_whole_and_trimmed() {
        assert_eq!(stderr_tail(b"fatal: not a git repository\n"), "fatal: not a git repository");
        assert_eq!(stderr_tail(b"   \n  \t"), "");
        assert_eq!(stderr_tail(b""), "");
    }

    #[test]
    fn stderr_tail_keeps_the_end_not_the_beginning() {
        // git prints warnings first and its `fatal:` line last. A head-keeping
        // truncation would keep the noise, drop the diagnosis, and with it the
        // needle `is_not_a_repository` matches on — so the repository would be
        // logged as an unexplained failure instead of skipped as "not a repo".
        let noisy =
            format!("{}\nfatal: not a git repository\n", "warning: hook is slow\n".repeat(80));
        let tail = stderr_tail(noisy.as_bytes());
        assert!(tail.starts_with('…'), "a truncated tail is marked as truncated: {tail:?}");
        assert!(tail.ends_with("fatal: not a git repository"), "the last line survives: {tail:?}");
        assert_eq!(tail.chars().count(), STDERR_TAIL_CHARS + 1, "bounded + the ellipsis");
        assert!(tail.chars().count() < noisy.chars().count(), "the hook noise was not all kept");
        // And the classifier still works off the truncated tail.
        assert!(GitError::Exit { code: 128, stderr: tail }.is_not_a_repository());
    }

    #[test]
    fn stderr_tail_never_splits_a_multibyte_character() {
        // Truncating by BYTES would cut a 3-byte char in half and either panic
        // on a non-boundary slice or emit a U+FFFD. Count characters instead.
        let wide = "é".repeat(STDERR_TAIL_CHARS + 50);
        let tail = stderr_tail(wide.as_bytes());
        assert_eq!(tail.chars().count(), STDERR_TAIL_CHARS + 1);
        assert!(tail.chars().skip(1).all(|c| c == 'é'), "no half character survived: {tail:?}");
    }

    #[test]
    fn stderr_tail_decodes_invalid_bytes_lossily_rather_than_failing() {
        // A diagnostic must always be printable; this is the ONE place in the
        // module where lossy decoding is the right answer.
        let tail = stderr_tail(b"fatal: \xff\xfe bad\n");
        assert!(tail.starts_with("fatal:"), "{tail:?}");
        assert!(tail.contains('\u{fffd}'), "undecodable bytes show as U+FFFD: {tail:?}");
    }

    // ── Pure: the UTF-8 refusal ──────────────────────────────────────────────

    #[test]
    fn decode_utf8_passes_valid_text_through() {
        assert_eq!(decode_utf8(b"7f733a882\n".to_vec()).unwrap(), "7f733a882\n");
        assert_eq!(decode_utf8(Vec::new()).unwrap(), "");
    }

    #[test]
    fn decode_utf8_refuses_to_mangle_an_undecodable_path() {
        // THE anti-fabrication case: a latin-1 path byte. A lossy decode would
        // hand back "src/caf\u{fffd}.rs" — a plausible path that matches no file
        // and would be stored as a commit fact. Error instead.
        let err = decode_utf8(b"src/caf\xe9.rs".to_vec()).unwrap_err();
        assert_eq!(err, GitError::NotUtf8 { bytes: 11, valid_up_to: 7 });
        assert_eq!(err.exit_code(), None, "no exit code: git exited 0, we could not read it");
        assert!(!err.is_not_a_repository(), "an undecodable path says nothing about the repo");
    }

    // ── Pure: classifying a failure ──────────────────────────────────────────

    #[test]
    fn is_not_a_repository_matches_git_in_both_casings() {
        // Current git lowercases it; older releases capitalised it. Both are the
        // same ordinary outcome and must classify the same way.
        for stderr in [
            "fatal: not a git repository (or any of the parent directories): .git",
            "fatal: Not a git repository (or any of the parent directories): .git",
            "fatal: not a git repository: '/tmp/x/.git'",
        ] {
            let err = GitError::Exit { code: 128, stderr: stderr.to_string() };
            assert!(err.is_not_a_repository(), "{stderr:?} is the ordinary not-a-repo case");
            assert_eq!(err.exit_code(), Some(128));
        }
    }

    #[test]
    fn is_not_a_repository_does_not_swallow_the_other_128s() {
        // git spends 128 on every `fatal:`. Dubious ownership hides a REAL
        // repository behind a `safe.directory` setting — classify it as "not a
        // repo" and that repository is quietly never scanned again. An ambiguous
        // argument is OUR bug in the argv. Both must stay loud.
        for stderr in [
            "fatal: detected dubious ownership in repository at '/Users/x/dev/sensei'",
            "fatal: ambiguous argument 'deadbeef': unknown revision or path not in the tree",
            "fatal: cannot change to '/tmp/gone': No such file or directory",
        ] {
            let err = GitError::Exit { code: 128, stderr: stderr.to_string() };
            assert!(!err.is_not_a_repository(), "{stderr:?} must not be read as a missing repo");
        }
    }

    #[test]
    fn a_failure_with_no_exit_code_is_never_mistaken_for_a_verdict() {
        // The scanner's rule is "only an Ok may advance the cursor". These two
        // carry no exit code at all, and neither may masquerade as one — least
        // of all as 0.
        let spawn = GitError::Spawn {
            kind: std::io::ErrorKind::NotFound,
            message: "No such file or directory (os error 2)".to_string(),
        };
        let signal =
            GitError::Signal { status: "signal: 9 (SIGKILL)".to_string(), stderr: String::new() };
        for err in [&spawn, &signal] {
            assert_eq!(err.exit_code(), None);
            assert!(!err.is_not_a_repository());
        }
        assert!(spawn.to_string().contains("could not be started"), "{spawn}");
        assert!(signal.to_string().contains("killed"), "{signal}");
    }

    // ── The runner, driven through /bin/sh stubs (no repository needed) ──────

    #[test]
    fn capture_returns_stdout_bytes_verbatim_including_nul() {
        // The `-z` contract: NUL is a RECORD SEPARATOR in git's output, not a
        // terminator of it. Anything that stopped at the first NUL, or decoded
        // on the way out, would lose every record after the first.
        let out = capture("/bin/sh", Path::new("/"), &["-c", r"printf 'a\0b\377\n'"]).unwrap();
        assert_eq!(out, vec![b'a', 0, b'b', 0xff, b'\n']);
        assert!(String::from_utf8(out).is_err(), "the fixture really is not utf-8");
    }

    #[test]
    fn capture_succeeding_with_no_output_is_ok_not_an_error() {
        // THE defect this module fixes, in miniature: a successful command that
        // printed nothing is an answer ("nothing to report"), not a failure.
        assert_eq!(
            capture("/bin/sh", Path::new("/"), &["-c", "exit 0"]).unwrap(),
            Vec::<u8>::new()
        );
    }

    #[test]
    fn capture_non_zero_exit_carries_the_code_and_the_stderr() {
        let err = capture("/bin/sh", Path::new("/"), &["-c", "echo boom >&2; exit 7"]).unwrap_err();
        assert_eq!(err, GitError::Exit { code: 7, stderr: "boom".to_string() });
        assert_eq!(err.exit_code(), Some(7), "the code survives — this is the whole point");
    }

    #[test]
    fn capture_spawn_failure_when_the_program_does_not_exist() {
        let err = capture(NO_SUCH_PROGRAM, Path::new("/"), &[]).unwrap_err();
        assert!(
            matches!(err, GitError::Spawn { kind: std::io::ErrorKind::NotFound, .. }),
            "a missing binary is a spawn failure, not an exit code: {err:?}"
        );
        assert_eq!(err.exit_code(), None, "git never ran, so there is no exit code to report");
    }

    #[test]
    fn capture_spawn_failure_when_the_working_directory_is_gone() {
        // A checkout deleted between discovery and the scan. The process cannot
        // chdir, so it never execs — this must not reach the caller as "that
        // repository has no commits".
        let gone = Path::new("/tmp/sensei-224-no-such-directory");
        let err = capture("/bin/sh", gone, &["-c", "exit 0"]).unwrap_err();
        assert!(matches!(err, GitError::Spawn { .. }), "{err:?}");
        assert_eq!(err.exit_code(), None);
    }

    #[cfg(unix)]
    #[test]
    fn capture_signalled_process_is_not_reported_as_an_exit_code() {
        // An OOM-killed git has NO exit code. Unix-only because a signal is a
        // Unix concept; the daemon ships on macOS and Linux.
        let err = capture("/bin/sh", Path::new("/"), &["-c", "kill -9 $$"]).unwrap_err();
        assert!(
            matches!(err, GitError::Signal { .. }),
            "a killed process must not be filed as an exit: {err:?}"
        );
        assert_eq!(err.exit_code(), None);
    }

    // ── Needs a real repository: `git` on PATH + a temp checkout ─────────────
    //
    // These four shell out to the real `git` binary and build a throwaway
    // repository with the shared fixture
    // (`crate::tasks::test_support::git_init_repo`). No database, no network.
    // They are the only tests here that cannot be driven by a stub, because the
    // behaviour under test IS git's: which outcomes it reports with an exit code
    // and which with silence.

    /// NEEDS A REAL REPOSITORY (temp checkout + the `git` binary).
    ///
    /// The distinction the whole module exists for, asserted end to end: an
    /// EMPTY repository and a NON-repository must not look alike. Mutating
    /// either arm into the other is what the `Option`-returning helpers already
    /// do, and it is why a transient failure could be written down as a clean
    /// scan of zero commits.
    #[test]
    fn run_bytes_separates_an_empty_repository_from_a_missing_one() {
        let empty = tempfile::tempdir().unwrap();
        crate::tasks::test_support::git_init_repo(empty.path());
        // The tip-set walk — `--branches --remotes --tags`, the form #224 uses.
        // Bare `git log` would exit 128 here ("does not have any commits yet");
        // the tip set answers honestly with success and nothing.
        let tips = ["log", "--branches", "--remotes", "--tags", "--format=%H"];
        let out = run_bytes(empty.path(), &tips).expect("an empty repo is a SUCCESSFUL empty walk");
        assert!(out.is_empty(), "no commits yet, so no shas: {out:?}");

        let not_a_repo = tempfile::tempdir().unwrap();
        let err = run_bytes(not_a_repo.path(), &tips)
            .expect_err("a directory with no .git is a failure, not an empty repository");
        assert!(err.is_not_a_repository(), "git's own diagnosis is recognised: {err}");
        assert_eq!(err.exit_code(), Some(128), "measured against git 2.50.1");

        // The convenience entry point must not be where the discipline leaks:
        // an empty string out of `run_text` would be exactly the old `Some("")`.
        assert_eq!(run_text(empty.path(), &tips).unwrap(), "", "empty repo → a real empty answer");
        assert!(
            run_text(not_a_repo.path(), &tips).unwrap_err().is_not_a_repository(),
            "non-repo → an error, not an empty string"
        );
    }

    /// NEEDS A REAL REPOSITORY (temp checkout + the `git` binary).
    ///
    /// `run_text` over git's scalar output, and the tip-set walk seeing a commit
    /// that `HEAD` alone would also see — the baseline the co-keyed-checkout
    /// behaviour builds on.
    #[test]
    fn run_text_reads_scalar_output_from_a_real_repository() {
        let repo = tempfile::tempdir().unwrap();
        crate::tasks::test_support::git_init_repo(repo.path());
        crate::tasks::test_support::git_commit_on_day(
            repo.path(),
            "2026-01-02",
            &[("a.rs", "x\n")],
        );

        let shas =
            run_text(repo.path(), &["log", "--branches", "--remotes", "--tags", "--format=%H"])
                .expect("a repository with one commit walks cleanly");
        let shas: Vec<&str> = shas.lines().collect();
        assert_eq!(shas.len(), 1, "one commit on the tip set: {shas:?}");
        assert_eq!(shas[0].len(), 40, "a full sha: {:?}", shas[0]);

        // authored_at, not the committer date (#224): the fixture pins both, so
        // this asserts the FIELD is readable, not that the two differ.
        let authored = run_text(repo.path(), &["log", "-1", "--format=%aI"]).unwrap();
        assert!(authored.trim().starts_with("2026-01-02"), "authored date: {authored:?}");
    }

    /// NEEDS A REAL REPOSITORY (temp checkout + the `git` binary).
    ///
    /// Our own bad argv must be loud. `ambiguous argument` shares exit 128 with
    /// the ordinary not-a-repository case, so if the classifier keyed on the
    /// code instead of the words, a typo in the scanner's revision range would
    /// be silently filed as "this folder isn't a checkout" and the repository
    /// would never be scanned again.
    #[test]
    fn run_bytes_surfaces_a_bad_revision_as_a_loud_failure() {
        let repo = tempfile::tempdir().unwrap();
        crate::tasks::test_support::git_init_repo(repo.path());
        crate::tasks::test_support::git_commit_on_day(
            repo.path(),
            "2026-01-02",
            &[("a.rs", "x\n")],
        );

        let err = run_bytes(repo.path(), &["log", "--format=%H", "deadbeefdeadbeef"]).unwrap_err();
        assert_eq!(err.exit_code(), Some(128), "git's catch-all fatal code");
        assert!(!err.is_not_a_repository(), "this IS a repository — the argv is wrong: {err}");
        assert!(err.to_string().contains("ambiguous argument"), "{err}");
    }

    /// NEEDS A REAL REPOSITORY (temp checkout + the `git` binary).
    ///
    /// Why [`run_bytes`] exists at all, shown against the live defect. Without
    /// `-z`, `core.quotePath` (on by default) hands back the 16-byte LITERAL
    /// `"caf\303\251.rs"` — octal-escaped and wrapped in two real quote
    /// characters — and that string is what today's churn feed stores as a path
    /// (30 such records in this repository). With `-z` the same path arrives as
    /// its own bytes, NUL-terminated. A byte-returning runner is the only way
    /// the shared numstat parser gets to see the second form.
    #[test]
    fn run_bytes_returns_the_z_form_of_a_path_not_the_c_quoted_one() {
        let repo = tempfile::tempdir().unwrap();
        crate::tasks::test_support::git_init_repo(repo.path());
        crate::tasks::test_support::git_commit_on_day(
            repo.path(),
            "2026-01-02",
            &[("café.rs", "x\n")],
        );

        let zed = run_bytes(repo.path(), &["log", "-1", "--numstat", "-z", "--format="]).unwrap();
        assert_eq!(zed, b"1\t0\tcaf\xc3\xa9.rs\0", "`-z` emits the path's own bytes, NUL-ended");

        let quoted = run_bytes(repo.path(), &["log", "-1", "--numstat", "--format="]).unwrap();
        assert_eq!(
            quoted, b"1\t0\t\"caf\\303\\251.rs\"\n",
            "without `-z` the path is C-quoted — the stored-escapes defect, in one line"
        );
        assert_ne!(zed, quoted, "the two forms really are different bytes");
    }
}
