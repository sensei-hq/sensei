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

// ── Operator-tunable scan rules ───────────────────────────────────────────────

/// The effective extension / glob lists the scanner runs with.
///
/// The built-in constants below are DEFAULTS, not the whole story: an operator
/// can add to or subtract from each list through `sensei.config` without a code
/// change or a release. Effective set = `(default ∪ add) \ remove`.
///
/// Add-and-remove rather than a wholesale override on purpose. A seeded full
/// list would freeze at whatever shipped the day it was written, so a later
/// release that recognises a new opaque format would never reach an operator who
/// had customised theirs. This way defaults keep improving underneath and local
/// choices still win.
pub struct ScanRules {
    binary_exts: std::collections::HashSet<String>,
    source_exts: std::collections::HashSet<String>,
    exclude: globset::GlobSet,
}

/// Per-list `add`/`remove` overrides, already parsed out of config.
#[derive(Debug, Default, Clone)]
pub struct ScanRuleOverrides {
    pub binary_add: Vec<String>,
    pub binary_remove: Vec<String>,
    pub source_add: Vec<String>,
    pub source_remove: Vec<String>,
    pub exclude_add: Vec<String>,
    pub exclude_remove: Vec<String>,
}

/// The `sensei.config` keys read at boot. Values are JSON arrays of strings.
pub const SCAN_RULE_CONFIG_KEYS: &[&str] = &[
    "scan.binary_exts.add",
    "scan.binary_exts.remove",
    "scan.source_exts.add",
    "scan.source_exts.remove",
    "scan.exclude_globs.add",
    "scan.exclude_globs.remove",
];

static SCAN_RULES: std::sync::OnceLock<ScanRules> = std::sync::OnceLock::new();

/// The process-wide scan rules.
///
/// Falls back to the built-in defaults when [`init_scan_rules`] was never called,
/// so unit tests and any non-daemon caller behave exactly as before rather than
/// panicking or seeing empty lists.
pub fn scan_rules() -> &'static ScanRules {
    SCAN_RULES.get_or_init(|| ScanRules::from_overrides(&ScanRuleOverrides::default()))
}

/// Install operator overrides. Call ONCE, early in daemon boot, before any
/// scanning starts. Returns `false` if the rules were already resolved (a later
/// call cannot silently swap the lists out from under an in-flight scan).
pub fn init_scan_rules(overrides: &ScanRuleOverrides) -> bool {
    SCAN_RULES.set(ScanRules::from_overrides(overrides)).is_ok()
}

fn normalise_ext(s: &str) -> String {
    s.trim().trim_start_matches('.').to_ascii_lowercase()
}

/// Parse a `sensei.config` value holding a JSON array of strings.
///
/// A malformed value logs and yields nothing rather than propagating: a typo in
/// one operator-edited key must never stop the daemon from scanning. Returning
/// empty means "no override", which degrades to the built-in defaults.
pub fn parse_string_array(key: &str, raw: Option<String>) -> Vec<String> {
    let Some(raw) = raw else { return Vec::new() };
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    match serde_json::from_str::<Vec<String>>(trimmed) {
        Ok(v) => v,
        Err(e) => {
            tracing::warn!(config_key = %key, error = %e,
                "scan rules: expected a JSON array of strings — ignoring this override");
            Vec::new()
        }
    }
}

impl ScanRuleOverrides {
    /// Build the overrides from raw config values. `get` is injected so this is
    /// pure and unit-testable without a database.
    pub fn from_config(get: impl Fn(&str) -> Option<String>) -> Self {
        let read = |key: &str| parse_string_array(key, get(key));
        Self {
            binary_add: read("scan.binary_exts.add"),
            binary_remove: read("scan.binary_exts.remove"),
            source_add: read("scan.source_exts.add"),
            source_remove: read("scan.source_exts.remove"),
            exclude_add: read("scan.exclude_globs.add"),
            exclude_remove: read("scan.exclude_globs.remove"),
        }
    }

    /// True when no key supplied anything — lets boot log "defaults" rather than
    /// implying an operator customised the lists.
    pub fn is_empty(&self) -> bool {
        self.binary_add.is_empty()
            && self.binary_remove.is_empty()
            && self.source_add.is_empty()
            && self.source_remove.is_empty()
            && self.exclude_add.is_empty()
            && self.exclude_remove.is_empty()
    }
}

