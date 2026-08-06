//! Shared helpers used by process and other handler modules.

use super::super::executor::TaskContext;

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
    use std::io::Read;
    let mut buf = [0u8; 8192];
    let n = match std::fs::File::open(path).and_then(|mut f| f.read(&mut buf)) {
        Ok(n) => n,
        Err(_) => return true, // unreadable → treat as skippable
    };
    let slice = &buf[..n];
    if slice.contains(&0) {
        return true;
    }
    match std::str::from_utf8(slice) {
        Ok(_) => false,
        // Only call it binary when invalid bytes appear well before the end —
        // a trailing error is likely a UTF-8 char split at the 8KB boundary.
        Err(e) => e.valid_up_to() + 4 < n,
    }
}

pub(crate) fn build_globset() -> globset::GlobSet {
    let patterns = &[
        "**/node_modules/**", "**/dist/**", "**/build/**", "**/target/**",
        "**/.next/**", "**/.svelte-kit/**",
        "**/__pycache__/**", "**/__MACOSX/**", "**/.venv/**", "**/venv/**",
        "**/*.spec.ts", "**/*.spec.tsx", "**/*.spec.js",
        "**/*.test.ts", "**/*.test.tsx", "**/*.test.js",
        "**/*_test.py", "**/*_test.go", "**/*_test.rs",
        "**/*.d.ts",
    ];
    let mut builder = globset::GlobSetBuilder::new();
    for p in patterns {
        if let Ok(g) = globset::Glob::new(p) { builder.add(g); }
    }
    builder.build().unwrap_or_else(|_| globset::GlobSetBuilder::new().build().unwrap())
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

    #[test]
    fn is_binary_ext_rejects_source_extensions() {
        assert!(!is_binary_ext("rs"));
        assert!(!is_binary_ext("ts"));
        assert!(!is_binary_ext("py"));
        assert!(!is_binary_ext("md"));
        assert!(!is_binary_ext("json"));
        assert!(!is_binary_ext(""));
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
