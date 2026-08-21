//! File classification traits.
//!
//! Consolidates the two extension lists that used to live inline in
//! `tasks/handlers/helpers.rs` (binary) and `tasks/handlers/scan_logic.rs`
//! (source). A single default classifier now owns both lists and the
//! `LanguageAdapter` handoff for source detection.
//!
//! Callers should route through `file_classifier()` — an accessor returning a
//! `&'static dyn FileClassifier`. Tests may construct alternate impls to
//! exercise callers with different classification semantics without touching
//! the global.

/// Classify a file extension as binary or source.
///
/// `ext` is the extension WITHOUT a leading dot, case-sensitive (callers
/// lowercase before calling for portable comparisons).
pub trait FileClassifier: Send + Sync {
    /// True if `ext` names a binary / opaque format the scanner must skip.
    /// A `false` return means either "text-like" or "unrecognised" — callers
    /// combine this with a content sniff for the unrecognised case.
    fn is_binary(&self, ext: &str) -> bool;

    /// True if `ext` names a first-party source language or docs format that
    /// signals a project. Data / config / binaries (`.csv`, `.json`, `.png`)
    /// deliberately return `false` — a folder of only those is not a project.
    fn is_source_file(&self, ext: &str) -> bool;
}

/// Return the process-wide default classifier.
///
/// Static lifetime lets callers cache the reference without lifetime plumbing.
pub fn file_classifier() -> &'static dyn FileClassifier {
    &DEFAULT_CLASSIFIER
}

static DEFAULT_CLASSIFIER: DefaultClassifier = DefaultClassifier;

/// Why the indexer examined a file but deliberately did not index it. Mirrors
/// the `sensei.scan_skip_reason` enum; persisted on the file's `scan_state` row
/// so the skip is recorded WITH its fingerprint. That pairing is what stops a
/// skipped file being re-enqueued on every reconcile, and it makes the skip
/// self-healing: fixing the file changes its fingerprint, so it is re-attempted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanSkipReason {
    /// A known non-source format (by extension) — expected, ignore quietly.
    UnsupportedFormat,
    /// Null bytes in the head of the file — opaque binary, ignore quietly.
    BinaryContent,
    /// Not valid UTF-8. ACTIONABLE: the user can re-encode the file.
    InvalidUtf8,
    /// The parser rejected or panicked on the file. Actionable / reportable.
    ParseError,
    /// Matched a user-configured exclude rule.
    ExcludedByConfig,
}

impl ScanSkipReason {
    /// The `sensei.scan_skip_reason` enum label for this variant.
    pub fn as_db(self) -> &'static str {
        match self {
            Self::UnsupportedFormat => "unsupported_format",
            Self::BinaryContent     => "binary_content",
            Self::InvalidUtf8       => "invalid_utf8",
            Self::ParseError        => "parse_error",
            Self::ExcludedByConfig  => "excluded_by_config",
        }
    }

    /// True when the user can plausibly act on this (fix an encoding, report a
    /// parser bug). The quiet reasons are expected and not worth surfacing.
    pub fn is_actionable(self) -> bool {
        matches!(self, Self::InvalidUtf8 | Self::ParseError)
    }
}

/// The default classifier used across the scanner and processor pipelines.
///
/// - Binary set: images, fonts, archives, compiled binaries, columnar data,
///   office documents, media, keys/certificates, ebooks, design documents,
///   model archives, platform packages, and a few misc opaque formats.
/// - Source set: any `LanguageAdapter`-backed extension, plus Markdown docs
///   and a fallback list of common source languages we recognise without a
///   parser adapter (Go / Ruby / Shell / PHP / Lua / Scala / etc.). The
///   fallback list stays flat here instead of scattering across handlers.
pub struct DefaultClassifier;

const BINARY_EXTS: &[&str] = &[
    // images
    "png", "jpg", "jpeg", "gif", "ico", "svg", "webp", "avif", "bmp",
    "tiff", "tif", "icns", "heic",
    // fonts
    "woff", "woff2", "ttf", "eot", "otf",
    // archives
    "zip", "tar", "gz", "tgz", "bz2", "xz", "7z", "rar", "z",
    // compiled / binaries
    "exe", "dll", "so", "dylib", "o", "a", "lib", "class", "jar",
    "pyc", "pyo", "pdb", "wasm",
    // databases / columnar data
    "db", "sqlite", "sqlite3", "profraw", "parquet", "arrow", "feather",
    // office documents
    "pdf", "doc", "docx", "xls", "xlsx", "ppt", "pptx",
    // media
    "mp4", "mov", "avi", "webm", "mkv", "mp3", "wav", "flac", "ogg",
    // keys / certificates / keystores (opaque; never source)
    "p12", "pfx", "jks", "keystore", "cer", "crt", "der",
    // ebooks
    "epub", "mobi", "azw3",
    // design documents
    "psd", "ai", "sketch", "xcf",
    // model / array serialisation
    "npz", "npy", "pkl", "h5", "onnx", "safetensors", "pt", "pth",
    // platform packages / disk images
    "aar", "apk", "aab", "ipa", "dmg", "deb", "rpm", "iso",
    // binary lockfiles + db dumps
    "lockb", "dump",
    // editor swap files
    "swp", "swo", "swn",
    // misc binary
    "bin", "dat", "pack", "idx", "map", "ds_store", "lock",
];

