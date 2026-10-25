//! Shared helpers used by process and other handler modules.

use super::super::executor::TaskContext;
use crate::classifiers::ScanSkipReason;

/// Check if a file extension indicates a binary (non-text) file. This is a
/// fast first pass; `sniff_content` is the robust net for anything not on this
/// list.
///
/// Thin wrapper over `classifiers::file_classifier()` — the extension list
/// itself lives in the classifier module so a single edit adds a new binary
/// type across every caller.
pub(crate) fn is_binary_ext(ext: &str) -> bool {
    crate::classifiers::file_classifier().is_binary(ext)
}

/// File modification time as Unix epoch milliseconds — a cheap stat (no read)
/// used to gate whether a file needs re-indexing on a re-scan.
pub(crate) fn file_mtime_ms(path: &std::path::Path) -> Option<i64> {
    std::fs::metadata(path)
        .and_then(|m| m.modified())
        .ok()?
        .duration_since(std::time::UNIX_EPOCH)
        .ok()
        .map(|d| d.as_millis() as i64)
}

/// SHA-256 hex of a file's bytes — the authoritative content-change signal
/// recorded in `sensei.files`. This is the ONLY read the change-detection does,
/// and it runs only for files the cheap mtime gate flagged as candidates (an
/// unchanged file is never hashed). Returns `None` if the file can't be read.
pub(crate) fn hash_file(path: &std::path::Path) -> Option<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).ok()?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Some(format!("{:x}", hasher.finalize()))
}

/// Incremental-index fingerprint: `(mtime_ms, sha256_hex)`. The mtime gates
/// re-indexing cheaply; the content hash is the authoritative change signal
/// recorded in `sensei.files`. Returns `None` if the file can't be read.
pub(crate) fn file_fingerprint(path: &std::path::Path) -> Option<(i64, String)> {
    let mtime = file_mtime_ms(path)?;
    let hash = hash_file(path)?;
    Some((mtime, hash))
}

/// The content-sniff half of [`classify_unscannable`]: read the head of a file
/// and decide whether it is binary or non-UTF8 (and therefore not parseable as
/// source text). Catches binaries whose extension isn't on the allowlist, plus
/// latin-1/cp1252 text.
///
/// `None` means the head of the file reads as UTF-8 source text. Otherwise it
/// distinguishes the two failure modes, because they differ in actionability: a
/// null byte means an opaque binary (ignore quietly), whereas merely-invalid
/// UTF-8 is usually latin-1/cp1252 text the user can re-encode.
///
/// An unreadable file returns `BinaryContent` so callers still treat it as
/// skippable — matching the previous behaviour.
fn sniff_content(path: &std::path::Path) -> Option<ScanSkipReason> {
    use std::io::Read;
    let mut buf = [0u8; 8192];
    let n = match std::fs::File::open(path).and_then(|mut f| f.read(&mut buf)) {
        Ok(n) => n,
        Err(_) => return Some(ScanSkipReason::BinaryContent), // unreadable → skippable
    };
    let slice = &buf[..n];
    // **A BOM IS A POSITIVE STATEMENT OF ENCODING, and it is read FIRST.**
    //
    // UTF-16LE ASCII is `X 00 X 00`, so the null test below fires on every
    // UTF-16 file. Once a BOM is present the nulls are explained and the file
    // is text — see `classifiers::decode_source`, which is the reader this
    // gate has to agree with.
    //
    // The head is NOT decoded here, deliberately. This buffer is 8KB of a
    // possibly larger file, so it may end mid-character or mid-surrogate, and a
    // decode of it would report an error that says nothing about the file. The
    // BOM alone answers the question this gate asks.
    if encoding_rs::Encoding::for_bom(slice).is_some() {
        return None;
    }
    if slice.contains(&0) {
        return Some(ScanSkipReason::BinaryContent);
    }
    match std::str::from_utf8(slice) {
        Ok(_) => generated_header(slice).then_some(ScanSkipReason::Generated),
        // Only call it unscannable when invalid bytes appear well before the end
        // — a trailing error is likely a UTF-8 char split at the 8KB boundary.
        Err(e) if e.valid_up_to() + 4 < n => Some(ScanSkipReason::InvalidUtf8),
        Err(e) => generated_header(&slice[..e.valid_up_to()]).then_some(ScanSkipReason::Generated),
    }
}

