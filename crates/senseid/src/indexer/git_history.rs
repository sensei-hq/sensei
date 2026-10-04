//! Git history as repository facts — the ONE `--numstat` reader in this tree.
//!
//! The scanner behind `sensei.commits` / `sensei.commit_files` / `sensei.commit_scans`
//! (#224). This module owns two halves of one decision and keeps them in the same
//! file on purpose: the `git log` invocation, and the parser for the bytes that
//! invocation produces. A format and its parser that live apart drift, and the
//! drift is silent — the parser keeps returning records, they are just the wrong
//! records. That is precisely what happened to the reader this one replaces.
//!
//! ## What it replaces, and the measured defects
//!
//! `tasks::handlers::metrics::churn::parse_numstat_log` reads **non-`-z`** output
//! with `splitn(3, '\t')`. Measured over the live table, that reader produced:
//!
//! - **2,087 of 24,496 records (8.51%) are rename pseudo-paths stored verbatim**,
//!   e.g. `crates/senseid/src/indexer/{barrier.rs => quality/reachability.rs}`.
//!   Without `-z`, git collapses a rename into one brace-expanded display string.
//!   No such path exists on disk, so the record can never join `sensei.files`.
//! - **30 records are C-quoted octal paths**, e.g.
//!   `"docs/.../Sensei D\305\215j\305\215 App.html"` — git quotes any path with a
//!   non-ASCII or special byte when the output is newline-terminated. The stored
//!   path then carries the quotes and the escapes, and matches nothing.
//! - **`add.parse().unwrap_or(0) + del.parse().unwrap_or(0)`** turns a binary
//!   file's `-`/`-` — *churn UNKNOWN* — into a reported **0**. A fabricated zero
//!   on a failure path, and it silently deflates every concentration denominator.
//!
//! `-z` fixes the first two at the source (git emits raw bytes, and a rename
//! becomes three fields instead of one brace string); [`FileTouch::lines_changed`]
//! being `Option<i64>` fixes the third by refusing to invent a number.
//!
//! ## The invocation
//!
//! [`log_args`] builds it. Flag by flag:
//!
//! - **`--branches --remotes --tags` — the TIP SET.** Not `HEAD`, which is one
//!   checkout's opinion; not `--all`, which sweeps `refs/stash` and
//!   `refs/original` (rewrite backups) into the facts. Measured: under the tip
//!   set, 3 of 4 co-keyed checkout pairs on this machine produce *byte-identical*
//!   commit sets (symmetric difference 0; the 4th differs by 1.7%). That is the
//!   property the whole design rests on — facts are keyed per REPOSITORY while
//!   scans run per CHECKOUT, so two checkouts of one repository must agree about
//!   what they saw. Verified side effect: a commit reachable only from a detached
//!   `HEAD` (mid-rebase, mid-bisect) is excluded. It is a transient per-checkout
//!   state, not a shared fact.
//! - **`--no-merges`.** `git log` shows no diff for a merge by default, so a
//!   merge would arrive as a commit with no touches; and any `--diff-merges`
//!   mode that gave it touches would re-attribute work the parents already
//!   carry. Excluded, so neither happens. ([`parse_log`] still handles a
//!   file-less commit correctly — `--allow-empty` and mode-only changes produce
//!   one too.)
//! - **`-z`.** NUL-terminated records, no path quoting, raw path bytes.
//! - **`--no-show-signature`.** The one piece of user configuration that can
//!   corrupt this stream. With `log.showSignature=true` — which plenty of people
//!   set — a **signed** commit makes `git log` print a verification line *before*
//!   the header, measured verbatim on git 2.50.1:
//!   `Good "git" signature for a@example.com with ED25519 key SHA256:…` followed by
//!   `LF`. That lands in field 0 ahead of the sentinel, so every scan on that
//!   machine would fail. It fails *loudly* — [`GitLogError::Desync`] at field 0,
//!   not a mis-parse — but it would fail, so the flag pins it off.
//! - **`-M`.** Rename detection, which is what makes the post-image path
//!   available as its own field instead of a brace string.
//! - **`--numstat`.** Per-file added/deleted, `-`/`-` for binary.
//! - **`--pretty=format:%x01%H%x00%aI%x00%ae`.** See the grammar below.
//!   `%aI` is the **author** date in strict ISO-8601, and it is deliberately not
//!   the committer date: an author date survives rebase and cherry-pick, so one
//!   logical change buckets to the same day through two checkouts of the same
//!   repository. **This DIVERGES from `churn.rs`, which buckets on the committer
//!   date (`%cd`).** Named here rather than hidden: when `churn.rs` migrates onto
//!   this parser (#224 step 5) its day bucketing moves with it, and a rebased
//!   commit will change which day it lands on. `%aI` also ignores `--date`, so
//!   there is no date-format flag to keep in sync.
//! - **`^<sha>` per entry of the previous `commit_scans.tips`.** The cursor.
//!   Each must already be present in the object database; a tip that was
//!   garbage-collected after a force-push makes `git log` fail with
//!   `bad revision`, so the caller checks `git cat-file -e <sha>^{commit}` first
//!   and drops the missing ones (re-walking is cheap and insert-only; guessing is
//!   not). [`tips_args`] builds the companion command that reads the new tips, so
//!   the walk and the cursor cannot disagree about what "the tip set" means.
//!
//! Flags deliberately **absent**, because they are unnecessary and a redundant
//! flag reads like a guard where there is no hazard: `--no-notes` and
//! `--no-decorate`. Measured on git 2.50.1 against an explicit
//! `--pretty=format:<custom>`, a commit with a `git note` actually attached
//! (`%N` non-empty) and `log.decorate=full` each inject **nothing**. Notes and
//! decoration reach a custom format only through `%N` and `%d`, which this
//! format does not use. The signature does not go through the format at all,
//! which is exactly why it is the one that bites.
//!
//! ## The stream grammar (measured, git 2.50.1)
//!
//! ```text
//! stream       := commit ( NUL commit )*            -- `format:` separates, it does not terminate
//! commit       := SOH sha NUL authored_at NUL email [ LF entry ( NUL entry )* NUL ]
//! entry        := added TAB deleted TAB path        -- ordinary
//!               | added TAB deleted TAB NUL from NUL to   -- rename (-M); pre-image FIRST
//! added/deleted:= digits | "-"                      -- "-" is BINARY, i.e. UNKNOWN
//! ```
//!
//! Read as NUL-separated fields, that is:
//!
//! | field | ordinary commit            | file-less commit (merge, `--allow-empty`) |
//! |-------|----------------------------|-------------------------------------------|
//! | 0     | `\x01` + sha               | `\x01` + sha                               |
//! | 1     | author date                | author date                                |
//! | 2     | `email` `\n` *first entry*  | `email` (ends here)                       |
//! | 3..   | further entries            | — next commit's field 0                    |
//! | last  | *empty* (the separator)    | — no empty field at all                    |
//!
//! Two shapes worth pinning, both verified by `od -c` against this repository:
//!
//! - A rename's counts and its *empty* path share one field — `50\t40\t` — and the
//!   pre-image and post-image follow as two further fields, **old first**.
//! - A file-less commit has no empty separator field: the separator NUL terminates
//!   the email field itself, and the very next field is the next commit's header.
//!
//! ## Why the commit header is unambiguous
//!
//! It is **not** because of the `\x01` sentinel. A git path may begin with any
//! byte, `\x01` included, so testing a field's first byte is not a record
//! discriminator — it is a guess that works until someone commits a file whose
//! name starts with SOH. [`parse_log`] therefore never asks "does this field look
//! like a header". It tracks **state**, and the grammar makes the state total:
//!
//! 1. At the start of the stream, and after a commit ends, the next field *is* a
//!    header. Nothing else can be there.
//! 2. A header is exactly three fields. The third carries the email, and whether
//!    it contains an `LF` decides — with no ambiguity and no lookahead — whether
//!    this commit has a file list at all. An email cannot contain `LF`; a numstat
//!    entry cannot begin with one; so splitting that field at its **first** `LF`
//!    is total.
//! 3. Inside a file list, an **empty** field is the commit separator. A git path
//!    is never empty, and a rename's empty path lives *inside* the non-empty
//!    `added TAB deleted TAB` field, so "empty" here can mean nothing else.
//!
//! The sentinel survives as an **assertion**, not a discriminator: by (1) the
//! parser already knows the field is a header, so a missing `\x01` means the
//! state machine has desynchronised from the stream, and that is
//! [`GitLogError::Desync`] — loud, never a salvaged half-record.
//!
//! ## What this module does NOT do
//!
//! - **It does not prune.** The three tables are insert-only. v1 pruned by
//!   `rev-list HEAD` reachability, which was unsound: facts are repository-grain
//!   and scans are per checkout, and 10 repositories on this machine have more
//!   than one anchor folder — they would prune and re-walk each other forever.
//!   The prune was redundant anyway: 4,298 of 6,654 paths ever touched in sensei
//!   (65%) no longer exist, so they have no `sensei.files` row and cannot reach a
//!   diagram. **Existence on disk is the filter, not reachability.**
//! - **It does not apply the bulk-commit cap.** The settled cap of 50 files
//!   belongs to the *co-change derivation*, not to the scan: naive co-change over
//!   sensei yields 588,865 pairs, 81.5% of them from 53 commits touching >50
//!   files. The commit and its 326 touches are still facts and are still stored.
//! - **It does not decide what a non-UTF-8 path becomes.** [`GitBytes`] keeps the
//!   bytes git gave, losslessly. `String::from_utf8_lossy` would mint a path that
//!   equals nothing real — a plausible-but-wrong value on a failure path — so the
//!   writer calls [`GitBytes::as_str`] and handles the `None` explicitly.
//!
//! ## Scale, and this parser measured against the defects it replaces
//!
//! Machine-wide: ~121,780 commits and ~738,000 `commit_files` rows, about
//! 185 MB. Running this module over sensei's whole tip set produces 1.53 MB of
//! stdout in one `git log`, so the caller can hold a repository's history in
//! memory; a repository an order of magnitude larger wants streaming.
//!
//! Every count below was taken twice — once through [`parse_log`], once through
//! an independent `git log … | awk` pass over the NON-`-z` output that
//! `churn.rs` reads today. They agree exactly, which is what says this parser
//! neither drops nor duplicates a record:
//!
//! | over sensei's tip set | `awk` over non-`-z` | [`parse_log`] over `-z` |
//! |-----------------------|---------------------|-------------------------|
//! | commits               | 4,064               | 4,064                   |
//! | touches               | 24,592              | 24,592                  |
//! | Σ added+deleted       | 5,466,374           | 5,466,374               |
//! | rename records        | 2,087 brace pseudo-paths | 2,087 real post-image paths |
//! | binary records        | 459 reported as `0` | 459 reported as `None`  |
//! | C-quoted paths        | **30**              | **0**                   |
//!
//! The 2,087 and the 30 are the live-table defect counts quoted above,
//! reproduced from git rather than from the table — so the fix is measured
//! against the same quantity the defect was. (4,064/24,592 against the brief's
//! 4,060/24,588: four commits have landed since that measurement.)