/// Extra source-language extensions we recognise without a parser adapter.
///
/// Keeping this flat here means new-language coverage in the classifier is
/// one edit, not a scattered scan of scan_logic.rs.
const FALLBACK_SOURCE_EXTS: &[&str] = &[
    "go", "rb", "sh", "bash", "zsh", "fish", "pl", "pm", "php",
    "lua", "r", "jl", "scala", "ex", "exs", "erl", "hs", "ml",
    "dart", "cs", "fs", "fsx", "clj", "cljs", "groovy", "m", "mm",
    "cxx", "hh", "hxx", "swift", "scss", "css", "html",
];

impl FileClassifier for DefaultClassifier {
    fn is_binary(&self, ext: &str) -> bool {
        // Normalise like `is_source_file` does. A raw case-sensitive match let
        // real binaries through on any filesystem that preserves upper-case
        // extensions (`IMG_1234.JPG`, `chart.PNG`), and each one that slipped
        // past here was then skipped later without being fingerprinted — so it
        // looked "changed" on every single reconcile pass. Keep BINARY_EXTS
        // entries lower-case so this lookup can hit them.
        let e = ext.trim_start_matches('.').to_ascii_lowercase();
        BINARY_EXTS.contains(&e.as_str())
    }

    fn is_source_file(&self, ext: &str) -> bool {
        let e = ext.trim_start_matches('.').to_ascii_lowercase();
        // Markdown docs: a docs-only folder is a documentation project.
        if matches!(e.as_str(), "md" | "mdx") {
            return true;
        }
        // Delegate to LanguageAdapter for anything the parser understands.
        if crate::languages::adapter_for_ext(&format!(".{e}")).is_some() {
            return true;
        }
        // Common source languages we recognise without a parser adapter.
        FALLBACK_SOURCE_EXTS.contains(&e.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_binary_recognises_binaries() {
        let c = file_classifier();
        assert!(c.is_binary("png"));
        assert!(c.is_binary("exe"));
        assert!(c.is_binary("wasm"));
        assert!(c.is_binary("sqlite3"));
        assert!(c.is_binary("lock"));
        assert!(c.is_binary("parquet"));
        assert!(c.is_binary("docx"));
        assert!(c.is_binary("icns"));
    }

    /// Upper-case extensions are real on case-preserving filesystems
    /// (`IMG_8877.JPG`, `Body Measurements.PNG`). A case-sensitive lookup let
    /// them through the walk filter, and they were then skipped downstream
    /// without a fingerprint — so they re-indexed on every reconcile forever.
    #[test]
    fn is_binary_is_case_insensitive() {
        let c = file_classifier();
        for ext in ["JPG", "PNG", "Png", "PDF", "ZIP", "Jpeg"] {
            assert!(c.is_binary(ext), "{ext} must be recognised as binary regardless of case");
        }
    }

    /// A leading dot must not defeat the lookup either.
    #[test]
    fn is_binary_tolerates_leading_dot() {
        let c = file_classifier();
        assert!(file_classifier().is_binary(".png"));
        assert!(c.is_binary(".JPG"));
    }

    /// Opaque formats found re-indexing every 5 minutes in the wild: keystores,
    /// ebooks, design docs, model archives, platform packages, binary
    /// lockfiles, db dumps and editor swap files.
    #[test]
    fn is_binary_covers_observed_reindex_loop_offenders() {
        let c = file_classifier();
        for ext in [
            "p12", "pfx", "jks", "keystore",
            "epub", "mobi",
            "psd",
            "npz", "npy", "safetensors",
            "aar", "apk", "dmg",
            "lockb", "dump",
            "swp",
        ] {
            assert!(c.is_binary(ext), "{ext} must be classified binary");
        }
    }

    #[test]
    fn is_binary_rejects_source_extensions() {
        let c = file_classifier();
        assert!(!c.is_binary("rs"));
        assert!(!c.is_binary("ts"));
        assert!(!c.is_binary("py"));
        assert!(!c.is_binary("md"));
        assert!(!c.is_binary("json"));
        assert!(!c.is_binary(""));
    }

    #[test]
    fn is_source_file_via_language_adapter() {
        let c = file_classifier();
        assert!(c.is_source_file("rs"));
        assert!(c.is_source_file("ts"));
        assert!(c.is_source_file("py"));
        assert!(c.is_source_file("svelte"));
        assert!(c.is_source_file("kt"));
    }

    #[test]
    fn is_source_file_recognises_markdown() {
        let c = file_classifier();
        assert!(c.is_source_file("md"));
        assert!(c.is_source_file("mdx"));
    }

    #[test]
    fn is_source_file_via_fallback_list() {
        let c = file_classifier();
        assert!(c.is_source_file("go"));
        assert!(c.is_source_file("rb"));
        assert!(c.is_source_file("sh"));
        assert!(c.is_source_file("php"));
        assert!(c.is_source_file("lua"));
        assert!(c.is_source_file("css"));
        assert!(c.is_source_file("html"));
    }

    #[test]
    fn is_source_file_rejects_data_and_binaries() {
        let c = file_classifier();
        assert!(!c.is_source_file("json"));
        assert!(!c.is_source_file("csv"));
        assert!(!c.is_source_file("txt"));
        assert!(!c.is_source_file("png"));
        assert!(!c.is_source_file("pdf"));
    }

    #[test]
    fn is_source_file_normalises_case_and_dot_prefix() {
        let c = file_classifier();
        // Callers should already have lowercased + stripped the dot, but
        // the default classifier is forgiving so a stray leading dot or
        // upper-case ext still resolves.
        assert!(c.is_source_file(".rs"));
        assert!(c.is_source_file("RS"));
        assert!(c.is_source_file(".MD"));
    }
}