/// How many lines of a file's head may carry a generated-code marker. A marker
/// is a header: further down, the words are prose about generated code.
const GENERATED_HEADER_LINES: usize = 5;

/// Whether a file's head says a tool wrote it: the `@generated` tag (Rust,
/// Facebook's tooling, tree-sitter) or Go's `Code generated … DO NOT EDIT.`
fn generated_header(head: &[u8]) -> bool {
    let text = String::from_utf8_lossy(head);
    text.lines().take(GENERATED_HEADER_LINES).any(|line| {
        let tagged = line.match_indices("@generated").any(|(at, tag)| {
            !line[at + tag.len()..].starts_with(|c: char| c.is_alphanumeric() || c == '_')
        });
        tagged || (line.contains("Code generated") && line.contains("DO NOT EDIT"))
    })
}

/// The repository's `linguist-vendored` and `linguist-generated` marks, read
/// from its `.gitattributes` files: code the repository carries but does not
/// own, and code a tool wrote. Neither is indexed.
pub(crate) struct LinguistMarks {
    /// Each `.gitattributes`, shallowest first: a deeper file, and a later line
    /// within one, overrides an earlier answer — as git reads them.
    files: Vec<MarksIn>,
}

struct MarksIn {
    dir: std::path::PathBuf,
    vendored: ignore::gitignore::Gitignore,
    generated: ignore::gitignore::Gitignore,
}

impl LinguistMarks {
    /// No marks at all: every file is the repository's own.
    pub(crate) fn none() -> Self {
        Self { files: Vec::new() }
    }

    /// Read the `.gitattributes` of `repo_abs` and of each of `dirs` (relative
    /// to it). An unreadable file is an error that names it.
    pub(crate) fn load(
        repo_abs: &std::path::Path,
        dirs: impl IntoIterator<Item = std::path::PathBuf>,
    ) -> Result<Self, String> {
        let mut dirs: Vec<std::path::PathBuf> = dirs.into_iter().collect();
        dirs.push(std::path::PathBuf::new());
        dirs.sort_by_key(|d| (d.components().count(), d.clone()));
        dirs.dedup();
        let mut files = Vec::new();
        for rel in dirs {
            let dir = repo_abs.join(&rel);
            let path = dir.join(".gitattributes");
            let text = match std::fs::read_to_string(&path) {
                Ok(text) => text,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
            };
            files.push(MarksIn {
                vendored: marked(&dir, &text, "linguist-vendored")?,
                generated: marked(&dir, &text, "linguist-generated")?,
                dir,
            });
        }
        Ok(Self { files })
    }

    /// Why the file at `abs` is not the repository's own, if it is not.
    pub(crate) fn classify(&self, abs: &std::path::Path) -> Option<ScanSkipReason> {
        let set = |pick: fn(&MarksIn) -> &ignore::gitignore::Gitignore| {
            let mut answer = false;
            for file in self.files.iter().filter(|f| abs.starts_with(&f.dir)) {
                match pick(file).matched(abs, false) {
                    ignore::Match::Ignore(_) => answer = true,
                    ignore::Match::Whitelist(_) => answer = false,
                    ignore::Match::None => {}
                }
            }
            answer
        };
        if set(|f| &f.vendored) {
            Some(ScanSkipReason::Vendored)
        } else if set(|f| &f.generated) {
            Some(ScanSkipReason::Generated)
        } else {
            None
        }
    }
}