use std::fmt;

use chrono::{DateTime, FixedOffset, NaiveDate};

/// The byte `--pretty=format:%x01…` puts in front of every commit header.
///
/// An integrity assertion, not a record discriminator — see the module docs.
pub const COMMIT_SENTINEL: u8 = 0x01;

/// The field separator `-z` uses, for both header fields and numstat records.
const FIELD_SEP: u8 = 0x00;

/// The numstat marker for a binary file, in either count column. It means the
/// line churn is **UNKNOWN**, which is not the same fact as zero.
const BINARY_MARKER: &[u8] = b"-";

/// The commit header format. `%x01` here and [`COMMIT_SENTINEL`] are the same
/// byte written twice, and `the_pretty_format_and_the_sentinel_cannot_drift`
/// holds them together.
pub const PRETTY_FORMAT: &str = "--pretty=format:%x01%H%x00%aI%x00%ae";

/// The ref namespaces that make up THE TIP SET, as `git log` pseudo-refs.
///
/// Paired with [`TIP_REF_GLOBS`], which names the same three namespaces in the
/// spelling `git for-each-ref` wants. One definition of "the tip set", used by
/// both the walk and the cursor that bounds the next walk.
const TIP_PSEUDO_REFS: &[&str] = &["--branches", "--remotes", "--tags"];

/// The same tip set, for `git for-each-ref`.
const TIP_REF_GLOBS: &[&str] = &["refs/heads", "refs/remotes", "refs/tags"];

/// Everything before the cursor exclusions in [`log_args`].
const BASE_LOG_ARGS: &[&str] =
    &["log", "--no-merges", "--no-show-signature", "-z", "-M", "--numstat", PRETTY_FORMAT];

// ── types ────────────────────────────────────────────────────────────────────

/// A byte string exactly as git produced it.
///
/// Git guarantees no encoding for a path or for an author identity — a path is
/// an arbitrary non-empty byte sequence with no `/` constraint beyond separators,
/// and the author line of a commit object is copied through verbatim. Under `-z`
/// git stops quoting, so those bytes arrive raw, and the only lossless thing to
/// hold them in is bytes.
///
/// `as_str` returns `Option<&str>` rather than lossily converting, so the
/// decision about a path that cannot go into a `text` column is taken once, by
/// the writer, in the open.
#[derive(Clone, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct GitBytes(Vec<u8>);

impl GitBytes {
    /// The bytes, as git produced them.
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// The bytes as UTF-8, or `None` when they are not UTF-8.
    ///
    /// Never lossy. A `None` is a real answer the caller must act on — skip the
    /// touch and count it — not a prompt to substitute replacement characters.
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.0).ok()
    }
}

impl From<&[u8]> for GitBytes {
    fn from(raw: &[u8]) -> Self {
        Self(raw.to_vec())
    }
}

/// UTF-8 renders as a quoted string; anything else as an escaped byte string, so
/// a failing assertion over a non-UTF-8 path shows the bytes instead of a row of
/// replacement characters.
impl fmt::Debug for GitBytes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(s) = self.as_str() {
            return write!(f, "{s:?}");
        }
        write!(f, "b\"")?;
        for &b in &self.0 {
            match b {
                b'\\' => write!(f, "\\\\")?,
                b'"' => write!(f, "\\\"")?,
                0x20..=0x7e => write!(f, "{}", b as char)?,
                _ => write!(f, "\\x{b:02x}")?,
            }
        }
        write!(f, "\"")
    }
}

