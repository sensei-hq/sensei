//! Shared helpers used by process and other handler modules.

use super::super::executor::TaskContext;
use crate::classifiers::ScanSkipReason;

/// Check if a file extension indicates a binary (non-text) file. This is a
/// fast first pass; `is_probably_binary` (content sniff) is the robust net for
/// anything not on this list.
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
/// recorded in `scan_state`. This is the ONLY read the change-detection does,
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
/// recorded in `scan_state`. Returns `None` if the file can't be read.
pub(crate) fn file_fingerprint(path: &std::path::Path) -> Option<(i64, String)> {
    let mtime = file_mtime_ms(path)?;
    let hash = hash_file(path)?;
    Some((mtime, hash))
}

/// Content sniff: read the head of a file and decide whether it is binary or
/// non-UTF8 (and therefore not parseable as source text). Catches binaries
/// whose extension isn't on the allowlist, plus latin-1/cp1252 text. A null
/// byte is a strong binary signal; otherwise we require the bytes to be valid
/// UTF-8 (tolerating a multi-byte char split at the read boundary).
pub(crate) fn is_probably_binary(path: &std::path::Path) -> bool {
    sniff_content(path).is_some()
}

/// The content-sniff half of [`classify_unscannable`], kept separate so
/// [`is_probably_binary`] and the reason-returning path share ONE implementation.
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
    if slice.contains(&0) {
        return Some(ScanSkipReason::BinaryContent);
    }
    match std::str::from_utf8(slice) {
        Ok(_) => None,
        // Only call it unscannable when invalid bytes appear well before the end
        // — a trailing error is likely a UTF-8 char split at the 8KB boundary.
        Err(e) if e.valid_up_to() + 4 < n => Some(ScanSkipReason::InvalidUtf8),
        Err(_) => None,
    }
}