impl ScanRules {
    fn from_overrides(o: &ScanRuleOverrides) -> Self {
        let build_ext_set = |defaults: &[&str], add: &[String], remove: &[String]| {
            let removed: std::collections::HashSet<String> =
                remove.iter().map(|s| normalise_ext(s)).collect();
            defaults
                .iter()
                .map(|s| normalise_ext(s))
                .chain(add.iter().map(|s| normalise_ext(s)))
                .filter(|e| !e.is_empty() && !removed.contains(e))
                .collect()
        };

        let removed_globs: std::collections::HashSet<&str> =
            o.exclude_remove.iter().map(|s| s.trim()).collect();
        let mut b = globset::GlobSetBuilder::new();
        for p in DEFAULT_EXCLUDE_GLOBS.iter().copied().chain(o.exclude_add.iter().map(|s| s.trim()))
        {
            if p.is_empty() || removed_globs.contains(p) {
                continue;
            }
            match globset::Glob::new(p) {
                Ok(g) => {
                    b.add(g);
                }
                // A typo in one operator-supplied pattern must not take the whole
                // exclude list (and therefore the scan) down with it.
                Err(e) => tracing::warn!(pattern = %p, error = %e,
                    "scan.exclude_globs: ignoring unparseable glob"),
            }
        }

        Self {
            binary_exts: build_ext_set(DEFAULT_BINARY_EXTS, &o.binary_add, &o.binary_remove),
            source_exts: build_ext_set(DEFAULT_SOURCE_EXTS, &o.source_add, &o.source_remove),
            exclude: b.build().unwrap_or_else(|e| {
                tracing::warn!(error = %e, "scan.exclude_globs: falling back to an empty set");
                globset::GlobSetBuilder::new().build().expect("empty globset always builds")
            }),
        }
    }

    /// `ext` is matched case-insensitively and with any leading dot stripped.
    pub fn is_binary_ext(&self, ext: &str) -> bool {
        self.binary_exts.contains(&normalise_ext(ext))
    }

    /// The fallback source list — languages recognised without a parser adapter.
    pub fn is_fallback_source_ext(&self, ext: &str) -> bool {
        self.source_exts.contains(&normalise_ext(ext))
    }

    /// Path patterns excluded from DIRECTORY discovery. Note this does not gate
    /// file indexing: test files match `**/*.spec.ts` here yet are deliberately
    /// indexed and flagged via `nodes.is_test`.
    pub fn exclude_globs(&self) -> &globset::GlobSet {
        &self.exclude
    }
}

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
            Self::BinaryContent => "binary_content",
            Self::InvalidUtf8 => "invalid_utf8",
            Self::ParseError => "parse_error",
            Self::ExcludedByConfig => "excluded_by_config",
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

/// Built-in binary extensions. Operators extend or trim this via
/// `scan.binary_exts.add` / `scan.binary_exts.remove` — see [`ScanRules`].
const DEFAULT_BINARY_EXTS: &[&str] = &[
    // images
    "png",
    "jpg",
    "jpeg",
    "gif",
    "ico",
    "svg",
    "webp",
    "avif",
    "bmp",
    "tiff",
    "tif",
    "icns",
    "heic",
    // fonts
    "woff",
    "woff2",
    "ttf",
    "eot",
    "otf",
    // archives
    "zip",
    "tar",
    "gz",
    "tgz",
    "bz2",
    "xz",
    "7z",
    "rar",
    "z",
    // compiled / binaries
    "exe",
    "dll",
    "so",
    "dylib",
    "o",
    "a",
    "lib",
    "class",
    "jar",
    "pyc",
    "pyo",
    "pdb",
    "wasm",
    // databases / columnar data
    "db",
    "sqlite",
    "sqlite3",
    "profraw",
    "parquet",
    "arrow",
    "feather",
    // office documents
    "pdf",
    "doc",
    "docx",
    "xls",
    "xlsx",
    "ppt",
    "pptx",
    // media
    "mp4",
    "mov",
    "avi",
    "webm",
    "mkv",
    "mp3",
    "wav",
    "flac",
    "ogg",
    // keys / certificates / keystores (opaque; never source)
    "p12",
    "pfx",
    "jks",
    "keystore",
    "cer",
    "crt",
    "der",
    // ebooks
    "epub",
    "mobi",
    "azw3",
    // design documents
    "psd",
    "ai",
    "sketch",
    "xcf",
    // model / array serialisation
    "npz",
    "npy",
    "pkl",
    "h5",
    "onnx",
    "safetensors",
    "pt",
    "pth",
    // platform packages / disk images
    "aar",
    "apk",
    "aab",
    "ipa",
    "dmg",
    "deb",
    "rpm",
    "iso",
    // binary lockfiles + db dumps
    "lockb",
    "dump",
    // editor swap files
    "swp",
    "swo",
    "swn",
    // misc binary
    "bin",
    "dat",
    "pack",
    "idx",
    "map",
    "ds_store",
    "lock",
];