impl PartialEq<&str> for GitBytes {
    fn eq(&self, other: &&str) -> bool {
        self.0 == other.as_bytes()
    }
}

/// One file touched by one commit — one `sensei.commit_files` row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileTouch {
    /// The **post-image** path: where the file is after this commit. For a
    /// rename that is the new path, never the `{old => new}` brace string the
    /// non-`-z` reader stored for 2,087 records.
    pub path: GitBytes,

    /// `added + deleted`, or `None` when git reported `-` for either count.
    ///
    /// `None` is *unknown*, which is what a binary file's churn is. The column is
    /// nullable for exactly this reason. Writing 0 here — what `churn.rs:234`
    /// does today via `unwrap_or(0)` — is a fabricated measurement: it is
    /// indistinguishable from a real whitespace-only diff and it deflates every
    /// denominator computed over the column.
    pub lines_changed: Option<i64>,

    /// The pre-image path when `-M` detected a rename, else `None`.
    ///
    /// Not a `commit_files` column — that table stores one row per touched path
    /// and the touched path is the post-image. It is kept because the parser's
    /// job is to lose nothing its input carried, and because following a rename
    /// chain is what lets co-change survive a file being moved.
    pub renamed_from: Option<GitBytes>,
}

/// One commit and everything it touched — one `sensei.commits` row plus its
/// `sensei.commit_files` rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Commit {
    /// The full object name from `%H`, validated as 40 (SHA-1) or 64 (SHA-256)
    /// hex digits.
    pub sha: String,

    /// The **author** date from `%aI`, with the author's own UTC offset kept.
    /// See the module docs for why this is not the committer date.
    pub authored_at: DateTime<FixedOffset>,

    /// `%ae`, or `None` when the commit object's author line carries `<>`.
    ///
    /// An empty email is an absent identity, not an identity that is the empty
    /// string, and `sensei.commits.author_email` is nullable to say so.
    pub author_email: Option<GitBytes>,

    /// Every file this commit touched. Empty for a commit that changed nothing.
    pub files: Vec<FileTouch>,
}

impl Commit {
    /// The calendar day **in the author's own offset**.
    ///
    /// Not `naive_utc().date()`: a commit authored at 01:00+09:00 happened on
    /// that day for the person who made it, and converting to UTC would file it
    /// under the day before. The day bucket is a human-activity bucket.
    pub fn authored_day(&self) -> NaiveDate {
        self.authored_at.date_naive()
    }
}

/// Every way reading a git history can fail. No variant has a salvage path:
/// a stream this module cannot account for is an error the caller surfaces, so
/// that a partial or mis-attributed history can never be written as if it were
/// complete.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitLogError {
    /// A cursor entry from `commit_scans.tips` is not an object name. Rejected
    /// before it reaches the command line: a value beginning with `-` would be
    /// read by git as a flag.
    InvalidCursorSha(String),

    /// A field that the grammar says must be a commit header does not start with
    /// [`COMMIT_SENTINEL`]. The parser and the stream have lost sync; everything
    /// after this point would be mis-attributed.
    Desync { field_index: usize, found: GitBytes },

    /// The stream ended part-way through a three-field commit header.
    TruncatedHeader { after_sha: Option<String> },

    /// `%H` was not 40 or 64 hex digits.
    MalformedSha(GitBytes),

    /// `%aI` did not parse as a strict ISO-8601 instant.
    MalformedAuthoredAt { sha: String, raw: GitBytes },

    /// A numstat entry was not `added TAB deleted TAB path`, or a count was
    /// neither a decimal number nor `-`. Distinct from binary-`-`: an
    /// unrecognised count means the field is not what we think it is, and
    /// calling it "unknown churn" would bury a desync as a legitimate NULL.
    MalformedEntry { sha: String, raw: GitBytes },

    /// A rename entry promised a pre-image and a post-image field and the stream
    /// ended first.
    TruncatedRename { sha: String },

    /// A rename's pre-image or post-image field was empty. Git paths are not.
    EmptyPath { sha: String },
}

impl fmt::Display for GitLogError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidCursorSha(s) => {
                write!(f, "cursor {s:?} is not a git object name")
            }
            Self::Desync { field_index, found } => write!(
                f,
                "field {field_index} should be a commit header but is {found:?} \
                 — the parser has desynchronised from the git log stream"
            ),
            Self::TruncatedHeader { after_sha } => match after_sha {
                Some(sha) => write!(f, "git log stream ended inside the header of {sha}"),
                None => write!(f, "git log stream ended inside a commit header"),
            },
            Self::MalformedSha(raw) => write!(f, "{raw:?} is not a 40- or 64-hex object name"),
            Self::MalformedAuthoredAt { sha, raw } => {
                write!(f, "{sha}: author date {raw:?} is not strict ISO-8601")
            }
            Self::MalformedEntry { sha, raw } => {
                write!(f, "{sha}: numstat entry {raw:?} is not `added TAB deleted TAB path`")
            }
            Self::TruncatedRename { sha } => {
                write!(f, "{sha}: git log stream ended inside a rename's path pair")
            }
            Self::EmptyPath { sha } => write!(f, "{sha}: a rename named an empty path"),
        }
    }
}

impl std::error::Error for GitLogError {}

// ── the invocation ───────────────────────────────────────────────────────────

/// A 40- or 64-hex-digit git object name.
///
/// The one predicate behind both the `%H` check and the cursor check, so a sha
/// this module will parse is exactly a sha it will put on a command line.
fn is_object_name(s: &str) -> bool {
    matches!(s.len(), 40 | 64) && s.bytes().all(|b| b.is_ascii_hexdigit())
}

/// The `git` arguments that walk the tip set, newest first, stopping at the
/// cursor.
///
/// `stop_at` is the previous scan's `commit_scans.tips`: each becomes `^<sha>`,
/// so the walk covers only what has appeared since. An empty `stop_at` walks the
/// whole tip set. Every entry must still exist in the object database — see the
/// module docs.
///
/// Returns `Err` rather than dropping a bad cursor entry: silently skipping it
/// would turn an incremental scan into a full re-walk with nothing saying so, and
/// an entry starting with `-` would be parsed by git as a flag.
pub fn log_args<S: AsRef<str>>(stop_at: &[S]) -> Result<Vec<String>, GitLogError> {
    let mut args: Vec<String> =
        BASE_LOG_ARGS.iter().chain(TIP_PSEUDO_REFS).map(|a| (*a).to_string()).collect();
    for sha in stop_at {
        let sha = sha.as_ref();
        if !is_object_name(sha) {
            return Err(GitLogError::InvalidCursorSha(sha.to_string()));
        }
        args.push(format!("^{sha}"));
    }
    Ok(args)
}

/// The `git` arguments that read the tip set's current object names — the value
/// to store back into `commit_scans.tips` after a successful scan.
///
/// Defined here, beside [`log_args`], because "the tip set" has to mean the same
/// three ref namespaces in both places. The caller sorts and de-duplicates the
/// output: the column is a SET, and `refs/remotes/origin/HEAD` is a symref that
/// repeats another ref's object name.
pub fn tips_args() -> Vec<String> {
    let mut args = vec!["for-each-ref".to_string(), "--format=%(objectname)".to_string()];
    args.extend(TIP_REF_GLOBS.iter().map(|g| (*g).to_string()));
    args
}