/// Decide whether a file can be indexed as source text, and if not, why.
///
/// `None` means "index it". `Some(reason)` is recorded on the file's
/// `scan_state` row together with its fingerprint, so the skip sticks across
/// reconciles instead of the file looking changed forever. The extension test
/// comes first because it is a pure string compare — the content sniff only
/// reads the head of files the cheap test didn't already settle.
pub(crate) fn classify_unscannable(
    path: &std::path::Path,
    ext: &str,
) -> Option<ScanSkipReason> {
    if is_binary_ext(ext) {
        return Some(ScanSkipReason::UnsupportedFormat);
    }
    sniff_content(path)
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
/// quasi-repo (a non-git project root) still skips the vendored / build
/// directories its own `.gitignore` lists (node_modules, Pods, vendor/bundle, …)
/// instead of indexing hundreds of MB of dependencies.
pub(crate) fn build_walker(path: &std::path::Path) -> ignore::WalkBuilder {
    let mut b = ignore::WalkBuilder::new(path);
    b.hidden(true)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .require_git(false);
    b
}

/// The set of files in `dir` that the scan's ignore rules leave VISIBLE — i.e.
/// what [`build_walker`] would yield for that one directory.
///
/// This exists so the fs-watcher and the scan cannot disagree about what belongs
/// in the index. The watcher receives raw FSEvents, which know nothing about
/// `.gitignore`; without this it enqueued a `ProcessFile` for every generated
/// artifact (an i18n compiler's 131 emitted message files, say). The scan then
/// correctly did NOT see those files, so they landed in `plan.removed` and had
/// their nodes deleted and edges unresolved — and the next build re-added them.
/// A permanent add/prune churn loop over files that should never be indexed.
///
/// Implemented by reusing `build_walker` at `max_depth(1)` rather than
/// re-deriving the ignore rules: `parents(true)` (the default) still reads
/// `.gitignore` from every ancestor, so a nested `.gitignore` — the case that
/// actually bit us — is honoured exactly as the full walk honours it. Reusing the
/// builder means the two paths cannot drift apart.
///
/// Costs one directory read, so callers handling a batch should group by parent
/// and call this once per directory.
pub(crate) fn visible_files_in_dir(dir: &std::path::Path) -> std::collections::HashSet<std::path::PathBuf> {
    let mut w = build_walker(dir);
    w.max_depth(Some(1));
    w.build()
        .flatten()
        .filter(|e| e.path().is_file())
        .map(|e| e.path().to_path_buf())
        .collect()
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

    #[test]
    fn is_probably_binary_detects_content() {
        let dir = tempfile::tempdir().unwrap();
        let text = dir.path().join("a.txt");
        std::fs::write(&text, "fn main() { println!(\"hi\"); }\n").unwrap();
        assert!(!is_probably_binary(&text), "valid UTF-8 text is not binary");

        let nul = dir.path().join("b.bin");
        std::fs::write(&nul, [0x00u8, 0x01, 0x02, b'a', b'b']).unwrap();
        assert!(is_probably_binary(&nul), "null bytes => binary");

        let latin1 = dir.path().join("c.htm");
        // 0xE9 ('é' in latin-1) is invalid as a standalone UTF-8 byte.
        std::fs::write(&latin1, [b'<', b'p', b'>', 0xE9, 0xE9, 0xE9, b'<', b'/', b'p', b'>']).unwrap();
        assert!(is_probably_binary(&latin1), "non-UTF8 text => skip");

        let missing = dir.path().join("nope");
        assert!(is_probably_binary(&missing), "unreadable => skip");
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

    /// The watcher/scan parity check must honour a NESTED `.gitignore` — that is
    /// the exact shape that leaked: a generated directory carrying its own
    /// `.gitignore` containing `*`, holding 131 emitted i18n files. FSEvents
    /// reported them, the scan's walker did not, so they were indexed then pruned
    /// then re-indexed forever.
    #[test]
    fn visible_files_in_dir_honours_nested_gitignore() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        // Make it a repo-ish root with a top-level ignore file too.
        std::fs::write(root.join(".gitignore"), "*.log\n").unwrap();

        let gen_dir = root.join("generated");
        std::fs::create_dir_all(&gen_dir).unwrap();
        // The generated dir disowns everything inside it.
        std::fs::write(gen_dir.join(".gitignore"), "*\n").unwrap();
        std::fs::write(gen_dir.join("messages_a.js"), "export const a = 1\n").unwrap();
        std::fs::write(gen_dir.join("messages_b.js"), "export const b = 2\n").unwrap();

        let visible = visible_files_in_dir(&gen_dir);
        assert!(
            visible.is_empty(),
            "a nested .gitignore of `*` must hide every file in that directory, got {visible:?}"
        );

        // A sibling directory with tracked files stays visible, and the
        // root-level rule still applies within it.
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("main.rs"), "fn main() {}\n").unwrap();
        std::fs::write(src.join("debug.log"), "noise\n").unwrap();

        let visible = visible_files_in_dir(&src);
        assert!(visible.contains(&src.join("main.rs")), "tracked source stays visible");
        assert!(
            !visible.contains(&src.join("debug.log")),
            "an ancestor .gitignore rule must still apply to a nested directory"
        );
    }

    /// A directory with no ignore rules at all yields its files — guards against
    /// the helper accidentally hiding everything (which would silently stop the
    /// watcher from indexing anything).
    #[test]
    fn visible_files_in_dir_yields_unignored_files() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("thing.ts");
        std::fs::write(&f, "export const x = 1\n").unwrap();
        let visible = visible_files_in_dir(dir.path());
        assert!(visible.contains(&f), "an unignored file must be visible, got {visible:?}");
    }

    #[test]
    fn build_globset_matches_excluded_paths() {
        let gs = build_globset();
        assert!(gs.is_match("node_modules/foo/bar.js"));
        assert!(gs.is_match("src/foo.spec.ts"));
        assert!(gs.is_match("src/foo.test.tsx"));
        assert!(gs.is_match("tests/foo_test.py"));
        assert!(gs.is_match("pkg/foo_test.go"));
        assert!(gs.is_match("src/types.d.ts"));
        assert!(gs.is_match("dist/bundle.js"));
        assert!(gs.is_match("target/debug/foo"));
        assert!(gs.is_match("__pycache__/foo.pyc"));
        assert!(gs.is_match("__MACOSX/._foo"));
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