/// Extra source-language extensions we recognise without a parser adapter.
///
/// Keeping this flat here means new-language coverage in the classifier is
/// one edit, not a scattered scan of scan_logic.rs.
const DEFAULT_SOURCE_EXTS: &[&str] = &[
    "go", "rb", "sh", "bash", "zsh", "fish", "pl", "pm", "php", "lua", "r", "jl", "scala", "ex",
    "exs", "erl", "hs", "ml", "dart", "cs", "fs", "fsx", "clj", "cljs", "groovy", "m", "mm", "cxx",
    "hh", "hxx", "swift", "scss", "css", "html",
];

/// Built-in path patterns excluded from DIRECTORY discovery. Operators extend or
/// trim this via `scan.exclude_globs.add` / `scan.exclude_globs.remove`.
///
/// NOTE this gates folder discovery, NOT file indexing. The `**/*.spec.ts`-style
/// patterns read as "don't treat a directory of only specs as a project"; test
/// files themselves are deliberately indexed and flagged via `nodes.is_test`.
const DEFAULT_EXCLUDE_GLOBS: &[&str] = &[
    "**/node_modules/**",
    "**/dist/**",
    "**/build/**",
    "**/target/**",
    "**/.next/**",
    "**/.svelte-kit/**",
    "**/__pycache__/**",
    "**/__MACOSX/**",
    "**/.venv/**",
    "**/venv/**",
    "**/*.spec.ts",
    "**/*.spec.tsx",
    "**/*.spec.js",
    "**/*.test.ts",
    "**/*.test.tsx",
    "**/*.test.js",
    "**/*_test.py",
    "**/*_test.go",
    "**/*_test.rs",
    "**/*.d.ts",
];