/// The patterns of one `.gitattributes` that set `attr` (as matches) or unset
/// it (as whitelist entries), in file order.
fn marked(
    dir: &std::path::Path,
    text: &str,
    attr: &str,
) -> Result<ignore::gitignore::Gitignore, String> {
    let mut builder = ignore::gitignore::GitignoreBuilder::new(dir);
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let mut tokens = line.split_whitespace();
        let Some(pattern) = tokens.next() else { continue };
        for token in tokens {
            let set = match token.split_once('=') {
                Some((name, value)) if name == attr => value != "false",
                None if token == attr => true,
                None if token.strip_prefix(['-', '!']) == Some(attr) => false,
                _ => continue,
            };
            let line = if set { pattern.to_string() } else { format!("!{pattern}") };
            builder.add_line(None, &line).map_err(|e| format!("{}: {e}", dir.display()))?;
        }
    }
    builder.build().map_err(|e| format!("{}: {e}", dir.display()))
}

/// Path patterns excluded from directory discovery.
///
/// Thin wrapper over the resolved [`crate::classifiers::ScanRules`] — the pattern
/// list itself lives with the other scan lists so it is operator-tunable through
/// `sensei.config` (`scan.exclude_globs.add` / `.remove`) rather than code-bound.
/// Returns a borrow of the process-wide set instead of rebuilding it per call.
pub(crate) fn build_globset() -> &'static globset::GlobSet {
    crate::classifiers::scan_rules().exclude_globs()
}

/// A directory walker that honours ignore files (.gitignore, .ignore, global
/// gitignore, .git/info/exclude). `require_git(false)` is the key bit: it makes
/// `.gitignore` apply even when the directory is NOT a git repository, so a
/// quasi-repo (a non-git workspace root) still skips the vendored / build
/// directories its own `.gitignore` lists (node_modules, Pods, vendor/bundle, …)
/// instead of indexing hundreds of MB of dependencies.
pub(crate) fn build_walker(path: &std::path::Path) -> ignore::WalkBuilder {
    let mut b = ignore::WalkBuilder::new(path);
    b.hidden(true).git_ignore(true).git_global(true).git_exclude(true).require_git(false);
    b
}

/// Flip a folder to `indexed` at the terminal community barrier (D4.1),
/// fail-closed (D6d). Promotes ONLY from `indexing` — the in-flight state
/// `process_git_folder` sets at scan start:
/// - `failed` (a ProcessFile recorded a fatal error) → left `failed` for
///   boot-reconcile / bounded-retry to re-drive (the D6d guard).
/// - already `indexed` (the daily analyzer re-detect of a settled folder) →
///   left as-is, so `indexed_at` isn't spuriously bumped.
/// - status read fails, so we can't certify the folder in-flight → left as-is.
///
/// This is the SOLE writer of `indexed`: the resolve/build barriers stamp libs
/// via `set_folder_props` but never advance the status, so `indexed` implies the
/// whole chain — including community detection — completed.
pub(crate) async fn mark_folder_indexed_fail_closed(
    ctx: &TaskContext,
    folder_id: &uuid::Uuid,
    folder_name: &str,
) {
    match ctx.pg().get_folder_status(folder_id).await {
        Ok(Some(status)) if status == "indexing" => {}
        Ok(Some(status)) => {
            tracing::debug!(folder = %folder_name, %status,
                "terminal barrier (D4.1): folder not `indexing` — leaving status unchanged");
            return;
        }
        Ok(None) => {
            tracing::warn!(folder = %folder_name,
                "terminal barrier (D4.1): folder row missing — not marking indexed");
            return;
        }
        Err(e) => {
            tracing::warn!(error = %e, folder = %folder_name,
                "fail-closed (D6d): get_folder_status failed — not marking indexed");
            return;
        }
    }
    if let Err(e) = ctx.pg().mark_folder_indexed(folder_id).await {
        tracing::warn!(error = %e, folder = %folder_name, "mark_folder_indexed failed");
    }
}

/// Decide whether a file can be indexed as source text, and if not, why.
///
/// `None` means "index it". `Some(reason)` is recorded on the file's
/// `sensei.files` row together with its fingerprint, so the skip sticks across
/// reconciles instead of the file looking changed forever. The extension test
/// comes first because it is a pure string compare — the content sniff only
/// reads the head of files the cheap test didn't already settle.
pub(crate) fn classify_unscannable(path: &std::path::Path, ext: &str) -> Option<ScanSkipReason> {
    if is_binary_ext(ext) {
        return Some(ScanSkipReason::UnsupportedFormat);
    }
    sniff_content(path).or_else(|| exceeds_parse_bounds(path).then_some(ScanSkipReason::Oversized))
}