// ── the parser ───────────────────────────────────────────────────────────────

/// The NUL-separated fields of a `-z` stream, with each field's ordinal.
///
/// Separated, not terminated: an empty input has **no** fields, and a stream
/// ending in NUL has one trailing empty field — which is exactly the commit
/// separator the file-list state is looking for.
struct NulFields<'a> {
    rest: Option<&'a [u8]>,
    index: usize,
}

impl<'a> NulFields<'a> {
    fn new(raw: &'a [u8]) -> Self {
        Self { rest: (!raw.is_empty()).then_some(raw), index: 0 }
    }

    /// Named `next_field` rather than `next` because this is not an `Iterator`:
    /// the parser's two states pull from it at different arities (a header takes
    /// three, a rename entry takes two more), and an `Iterator` would invite a
    /// combinator that silently consumes past a commit boundary.
    fn next_field(&mut self) -> Option<(usize, &'a [u8])> {
        let rest = self.rest?;
        let index = self.index;
        self.index += 1;
        Some(match rest.iter().position(|&b| b == FIELD_SEP) {
            Some(at) => {
                self.rest = Some(&rest[at + 1..]);
                (index, &rest[..at])
            }
            None => {
                self.rest = None;
                (index, rest)
            }
        })
    }
}

/// Split at the first `needle`, returning what precedes it and what follows, or
/// `None` for the tail when the byte is absent.
fn split_first(hay: &[u8], needle: u8) -> (&[u8], Option<&[u8]>) {
    match hay.iter().position(|&b| b == needle) {
        Some(at) => (&hay[..at], Some(&hay[at + 1..])),
        None => (hay, None),
    }
}

/// One numstat count column: a decimal number, or `-` for binary.
///
/// `Ok(None)` is binary — the count is unknown. `Err(())` is "this is not a count
/// at all", which the caller turns into [`GitLogError::MalformedEntry`]. Keeping
/// those two apart is the whole point: collapsing them would let a desynchronised
/// field arrive in the database as a legitimate NULL.
fn parse_count(raw: &[u8]) -> Result<Option<i64>, ()> {
    if raw == BINARY_MARKER {
        return Ok(None);
    }
    if raw.is_empty() || !raw.iter().all(u8::is_ascii_digit) {
        return Err(());
    }
    std::str::from_utf8(raw).map_err(|_| ())?.parse::<i64>().map(Some).map_err(|_| ())
}

/// Parse one numstat entry, pulling the two extra fields a rename needs from the
/// same cursor the caller is reading.
///
/// A rename takes its extra fields from `fields` rather than from a lookahead
/// buffer so that there is exactly one position in the stream, shared by the
/// commit's first entry (which arrives glued to the header by `LF`) and all the
/// rest. Two readers here is how a rename at the head of a commit leaks its
/// pre-image out as a phantom second file.
fn parse_entry<'a>(
    sha: &str,
    field: &'a [u8],
    fields: &mut NulFields<'a>,
) -> Result<FileTouch, GitLogError> {
    let malformed = || GitLogError::MalformedEntry { sha: sha.to_string(), raw: field.into() };

    let (added, after_added) = split_first(field, b'\t');
    let (deleted, path) = split_first(after_added.ok_or_else(malformed)?, b'\t');
    let path = path.ok_or_else(malformed)?;

    // `-` on EITHER side makes the sum unknown. Git only ever emits `-`/`-`
    // together, but a half-binary entry would still be an unknown total, and
    // summing the known half would be a number nobody measured.
    let lines_changed = match (
        parse_count(added).map_err(|()| malformed())?,
        parse_count(deleted).map_err(|()| malformed())?,
    ) {
        (Some(a), Some(d)) => Some(a.checked_add(d).ok_or_else(malformed)?),
        _ => None,
    };

    if !path.is_empty() {
        return Ok(FileTouch { path: path.into(), lines_changed, renamed_from: None });
    }

    // Rename form: the path field is empty and the pre-image and post-image
    // follow as two further fields, OLD FIRST. An empty path field cannot mean
    // anything else — a git path is never empty.
    let truncated = || GitLogError::TruncatedRename { sha: sha.to_string() };
    let (_, from) = fields.next_field().ok_or_else(truncated)?;
    let (_, to) = fields.next_field().ok_or_else(truncated)?;
    if from.is_empty() || to.is_empty() {
        return Err(GitLogError::EmptyPath { sha: sha.to_string() });
    }
    Ok(FileTouch { path: to.into(), lines_changed, renamed_from: Some(from.into()) })
}