impl FileClassifier for DefaultClassifier {
    fn is_binary(&self, ext: &str) -> bool {
        // Case- and dot-insensitive (see `ScanRules::is_binary_ext`). A raw
        // case-sensitive match used to let real binaries through on any
        // filesystem that preserves upper-case extensions (`IMG_1234.JPG`,
        // `chart.PNG`), and each one that slipped past here was then skipped
        // later WITHOUT being fingerprinted — so it looked "changed" on every
        // single reconcile pass, forever.
        scan_rules().is_binary_ext(ext)
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
        scan_rules().is_fallback_source_ext(&e)
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

    // The override tests below exercise `ScanRules::from_overrides` directly and
    // never call `init_scan_rules`: that writes a process-wide OnceLock, so
    // touching it here would leak into every other test in the binary.

    /// The whole point of the config layer: add a format without a release.
    #[test]
    fn overrides_can_add_a_binary_extension() {
        let rules = ScanRules::from_overrides(&ScanRuleOverrides {
            binary_add: vec!["parquet2".into(), ".XYZ".into()],
            ..Default::default()
        });
        assert!(rules.is_binary_ext("parquet2"), "added extension is binary");
        assert!(rules.is_binary_ext("xyz"), "added extension is normalised (dot + case stripped)");
        assert!(rules.is_binary_ext("png"), "defaults still apply alongside an add");
    }

    /// And remove one the defaults get wrong for a given user.
    #[test]
    fn overrides_can_remove_a_default_binary_extension() {
        let rules = ScanRules::from_overrides(&ScanRuleOverrides {
            binary_remove: vec!["SVG".into(), ".epub".into()],
            ..Default::default()
        });
        assert!(!rules.is_binary_ext("svg"), "removal is case-insensitive");
        assert!(!rules.is_binary_ext("epub"), "removal tolerates a leading dot");
        assert!(rules.is_binary_ext("png"), "unrelated defaults are untouched");
    }

    #[test]
    fn overrides_can_add_and_remove_exclude_globs() {
        let rules = ScanRules::from_overrides(&ScanRuleOverrides {
            exclude_add: vec!["**/coverage/**".into()],
            exclude_remove: vec!["**/*.d.ts".into()],
            ..Default::default()
        });
        assert!(rules.exclude_globs().is_match("pkg/coverage/index.html"), "added glob excludes");
        assert!(
            !rules.exclude_globs().is_match("src/types.d.ts"),
            "removed glob no longer excludes"
        );
        assert!(
            rules.exclude_globs().is_match("node_modules/x/y.js"),
            "other defaults still apply"
        );
    }

    /// One bad operator-supplied pattern must not empty the whole exclude set —
    /// that would silently start indexing node_modules.
    #[test]
    fn one_unparseable_glob_does_not_destroy_the_exclude_set() {
        let rules = ScanRules::from_overrides(&ScanRuleOverrides {
            exclude_add: vec!["**/{unclosed".into()],
            ..Default::default()
        });
        assert!(
            rules.exclude_globs().is_match("node_modules/x/y.js"),
            "defaults survive an invalid added pattern"
        );
    }

    /// A malformed config value degrades to "no override", never an error that
    /// stops the daemon scanning.
    #[test]
    fn malformed_config_value_is_ignored() {
        assert!(parse_string_array("scan.binary_exts.add", Some("not json".into())).is_empty());
        assert!(parse_string_array("scan.binary_exts.add", Some("{\"a\":1}".into())).is_empty());
        assert!(parse_string_array("scan.binary_exts.add", Some("  ".into())).is_empty());
        assert!(parse_string_array("scan.binary_exts.add", None).is_empty());
        assert_eq!(
            parse_string_array("scan.binary_exts.add", Some(r#"["a","b"]"#.into())),
            vec!["a".to_string(), "b".to_string()],
        );
    }

    #[test]
    fn overrides_from_config_reads_every_documented_key() {
        let overrides = ScanRuleOverrides::from_config(|k| match k {
            "scan.binary_exts.add" => Some(r#"["aa"]"#.into()),
            "scan.binary_exts.remove" => Some(r#"["bb"]"#.into()),
            "scan.source_exts.add" => Some(r#"["cc"]"#.into()),
            "scan.source_exts.remove" => Some(r#"["dd"]"#.into()),
            "scan.exclude_globs.add" => Some(r#"["**/ee/**"]"#.into()),
            "scan.exclude_globs.remove" => Some(r#"["**/ff/**"]"#.into()),
            _ => None,
        });
        assert_eq!(overrides.binary_add, ["aa"]);
        assert_eq!(overrides.binary_remove, ["bb"]);
        assert_eq!(overrides.source_add, ["cc"]);
        assert_eq!(overrides.source_remove, ["dd"]);
        assert_eq!(overrides.exclude_add, ["**/ee/**"]);
        assert_eq!(overrides.exclude_remove, ["**/ff/**"]);
        assert!(!overrides.is_empty());
        // Every key the loader reads must be documented in the public list, or an
        // operator has no way to discover it.
        for k in ["scan.binary_exts.add", "scan.exclude_globs.remove"] {
            assert!(SCAN_RULE_CONFIG_KEYS.contains(&k), "{k} must be listed");
        }
        assert!(ScanRuleOverrides::from_config(|_| None).is_empty(), "no keys set → no overrides");
    }

    /// Defaults must be reachable with no config at all — the fallback path every
    /// unit test and any non-daemon caller takes.
    #[test]
    fn defaults_apply_when_nothing_is_overridden() {
        let rules = ScanRules::from_overrides(&ScanRuleOverrides::default());
        assert!(rules.is_binary_ext("png"));
        assert!(rules.is_binary_ext("p12"), "the extended defaults are present");
        assert!(!rules.is_binary_ext("rs"));
        assert!(rules.is_fallback_source_ext("go"));
        assert!(rules.exclude_globs().is_match("node_modules/x/y.js"));
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
            "p12",
            "pfx",
            "jks",
            "keystore",
            "epub",
            "mobi",
            "psd",
            "npz",
            "npy",
            "safetensors",
            "aar",
            "apk",
            "dmg",
            "lockb",
            "dump",
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