/// The largest file the indexer parses, in bytes (#178).
const MAX_PARSE_BYTES: u64 = 2 * 1024 * 1024;

/// The longest line the indexer parses, in bytes (#178).
const MAX_LINE_BYTES: usize = 10_000;

/// Whether a file is past what the indexer will hand a parser: larger than
/// [`MAX_PARSE_BYTES`], or holding a line longer than [`MAX_LINE_BYTES`].
///
/// The bounds were measured, not guessed (2026-10-25, 61,345 tracked source
/// files under the watch roots): hand-written code reached 267 KB and a
/// 1,632-char line; the 137 files past either bound were bundles, data loads
/// and generated tables. A parse that overflows the stack is not a panic — it
/// aborts the daemon — so the bound is a rule about the input, not its name.
///
/// The size comes from the metadata, so a 100 MB data load is never read; a
/// file under the size bound is read whole, as its fingerprint already is. An
/// unreadable file is not called oversized: the content sniff has already
/// answered for it.
fn exceeds_parse_bounds(path: &std::path::Path) -> bool {
    match std::fs::metadata(path) {
        Ok(m) if m.len() > MAX_PARSE_BYTES => return true,
        Ok(_) => {}
        Err(_) => return false,
    }
    std::fs::read(path)
        .is_ok_and(|bytes| bytes.split(|&b| b == b'\n').any(|line| line.len() > MAX_LINE_BYTES))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_binary_ext_recognises_binaries() {
        assert!(is_binary_ext("png"));
        assert!(is_binary_ext("exe"));
        assert!(is_binary_ext("wasm"));
        assert!(is_binary_ext("sqlite3"));
        assert!(is_binary_ext("lock"));
        // extended set (from the ~/Developer scan's binary failures)
        assert!(is_binary_ext("avif"));
        assert!(is_binary_ext("webp"));
        assert!(is_binary_ext("pdf"));
        assert!(is_binary_ext("jar"));
        assert!(is_binary_ext("pyc"));
        assert!(is_binary_ext("parquet"));
        assert!(is_binary_ext("docx"));
        assert!(is_binary_ext("xlsx"));
        assert!(is_binary_ext("icns"));
    }

    /// **A UTF-16 SOURCE FILE IS NOT A BINARY**, and the scan gate is where it
    /// was being called one.
    ///
    /// UTF-16LE ASCII is `X 00 X 00`, so the null-byte test fired on every one.
    /// MEASURED before this landed: all 754 UTF-16 `.sql` files in the watched
    /// roots carried `skip_reason = binary_content`, and SSMS exports UTF-16LE
    /// by default — so an entire codebase of change scripts was invisible to
    /// the indexer before any parser was involved.
    ///
    /// MUTATION: drop the BOM check from `sniff_content` — every UTF-16 file
    /// goes back to being skipped as opaque.
    #[test]
    fn a_bom_carrying_utf16_file_is_text_rather_than_binary() {
        let dir = tempfile::tempdir().unwrap();
        let sql = dir.path().join("sp_X.sql");
        let mut bytes = vec![0xFFu8, 0xFE];
        for c in "CREATE PROCEDURE [dbo].[sp_X] AS SELECT 1".chars() {
            bytes.extend_from_slice(&(c as u16).to_le_bytes());
        }
        std::fs::write(&sql, &bytes).unwrap();
        assert!(!sniff_content_skips(&sql), "a BOM explains the nulls");
        assert_eq!(classify_unscannable(&sql, "sql"), None, "it is indexable");
    }

    fn sniff_content_skips(path: &std::path::Path) -> bool {
        sniff_content(path).is_some()
    }

    #[test]
    fn sniff_content_detects_content() {
        let dir = tempfile::tempdir().unwrap();
        let text = dir.path().join("a.txt");
        std::fs::write(&text, "fn main() { println!(\"hi\"); }\n").unwrap();
        assert!(!sniff_content_skips(&text), "valid UTF-8 text is not binary");

        let nul = dir.path().join("b.bin");
        std::fs::write(&nul, [0x00u8, 0x01, 0x02, b'a', b'b']).unwrap();
        assert!(sniff_content_skips(&nul), "null bytes => binary");

        let latin1 = dir.path().join("c.htm");
        // 0xE9 ('é' in latin-1) is invalid as a standalone UTF-8 byte.
        std::fs::write(&latin1, [b'<', b'p', b'>', 0xE9, 0xE9, 0xE9, b'<', b'/', b'p', b'>'])
            .unwrap();
        assert!(sniff_content_skips(&latin1), "non-UTF8 text => skip");

        let missing = dir.path().join("nope");
        assert!(sniff_content_skips(&missing), "unreadable => skip");
    }

    /// The reason matters, not just the yes/no: an unreadable-encoding file is
    /// something the USER can fix (re-encode it), whereas an opaque binary is
    /// expected and should stay quiet. The scan persists this reason so the file
    /// stops being re-enqueued every reconcile.
    #[test]
    fn classify_unscannable_distinguishes_the_reason() {
        let dir = tempfile::tempdir().unwrap();

        // Indexable source text → no reason, index it.
        let src = dir.path().join("a.rs");
        std::fs::write(&src, "fn main() {}\n").unwrap();
        assert_eq!(classify_unscannable(&src, "rs"), None, "valid source is indexable");

        // Known non-source extension → settled by the cheap string test.
        let cert = dir.path().join("Prod_Push_Certificate.p12");
        std::fs::write(&cert, [0x30u8, 0x82, 0x04, 0x00]).unwrap();
        assert_eq!(
            classify_unscannable(&cert, "p12"),
            Some(ScanSkipReason::UnsupportedFormat),
            "binary extension → unsupported_format"
        );

        // Upper-case extension must be caught too — these were the files that
        // slipped through and re-indexed forever.
        let img = dir.path().join("IMG_8877.JPG");
        std::fs::write(&img, [0xFFu8, 0xD8, 0xFF, 0xE0]).unwrap();
        assert_eq!(
            classify_unscannable(&img, "JPG"),
            Some(ScanSkipReason::UnsupportedFormat),
            "upper-case binary extension → unsupported_format"
        );

        // Null bytes with a source-ish extension → binary by content.
        let nul = dir.path().join("stats.json");
        std::fs::write(&nul, [b'{', 0x00, b'}']).unwrap();
        assert_eq!(
            classify_unscannable(&nul, "json"),
            Some(ScanSkipReason::BinaryContent),
            "null bytes → binary_content"
        );

        // Latin-1 text in a text file → actionable encoding problem. The invalid
        // bytes must sit well before the end: a trailing decode error is
        // deliberately tolerated as a multi-byte char split at the read boundary.
        let latin1 = dir.path().join("License.txt");
        let mut bytes = b"Copyright ".to_vec();
        bytes.extend_from_slice(&[0xE9, 0xE9, 0xE9]); // 'ééé' in latin-1
        bytes.extend_from_slice(b" - all rights reserved.\n");
        std::fs::write(&latin1, &bytes).unwrap();
        assert_eq!(
            classify_unscannable(&latin1, "txt"),
            Some(ScanSkipReason::InvalidUtf8),
            "non-UTF8 text → invalid_utf8 (the user can re-encode it)"
        );
    }

    /// **A FILE PAST THE PARSE BOUNDS IS REFUSED BY RULE, NOT BY NAME** (#178).
    ///
    /// A parse that overflows the stack aborts the daemon — no unwind, no file
    /// name. Exclude globs catch bundles by what they are CALLED; this catches
    /// them by what they ARE: past `MAX_PARSE_BYTES`, or holding a line past
    /// `MAX_LINE_BYTES` (measured 2026-10-25 over 61,345 tracked source files:
    /// hand-written code tops out at 267 KB and a 1,632-char line; the files
    /// past either bound are bundles, data loads and generated tables).
    ///
    /// MUTATION: drop either test from `exceeds_parse_bounds` — its file goes
    /// back to being parsed.
    #[test]
    fn a_file_past_the_parse_bounds_is_skipped_as_oversized() {
        let dir = tempfile::tempdir().unwrap();

        let big = dir.path().join("big.rs");
        let body = "fn a() {}\n".repeat(MAX_PARSE_BYTES as usize / 10 + 1);
        assert!(body.len() as u64 > MAX_PARSE_BYTES);
        std::fs::write(&big, &body).unwrap();
        assert_eq!(classify_unscannable(&big, "rs"), Some(ScanSkipReason::Oversized));

        let long = dir.path().join("bundle.js");
        std::fs::write(&long, format!("// head\nvar a={};\n", "1+".repeat(MAX_LINE_BYTES)))
            .unwrap();
        assert_eq!(classify_unscannable(&long, "js"), Some(ScanSkipReason::Oversized));

        // At the bounds, not past them: indexable.
        let edge = dir.path().join("edge.js");
        let line = "x".repeat(MAX_LINE_BYTES);
        std::fs::write(&edge, format!("{line}\n{line}")).unwrap();
        assert_eq!(classify_unscannable(&edge, "js"), None, "a line AT the bound is allowed");
        assert!(!ScanSkipReason::Oversized.is_actionable(), "nothing for the user to fix");
    }

    /// Only the reasons a user can act on should be surfaced; the rest are
    /// expected and would just be noise.
    #[test]
    fn only_encoding_and_parse_failures_are_actionable() {
        assert!(ScanSkipReason::InvalidUtf8.is_actionable());
        assert!(ScanSkipReason::ParseError.is_actionable());
        assert!(!ScanSkipReason::UnsupportedFormat.is_actionable());
        assert!(!ScanSkipReason::BinaryContent.is_actionable());
        assert!(!ScanSkipReason::ExcludedByConfig.is_actionable());
    }

    /// Every variant must map to a label the `sensei.scan_skip_reason` enum
    /// actually accepts — a typo here becomes a runtime insert failure.
    #[test]
    fn skip_reason_db_labels_match_the_pg_enum() {
        assert_eq!(ScanSkipReason::UnsupportedFormat.as_db(), "unsupported_format");
        assert_eq!(ScanSkipReason::BinaryContent.as_db(), "binary_content");
        assert_eq!(ScanSkipReason::InvalidUtf8.as_db(), "invalid_utf8");
        assert_eq!(ScanSkipReason::ParseError.as_db(), "parse_error");
        assert_eq!(ScanSkipReason::ExcludedByConfig.as_db(), "excluded_by_config");
        assert_eq!(ScanSkipReason::Vendored.as_db(), "vendored");
        assert_eq!(ScanSkipReason::Generated.as_db(), "generated");
        assert_eq!(ScanSkipReason::Oversized.as_db(), "oversized");
        assert!(!ScanSkipReason::Vendored.is_actionable());
        assert!(!ScanSkipReason::Generated.is_actionable());
    }

    /// A header says a tool wrote the file; prose ABOUT generated code, or a
    /// marker below the header, does not.
    #[test]
    fn a_generated_header_is_read_only_from_the_head_and_only_as_a_tag() {
        assert!(generated_header(b"// @generated by tree-sitter\nint x;\n"));
        assert!(generated_header(b"#!/bin/sh\n# @generated\n"));
        assert!(generated_header(b"// Code generated by protoc-gen-go. DO NOT EDIT.\npackage x\n"));
        assert!(!generated_header(b"//   (a _worker.js + auto-generated .assetsignore) which\n"));
        assert!(!generated_header(b"const tag = '@generated_by';\n"));
        assert!(!generated_header(b"1\n2\n3\n4\n5\n// @generated\n"));
    }

    /// A nested `.gitattributes` speaks for its own directory and overrides
    /// the root's; a directory pattern does not reach into the directory (git
    /// reads attributes per path, not by parents).
    #[test]
    fn nested_attributes_override_the_root_and_patterns_match_paths() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join("third/keep")).unwrap();
        std::fs::write(
            root.join(".gitattributes"),
            "third/** linguist-vendored\nwhole linguist-vendored\n",
        )
        .unwrap();
        std::fs::write(root.join("third/keep/.gitattributes"), "*.c -linguist-vendored\n").unwrap();
        let marks = LinguistMarks::load(root, [std::path::PathBuf::from("third/keep")]).unwrap();
        assert_eq!(marks.classify(&root.join("third/a.c")), Some(ScanSkipReason::Vendored));
        assert_eq!(marks.classify(&root.join("third/keep/b.c")), None, "the nested file unsets it");
        assert_eq!(marks.classify(&root.join("third/keep/b.h")), Some(ScanSkipReason::Vendored));
        assert_eq!(
            marks.classify(&root.join("whole/x.rs")),
            None,
            "a directory pattern is not `/**`"
        );
        assert_eq!(marks.classify(&root.join("src/x.rs")), None);
    }

    #[test]
    fn is_binary_ext_rejects_source_extensions() {
        assert!(!is_binary_ext("rs"));
        assert!(!is_binary_ext("ts"));
        assert!(!is_binary_ext("py"));
        assert!(!is_binary_ext("md"));
        assert!(!is_binary_ext("json"));
        assert!(!is_binary_ext(""));
    }

    /// DEPENDENCIES and GENERATED OUTPUT are excluded; TESTS are not.
    ///
    /// This test previously asserted the opposite for tests — `src/foo.spec.ts`,
    /// `src/foo.test.tsx`, `tests/foo_test.py`, `pkg/foo_test.go` were all
    /// required to match. That pinned the old list rather than the intent: the
    /// graph describes code, TESTS and docs, and a test is often the only place a
    /// public API's real usage is written down. Those globs were also INERT on
    /// the indexing path, so the files were indexed anyway while
    /// `count_indexable_files` reported them excluded.
    #[test]
    fn build_globset_excludes_dependencies_and_generated_output() {
        let gs = build_globset();
        assert!(gs.is_match("node_modules/foo/bar.js"));
        assert!(gs.is_match("dist/bundle.js"));
        assert!(gs.is_match("target/debug/foo"));
        assert!(gs.is_match("__pycache__/foo.pyc"));
        assert!(gs.is_match("__MACOSX/._foo"));
        assert!(gs.is_match("src/types.d.ts"), "generated declarations");

        // Build output that lives OUTSIDE dist/build — the case that put 3,555
        // minified nodes in the graph.
        assert!(gs.is_match("web/static/app.min.js"));
        assert!(gs.is_match("web/static/app.min.css"));
        assert!(gs.is_match("web/static/app.bundle.js"));
        assert!(gs.is_match("web/static/app.js.map"));
        assert!(
            gs.is_match("documentation/artifacts/erd/assets/index-Cgv8QKbu.js"),
            "a Vite content-hashed asset"
        );
    }

    /// A TEST is part of the structure being described, so it must NOT match.
    #[test]
    fn build_globset_keeps_tests_and_normal_source() {
        let gs = build_globset();
        for kept in [
            "src/foo.spec.ts",
            "src/foo.test.tsx",
            "tests/foo_test.py",
            "pkg/foo_test.go",
            "crates/x/src/lib_test.rs",
            "app/e2e/flow.spec.ts",
        ] {
            assert!(!gs.is_match(kept), "{kept} is part of the codebase");
        }
        // A hand-written asset with no content hash is untouched.
        assert!(!gs.is_match("src/assets/index.js"), "no hash suffix — not generated");
    }

    #[test]
    fn build_globset_allows_normal_source_files() {
        let gs = build_globset();
        assert!(!gs.is_match("src/main.rs"));
        assert!(!gs.is_match("lib/utils.ts"));
        assert!(!gs.is_match("app.py"));
        assert!(!gs.is_match("docs/readme.md"));
    }
}