/// Parse the stdout of the command [`log_args`] builds.
///
/// Pure over bytes, so the whole grammar is testable without a subprocess, and
/// over `&[u8]` rather than `&str` because a path is not text — see [`GitBytes`].
///
/// The walk is state-driven end to end; no byte test ever decides whether a field
/// is a header, because a path may begin with any byte including
/// [`COMMIT_SENTINEL`]. See the module docs for why the three states are total.
pub fn parse_log(stdout: &[u8]) -> Result<Vec<Commit>, GitLogError> {
    let mut fields = NulFields::new(stdout);
    let mut commits = Vec::new();

    // STATE: header. The stream starts here, and every commit returns here.
    while let Some((index, head)) = fields.next_field() {
        let Some(raw_sha) = head.strip_prefix(&[COMMIT_SENTINEL]) else {
            return Err(GitLogError::Desync { field_index: index, found: head.into() });
        };
        let sha = std::str::from_utf8(raw_sha)
            .ok()
            .filter(|s| is_object_name(s))
            .ok_or_else(|| GitLogError::MalformedSha(raw_sha.into()))?
            .to_string();

        let truncated = || GitLogError::TruncatedHeader { after_sha: Some(sha.clone()) };
        let (_, raw_when) = fields.next_field().ok_or_else(truncated)?;
        let authored_at = std::str::from_utf8(raw_when)
            .ok()
            .and_then(|w| DateTime::parse_from_rfc3339(w).ok())
            .ok_or_else(|| GitLogError::MalformedAuthoredAt {
                sha: sha.clone(),
                raw: raw_when.into(),
            })?;

        // The third header field is `email` or `email LF <first entry>`. Split at
        // the FIRST LF: an email cannot contain one, so everything after it
        // belongs to the diff — including a path that contains LF itself, which
        // stays intact because only the first is consumed.
        let (_, tail) = fields.next_field().ok_or_else(truncated)?;
        let (raw_email, first_entry) = split_first(tail, b'\n');
        let author_email = (!raw_email.is_empty()).then(|| GitBytes::from(raw_email));

        let mut files = Vec::new();
        // No LF means no diff followed the header, and the separator NUL has
        // already been consumed as this field's terminator — so the next field is
        // the next commit's header. Consuming one more here is what makes a
        // merge-only log swallow every second commit.
        if let Some(first) = first_entry {
            // Tolerate a blank line between header and diff without a byte test
            // on a path: a numstat entry begins with a digit or `-`, never LF, so
            // leading LF here can only be padding.
            let first = first.strip_prefix(b"\n").unwrap_or(first);
            if !first.is_empty() {
                files.push(parse_entry(&sha, first, &mut fields)?);

                // STATE: file list. Ends at the empty field `format:` writes
                // between commits, or at the end of the stream.
                while let Some((_, field)) = fields.next_field() {
                    if field.is_empty() {
                        break;
                    }
                    files.push(parse_entry(&sha, field, &mut fields)?);
                }
            }
        }

        commits.push(Commit { sha, authored_at, author_email, files });
    }

    Ok(commits)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a byte fixture from already-NUL-free pieces. The pieces are written
    /// out literally in each test — this only joins them — so no test can agree
    /// with the parser by re-deriving the grammar the parser implements.
    fn nul_join(parts: &[&[u8]]) -> Vec<u8> {
        parts.join(&FIELD_SEP)
    }

    fn day(y: i32, m: u32, d: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, d).expect("a real calendar day")
    }

    // ── the grammar, one shape per test ──────────────────────────────────────

    #[test]
    fn a_plain_commit_yields_its_touches_with_added_plus_deleted() {
        // MUTATION THAT MUST BREAK IT: in `parse_entry`, return `Some(a)` instead
        // of `Some(a.checked_add(d)?)` — the deleted column stops counting.
        let log = nul_join(&[
            b"\x01aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            b"2020-01-01T09:30:00+00:00",
            b"dev@example.com\n3\t1\ta.rs",
            b"0\t2\tsrc/b.rs",
        ]);

        let commits = parse_log(&log).expect("a well-formed stream parses");
        assert_eq!(commits.len(), 1, "one header, one commit");
        let c = &commits[0];
        assert_eq!(c.sha, "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
        assert_eq!(c.author_email.as_ref().map(GitBytes::as_bytes), Some(&b"dev@example.com"[..]));
        assert_eq!(c.files.len(), 2, "two numstat entries, two touches");
        assert_eq!(c.files[0].path, "a.rs");
        assert_eq!(c.files[0].lines_changed, Some(4), "3 added + 1 deleted");
        assert_eq!(c.files[0].renamed_from, None, "an ordinary entry is not a rename");
        assert_eq!(c.files[1].path, "src/b.rs");
        assert_eq!(c.files[1].lines_changed, Some(2), "0 added + 2 deleted");
    }

    #[test]
    fn a_rename_keeps_the_post_image_and_records_the_pre_image() {
        // MUTATION THAT MUST BREAK IT: swap `from`/`to` in `parse_entry`'s rename
        // branch — git emits the PRE-image first, so the stored path becomes the
        // path that no longer exists.
        let log = nul_join(&[
            b"\x01bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
            b"2020-02-02T00:00:00+00:00",
            b"dev@example.com\n1\t1\tkeep.rs",
            b"38\t24\t",
            b"crates/senseid/src/indexer/barrier.rs",
            b"crates/senseid/src/indexer/quality/reachability.rs",
            b"2\t0\tafter.rs",
        ]);

        let c = &parse_log(&log).expect("a rename parses")[0];
        assert_eq!(c.files.len(), 3, "a rename is ONE touch, not two and not zero");
        let renamed = &c.files[1];
        assert_eq!(
            renamed.path, "crates/senseid/src/indexer/quality/reachability.rs",
            "the stored path is the POST-image — the file that exists now",
        );
        assert_eq!(
            renamed.renamed_from.as_ref().map(GitBytes::as_bytes),
            Some(&b"crates/senseid/src/indexer/barrier.rs"[..]),
            "the pre-image is kept, not discarded",
        );
        assert_eq!(renamed.lines_changed, Some(62), "38 + 24");
        assert!(
            !renamed.path.as_str().expect("utf-8 fixture").contains("=>"),
            "no brace pseudo-path survives -z — this is the 8.51% defect",
        );
        assert_eq!(c.files[2].path, "after.rs", "the entry after a rename is not swallowed");
    }

    #[test]
    fn a_binary_file_has_unknown_churn_not_zero_churn() {
        // MUTATION THAT MUST BREAK IT: make `parse_count` return `Ok(Some(0))`
        // for `b"-"` — i.e. reinstate churn.rs:234's `unwrap_or(0)`.
        let log = nul_join(&[
            b"\x01cccccccccccccccccccccccccccccccccccccccc",
            b"2020-03-03T00:00:00+00:00",
            b"dev@example.com\n-\t-\tassets/logo.png",
            b"0\t0\ttouched-but-unchanged.rs",
        ]);

        let c = &parse_log(&log).expect("a binary entry parses")[0];
        assert_eq!(
            c.files[0].lines_changed, None,
            "a binary file's churn is UNKNOWN; 0 would be a fabricated measurement",
        );
        assert_eq!(c.files[0].path, "assets/logo.png", "it is still a touched file");
        assert_eq!(
            c.files[1].lines_changed,
            Some(0),
            "a REAL 0/0 diff is a real zero, and must stay distinguishable from unknown",
        );
    }

    #[test]
    fn a_non_utf8_path_survives_byte_for_byte() {
        // MUTATION THAT MUST BREAK IT: store the path as
        // `String::from_utf8_lossy(path).into_owned()` — every non-UTF-8 byte
        // becomes U+FFFD and the path matches nothing on disk.
        //
        // The bytes are a real latin-1 `déjà.txt`, which `git log` WITHOUT -z
        // emits as the C-quoted `"d\351j\340.txt"` (measured) — the 30-record
        // defect.
        let mut log = Vec::new();
        log.extend_from_slice(b"\x01dddddddddddddddddddddddddddddddddddddddd\x00");
        log.extend_from_slice(b"2020-04-04T00:00:00+00:00\x00");
        log.extend_from_slice(b"dev@example.com\n1\t0\td\xe9j\xe0.txt\x00");

        let c = &parse_log(&log).expect("a non-utf-8 path parses")[0];
        assert_eq!(
            c.files[0].path.as_bytes(),
            b"d\xe9j\xe0.txt",
            "the raw bytes git gave, unaltered and unquoted",
        );
        assert_eq!(
            c.files[0].path.as_str(),
            None,
            "as_str reports honestly that it is not UTF-8 — the writer must decide",
        );
        assert!(
            !format!("{:?}", c.files[0].path).contains('\u{fffd}'),
            "not even the Debug rendering fabricates replacement characters",
        );
    }

    #[test]
    fn a_commit_whose_first_entry_is_a_rename() {
        // MUTATION THAT MUST BREAK IT: in `parse_log`, parse the LF-glued first
        // entry with a reader that cannot pull further fields (e.g. inline
        // `added/deleted/path` handling with no rename branch) — the pre-image
        // then surfaces as a phantom second file and every subsequent field is
        // off by one.
        //
        // The first entry arrives glued to the header by LF, so it is the one
        // entry reached by a different code path. It must use the SAME cursor.
        let log = nul_join(&[
            b"\x01eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            b"2020-05-05T00:00:00+00:00",
            b"dev@example.com\n7\t3\t",
            b"old/name.rs",
            b"new/name.rs",
            b"5\t5\tsecond.rs",
        ]);

        let c = &parse_log(&log).expect("a leading rename parses")[0];
        assert_eq!(c.files.len(), 2, "a leading rename is ONE touch plus the ordinary entry");
        assert_eq!(c.files[0].path, "new/name.rs");
        assert_eq!(
            c.files[0].renamed_from.as_ref().map(GitBytes::as_bytes),
            Some(&b"old/name.rs"[..]),
        );
        assert_eq!(c.files[0].lines_changed, Some(10), "7 + 3");
        assert_eq!(c.files[1].path, "second.rs", "the stream is still in sync afterwards");
        assert_eq!(c.files[1].renamed_from, None);
    }

    #[test]
    fn an_empty_log_is_no_commits_and_not_an_error() {
        // MUTATION THAT MUST BREAK IT: drop the `is_empty` guard in
        // `NulFields::new` — an empty input then yields one empty field, which
        // the header state reports as a Desync.
        //
        // Reachable in production: `git log --branches --remotes --tags` in a
        // repository with no refs exits 0 with empty stdout (measured), and so
        // does an incremental walk whose cursor already covers every tip.
        assert_eq!(parse_log(b"").expect("an empty stream is a valid empty history"), vec![]);
    }

    #[test]
    fn a_merge_only_log_yields_commits_with_no_touches() {
        // MUTATION THAT MUST BREAK IT: in `parse_log`, consume a field after the
        // header unconditionally instead of only when the header's third field
        // contains LF — the second commit's header is then eaten as a numstat
        // entry, and the parse fails or mis-attributes.
        //
        // The scan passes --no-merges so this shape should not arrive, but a
        // file-less commit is also produced by `--allow-empty` and by a
        // mode-only change, and a commit that changed nothing must never absorb
        // the NEXT commit's files. Measured: a file-less commit emits no LF and
        // no empty separator field — the separator NUL terminates the email.
        let log = nul_join(&[
            b"\x01f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0f0",
            b"2020-06-06T00:00:00+00:00",
            b"merger@example.com",
            b"\x01f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1",
            b"2020-06-07T00:00:00+00:00",
            b"merger@example.com",
            b"\x01f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2f2",
            b"2020-06-08T00:00:00+00:00",
            b"dev@example.com\n1\t0\treal.rs",
        ]);

        let commits = parse_log(&log).expect("file-less commits parse");
        assert_eq!(commits.len(), 3, "three headers, three commits");
        assert!(commits[0].files.is_empty(), "a file-less commit touches nothing");
        assert!(commits[1].files.is_empty(), "two in a row still touch nothing");
        assert_eq!(commits[1].sha, "f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1f1");
        assert_eq!(
            commits[2].files.len(),
            1,
            "the following commit keeps its own file — nothing was absorbed",
        );
        assert_eq!(commits[2].files[0].path, "real.rs");
    }

    // ── the discriminator rules the design turns on ──────────────────────────

    #[test]
    fn a_path_that_begins_with_the_sentinel_byte_is_a_path() {
        // MUTATION THAT MUST BREAK IT: in `parse_entry`'s rename branch, decide
        // the pair has ended with a byte test instead of taking it by state —
        // `fields.next_field().filter(|(_, f)| !f.starts_with(&[COMMIT_SENTINEL]))`
        // — which reports a truncated rename for the perfectly legal paths below.
        //
        // This is the whole reason the parser is state-driven: a git path may
        // begin with any byte, SOH included. A rename's pre-image and post-image
        // are the ONLY fields in the grammar that are a bare path — everywhere
        // else a path is preceded by `added TAB deleted TAB` in the same field —
        // so they are the only position where a sentinel byte test could be
        // written at all, and the only position where it would silently corrupt
        // a real history. An earlier version of this fixture put the SOH path in
        // the two non-bare positions only, and passed against its own mutation.
        //
        // This is also the only multi-commit fixture with a file list, so it
        // pins the empty separator field that `format:` writes between two
        // file-bearing commits.
        let log = nul_join(&[
            b"\x019999999999999999999999999999999999999999",
            b"2020-07-07T00:00:00+00:00",
            b"dev@example.com\n1\t0\t\x01first-entry.rs",
            b"2\t0\t\x01later-entry.rs",
            b"5\t5\t",
            b"\x01old-soh.rs",
            b"\x01new-soh.rs",
            b"3\t0\tplain.rs",
            b"", // the commit separator
            b"\x018888888888888888888888888888888888888888",
            b"2020-07-08T00:00:00+00:00",
            b"dev@example.com\n4\t0\tnext.rs",
        ]);

        let commits = parse_log(&log).expect("a path starting with SOH is still a path");
        assert_eq!(commits.len(), 2, "two headers — no SOH path became a phantom commit");

        let files = &commits[0].files;
        assert_eq!(files.len(), 4, "four touches: two ordinary, one rename, one plain");
        assert_eq!(
            files[0].path.as_bytes(),
            b"\x01first-entry.rs",
            "an SOH path reached through the header split stays a path",
        );
        assert_eq!(
            files[1].path.as_bytes(),
            b"\x01later-entry.rs",
            "an SOH path reached through the file-list loop stays a path",
        );
        assert_eq!(
            files[2].path.as_bytes(),
            b"\x01new-soh.rs",
            "a rename POST-image that is a bare field beginning with SOH stays a path",
        );
        assert_eq!(
            files[2].renamed_from.as_ref().map(GitBytes::as_bytes),
            Some(&b"\x01old-soh.rs"[..]),
            "and so does the bare PRE-image field",
        );
        assert_eq!(files[3].path, "plain.rs", "the stream is still in sync after the rename");

        assert_eq!(commits[1].sha, "8888888888888888888888888888888888888888");
        assert_eq!(commits[1].files.len(), 1, "the real next commit is found after the separator");
        assert_eq!(commits[1].files[0].path, "next.rs");
    }

    #[test]
    fn a_path_containing_a_newline_survives_the_header_split() {
        // MUTATION THAT MUST BREAK IT: split the header's third field at the LAST
        // LF (`rposition`) instead of the first — the email then swallows the
        // counts and the path is truncated at its own newline.
        let mut log = Vec::new();
        log.extend_from_slice(b"\x010a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a0a\x00");
        log.extend_from_slice(b"2020-08-08T00:00:00+00:00\x00");
        log.extend_from_slice(b"dev@example.com\n4\t0\tweird\nname.rs\x00");

        let c = &parse_log(&log).expect("a newline inside a path parses")[0];
        assert_eq!(c.author_email.as_ref().map(GitBytes::as_bytes), Some(&b"dev@example.com"[..]));
        assert_eq!(c.files.len(), 1);
        assert_eq!(c.files[0].path.as_bytes(), b"weird\nname.rs", "the path keeps its newline");
        assert_eq!(c.files[0].lines_changed, Some(4));
    }

    #[test]
    fn an_empty_author_email_is_none_rather_than_an_empty_string() {
        // MUTATION THAT MUST BREAK IT: build `author_email` as
        // `Some(GitBytes::from(raw_email))` unconditionally — the column then
        // stores "" for an author line of `<>`, which no later query can tell
        // apart from a real address.
        let log = nul_join(&[
            b"\x017777777777777777777777777777777777777777",
            b"2020-09-09T00:00:00+00:00",
            b"\n1\t0\ta.rs",
        ]);

        let c = &parse_log(&log).expect("an empty author email parses")[0];
        assert_eq!(c.author_email, None, "absent identity, not an empty-string identity");
        assert_eq!(c.files.len(), 1, "the file list is still read");
    }

    #[test]
    fn authored_day_is_the_authors_own_day_not_the_utc_day() {
        // MUTATION THAT MUST BREAK IT: implement `authored_day` as
        // `self.authored_at.naive_utc().date()` — the commit below moves to the
        // previous day and lands in the wrong churn bucket.
        let log = nul_join(&[
            b"\x016666666666666666666666666666666666666666",
            b"2026-01-01T01:00:00+09:00",
            b"dev@example.com\n1\t0\ta.rs",
        ]);

        let c = &parse_log(&log).expect("an offset date parses")[0];
        assert_eq!(c.authored_at.offset().local_minus_utc(), 9 * 3600, "the offset is preserved");
        assert_eq!(
            c.authored_day(),
            day(2026, 1, 1),
            "01:00+09:00 is 2025-12-31T16:00Z, but the author's day is 2026-01-01",
        );
    }

    // ── failures stay failures ───────────────────────────────────────────────

    #[test]
    fn a_truncated_rename_is_an_error_not_half_a_record() {
        // MUTATION THAT MUST BREAK IT: replace either `ok_or_else(truncated)?`
        // in the rename branch with `unwrap_or_default()` — the touch is then
        // written with an empty path.
        let log = nul_join(&[
            b"\x015555555555555555555555555555555555555555",
            b"2020-10-10T00:00:00+00:00",
            b"dev@example.com\n1\t1\t",
            b"only/the/pre-image.rs",
        ]);

        assert_eq!(
            parse_log(&log),
            Err(GitLogError::TruncatedRename {
                sha: "5555555555555555555555555555555555555555".to_string()
            }),
            "a half-read rename is surfaced, never completed with a guess",
        );
    }

    #[test]
    fn a_malformed_count_is_an_error_not_unknown_churn() {
        // MUTATION THAT MUST BREAK IT: make `parse_count` return `Ok(None)` for
        // anything non-numeric instead of `Err(())` — a desynchronised field then
        // arrives in the database as a legitimate NULL, indistinguishable from a
        // binary file.
        let log = nul_join(&[
            b"\x014444444444444444444444444444444444444444",
            b"2020-11-11T00:00:00+00:00",
            b"dev@example.com\n1x\t0\ta.rs",
        ]);

        assert!(
            matches!(parse_log(&log), Err(GitLogError::MalformedEntry { .. })),
            "`1x` is not a count and not binary; it is a parse failure",
        );
    }

    #[test]
    fn a_field_where_a_header_belongs_is_a_desync_error() {
        // MUTATION THAT MUST BREAK IT: fall back to `head` when the sentinel is
        // absent (`strip_prefix(..).unwrap_or(head)`) — the parser then invents a
        // commit boundary wherever it has lost its place.
        let log = nul_join(&[b"not-a-header", b"2020-12-12T00:00:00+00:00", b"dev@example.com"]);

        assert_eq!(
            parse_log(&log),
            Err(GitLogError::Desync { field_index: 0, found: b"not-a-header"[..].into() }),
            "losing sync is reported with the position, never papered over",
        );
    }

    #[test]
    fn a_header_truncated_mid_way_is_an_error() {
        // MUTATION THAT MUST BREAK IT: default the missing date to
        // `DateTime::<Utc>::MIN_UTC` (or `Utc::now()`) instead of erroring — the
        // commit is then stored with a timestamp nobody measured.
        let log = nul_join(&[b"\x013333333333333333333333333333333333333333"]);

        assert_eq!(
            parse_log(&log),
            Err(GitLogError::TruncatedHeader {
                after_sha: Some("3333333333333333333333333333333333333333".to_string())
            }),
        );
    }

    #[test]
    fn a_malformed_sha_or_date_is_an_error() {
        // MUTATION THAT MUST BREAK IT: drop the `is_object_name` filter on `%H`,
        // or swap `parse_from_rfc3339(..).ok()` for a defaulted date.
        let short = nul_join(&[b"\x01abc", b"2020-01-01T00:00:00+00:00", b"dev@example.com"]);
        assert_eq!(parse_log(&short), Err(GitLogError::MalformedSha(b"abc"[..].into())));

        let bad_date = nul_join(&[
            b"\x012222222222222222222222222222222222222222",
            b"last Tuesday",
            b"dev@example.com",
        ]);
        assert!(matches!(parse_log(&bad_date), Err(GitLogError::MalformedAuthoredAt { .. })));
    }

    #[test]
    fn a_signature_prefixed_stream_fails_loudly_at_field_zero() {
        // MUTATION THAT MUST BREAK IT: skip to the first byte equal to
        // COMMIT_SENTINEL before reading the header, "tolerating" the preamble —
        // the scan would then appear to work while silently depending on a
        // byte-scan that a path beginning with SOH defeats.
        //
        // `--no-show-signature` is what keeps this out of the stream. This test
        // pins what happens if it is ever lost: a loud, positioned error — never
        // a partial history written as if it were complete. The preamble is the
        // literal line measured from git 2.50.1 with an SSH-signed commit.
        let mut log =
            b"Good \"git\" signature for a@example.com with ED25519 key SHA256:q9CAm\n".to_vec();
        log.extend_from_slice(&nul_join(&[
            b"\x011111111111111111111111111111111111111111",
            b"2020-01-01T00:00:00+00:00",
            b"dev@example.com\n1\t0\ta.rs",
        ]));

        match parse_log(&log) {
            Err(GitLogError::Desync { field_index, .. }) => {
                assert_eq!(field_index, 0, "the very first field is already wrong");
            }
            other => panic!("a signature preamble must be a Desync, got {other:?}"),
        }
    }

    // ── the invocation ───────────────────────────────────────────────────────

    #[test]
    fn log_args_walks_the_tip_set_and_never_head_or_all() {
        // MUTATION THAT MUST BREAK IT: replace the three pseudo-refs with
        // `--all` (sweeps refs/stash and refs/original into the facts) or drop
        // them entirely (falls back to HEAD, which is one checkout's opinion and
        // makes two checkouts of one repository disagree).
        let args = log_args::<String>(&[]).expect("no cursor is a full walk");

        assert_eq!(
            args,
            vec![
                "log",
                "--no-merges",
                "--no-show-signature",
                "-z",
                "-M",
                "--numstat",
                "--pretty=format:%x01%H%x00%aI%x00%ae",
                "--branches",
                "--remotes",
                "--tags",
            ],
        );
        assert!(!args.iter().any(|a| a == "--all"), "--all sweeps refs/stash and refs/original");
        assert!(!args.iter().any(|a| a == "HEAD"), "HEAD alone is one checkout's opinion");
        assert!(args.iter().any(|a| a.contains("%aI")), "author date, not committer date");
        assert!(!args.iter().any(|a| a.contains("%cI") || a.contains("%cd")), "no committer date");
    }

    #[test]
    fn log_args_pins_off_the_one_config_that_corrupts_the_stream() {
        // MUTATION THAT MUST BREAK IT: drop `--no-show-signature` from
        // BASE_LOG_ARGS — on any machine with `log.showSignature=true` and
        // signed commits, git prefixes a verification line to the stream and
        // every scan of that repository fails at field 0.
        //
        // Measured on git 2.50.1 with an SSH-signed commit: without the flag the
        // stream opens `Good "git" signature for a@example.com with ED25519 key
        // SHA256:…\n` BEFORE the sentinel; with it, the stream opens with the
        // sentinel. Notes and decoration were measured the same way and need no
        // flag — they reach a custom format only via `%N` and `%d`.
        assert!(
            log_args::<String>(&[]).expect("no cursor").iter().any(|a| a == "--no-show-signature"),
            "log.showSignature is the one user setting that can corrupt this stream",
        );
    }

    #[test]
    fn log_args_turns_the_stored_tips_into_exclusions() {
        // MUTATION THAT MUST BREAK IT: emit the cursor shas bare instead of
        // `^`-prefixed — the walk is then LIMITED to those commits rather than
        // stopping at them, and every incremental scan returns almost nothing.
        let tips = ["a".repeat(40), "b".repeat(64)];
        let args = log_args(&tips).expect("two valid object names");

        assert_eq!(
            &args[args.len() - 2..],
            &[format!("^{}", "a".repeat(40)), format!("^{}", "b".repeat(64))]
        );
    }

    #[test]
    fn log_args_rejects_a_cursor_that_is_not_an_object_name() {
        // MUTATION THAT MUST BREAK IT: drop the `is_object_name` check — a stored
        // tip of `--output=/etc/passwd` then reaches git's command line as a flag.
        for bad in ["--output=/tmp/x", "", "HEAD", "zzzz", &"a".repeat(41)] {
            assert_eq!(
                log_args(&[bad]),
                Err(GitLogError::InvalidCursorSha(bad.to_string())),
                "{bad:?} must be refused, not forwarded and not silently dropped",
            );
        }
    }

    #[test]
    fn tips_args_reads_the_same_three_namespaces_the_walk_covers() {
        // MUTATION THAT MUST BREAK IT: point `tips_args` at a different ref set
        // (e.g. only `refs/heads`) — the cursor then records fewer tips than the
        // walk covers, so tag-only commits are re-walked on every scan forever.
        assert_eq!(
            tips_args(),
            vec![
                "for-each-ref",
                "--format=%(objectname)",
                "refs/heads",
                "refs/remotes",
                "refs/tags",
            ],
        );
        assert_eq!(
            TIP_REF_GLOBS.len(),
            TIP_PSEUDO_REFS.len(),
            "the cursor and the walk name the same number of namespaces",
        );
    }

    #[test]
    fn the_pretty_format_and_the_sentinel_cannot_drift() {
        // MUTATION THAT MUST BREAK IT: change `COMMIT_SENTINEL` to 0x02 without
        // changing `%x01` in PRETTY_FORMAT (or the reverse) — every header then
        // fails the sentinel assertion at runtime, and nothing at compile time
        // would have said so.
        assert!(
            PRETTY_FORMAT.contains(&format!("%x{:02x}", COMMIT_SENTINEL)),
            "PRETTY_FORMAT must emit COMMIT_SENTINEL; it is {PRETTY_FORMAT}",
        );
        assert!(PRETTY_FORMAT.contains("%x00"), "the header's fields are NUL-separated");
    }

    // ── measured corpus ──────────────────────────────────────────────────────

    /// The exact 877 bytes `git log -z -M --numstat --no-merges` emits for
    /// sensei commit `9ed345d5` — the commit whose rename the non-`-z` reader
    /// stored as `crates/senseid/src/indexer/{barrier.rs => quality/reachability.rs}`.
    ///
    /// A fixture proves the parser handles what its author thought of; real
    /// output proves it handles what git actually writes.
    const MEASURED: &[&[u8]] = &[
        b"\x019ed345d596d38b5ce023898423ea8b1e1f3b4705\x00",
        b"2026-09-29T18:58:51-05:00\x00",
        b"hi@sensei-hq.com\n1\t1\tcrates/senseid/src/indexer/facts.rs\x00",
        b"3\t3\tcrates/senseid/src/indexer/impact.rs\x00",
        b"1\t1\tcrates/senseid/src/indexer/lang/c/walk.rs\x00",
        b"3\t3\tcrates/senseid/src/indexer/lang/java/mod.rs\x00",
        b"3\t3\tcrates/senseid/src/indexer/lang/javascript.rs\x00",
        b"1\t1\tcrates/senseid/src/indexer/lang/mod.rs\x00",
        b"12\t16\tcrates/senseid/src/indexer/mod.rs\x00",
        b"1\t1\tcrates/senseid/src/indexer/persist.rs\x00",
        b"50\t40\t\x00",
        b"crates/senseid/src/indexer/acceptance.rs\x00",
        b"crates/senseid/src/indexer/quality/acceptance.rs\x00",
        b"25\t0\tcrates/senseid/src/indexer/quality/mod.rs\x00",
        b"38\t24\t\x00",
        b"crates/senseid/src/indexer/barrier.rs\x00",
        b"crates/senseid/src/indexer/quality/reachability.rs\x00",
        b"1\t1\tcrates/senseid/src/indexer/resolve.rs\x00",
        b"1\t1\tcrates/senseid/src/languages/mod.rs\x00",
        b"3\t3\tdocs/backlog.md\x00",
        b"3\t2\tdocs/spec/indexer/18-acceptance.md\x00",
        b"4\t3\tdocs/spec/indexer/22-module-shape-and-the-second-pass.md\x00",
    ];

    #[test]
    fn real_git_output_from_this_repository_parses_whole() {
        // MUTATION THAT MUST BREAK IT: anything that changes the field arity of a
        // rename — the two renames below sit in the MIDDLE of sixteen entries, so
        // an off-by-one consumes a real entry and the count drops.
        let log: Vec<u8> = MEASURED.concat();
        assert_eq!(log.len(), 877, "the fixture is the measured byte count, unedited");

        let commits = parse_log(&log).expect("real git output parses");
        assert_eq!(commits.len(), 1, "a single-commit walk is one commit");
        let c = &commits[0];
        assert_eq!(c.sha, "9ed345d596d38b5ce023898423ea8b1e1f3b4705");
        assert_eq!(c.authored_day(), day(2026, 9, 29));
        assert_eq!(c.authored_at.offset().local_minus_utc(), -5 * 3600);
        assert_eq!(c.files.len(), 16, "sixteen numstat entries, two of them renames");

        // Not one stored path is a brace pseudo-path, and every one is a path
        // that could exist — this is the 8.51% defect, absent.
        for touch in &c.files {
            let p = touch.path.as_str().expect("this commit's paths are UTF-8");
            assert!(!p.contains("=>"), "{p} is a rename pseudo-path");
            assert!(!p.starts_with('"'), "{p} is C-quoted");
            assert!(touch.lines_changed.is_some(), "{p} has no binary entry in this commit");
        }

        let renames: Vec<_> = c.files.iter().filter(|t| t.renamed_from.is_some()).collect();
        assert_eq!(renames.len(), 2, "git detected two renames in this commit");
        assert_eq!(renames[1].path, "crates/senseid/src/indexer/quality/reachability.rs");
        assert_eq!(
            renames[1].renamed_from.as_ref().map(GitBytes::as_bytes),
            Some(&b"crates/senseid/src/indexer/barrier.rs"[..]),
        );
        assert_eq!(renames[1].lines_changed, Some(62), "38 + 24");

        // A conservation check over the whole commit. The expected value is
        // derived by a DIFFERENT tool rather than read back from this parser:
        // `git show --numstat -M --format= 9ed345d5 | awk -F'\t' '{a+=$1;
        // d+=$2} END {print a, d, a+d}'` reports 150 103 253.
        let total: i64 = c.files.iter().map(|t| t.lines_changed.unwrap_or(0)).sum();
        assert_eq!(total, 253, "sum of every added+deleted in the measured commit");
    }
}
