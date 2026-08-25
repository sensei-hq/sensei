//! Deterministic project-icon inference — fills the generic 場 fallback so
//! every project gets a meaningful icon at a glance. See
//! [[pipeline/project-icon]] for the design.
//!
//! The scanner writes `sensei.projects.icon` (jsonb `{kind, value, source}`).
//! The app reads `kind` + `value` only (`kind:"image"` → `<img src=value>`,
//! `kind:"kanji"` → glyph, else → 場); `source` is provenance the app ignores.
//!
//! The *choice* is pure and unit-testable: [`infer_icon`] takes the project
//! name, its stack, the current icon, and the set of logo/asset paths already
//! found on disk (fed by the existing `scan_icons` filesystem walk). No disk,
//! no clock — deterministic.
//!
//! Voice: sentence case, lowercase "sensei", no filler.

use std::path::{Component, Path, PathBuf};

use serde::Serialize;
use serde_json::Value;

/// The last-resort glyph — 場 (place). Reused from the project-overview
/// read-path so the write and read sides agree on the single fallback.
use crate::project_overview::DEFAULT_KANJI;

// Provenance tags (also serialized into `icon.source`). Ordered by priority —
// lower rank wins. Kept in one place so the guard and the chain agree.
const SRC_README: &str = "readme";
const SRC_LOGO: &str = "logo_file";
const SRC_FAVICON: &str = "favicon";
const SRC_PACKAGE: &str = "package_branding";
const SRC_KANJI: &str = "kanji_map";
const SRC_LETTER: &str = "letter_fallback";
const SRC_DEFAULT: &str = "default";

/// Priority rank of a machine-inference `source` (lower = higher priority).
/// `None` marks a source sensei never produces — i.e. an author/About-form
/// choice, which the guard must never clobber.
fn source_rank(source: &str) -> Option<u8> {
    match source {
        SRC_README => Some(0),
        SRC_LOGO => Some(1),
        SRC_FAVICON => Some(2),
        SRC_PACKAGE => Some(3),
        SRC_KANJI => Some(4),
        SRC_LETTER => Some(5),
        SRC_DEFAULT => Some(6),
        _ => None,
    }
}

/// An inferred icon, serialized verbatim into `sensei.projects.icon`.
///
/// `kind` is always one of the two the app renders (`"image"` or `"kanji"`);
/// the letter fallback rides on `kind:"kanji"` so a Latin initial renders as a
/// glyph today (the app has no `letter` arm — see module notes / spec deviation).
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Icon {
    pub kind: &'static str,
    pub value: String,
    pub source: &'static str,
}

/// The write decision for a project's icon.
///
/// `Keep` means leave the stored icon untouched — it is either an author choice
/// or an equal-or-higher-priority machine icon (re-writing would churn
/// `modified_at` or flip-flop between a project's repos). `Set` carries the icon
/// to persist.
#[derive(Debug, Clone, PartialEq)]
pub enum IconDecision {
    Keep,
    Set(Icon),
}

/// Whether the stored icon is an author/human choice (About form or a
/// create endpoint) that inference must never override. A choice is "author-set"
/// when it has a non-empty `value` and its `source` is absent or is not one of
/// sensei's inference tags.
fn is_author_set(existing: &Value) -> bool {
    let has_value =
        existing.get("value").and_then(Value::as_str).is_some_and(|v| !v.trim().is_empty());
    if !has_value {
        return false;
    }
    match existing.get("source").and_then(Value::as_str) {
        Some(s) => source_rank(s).is_none(),
        None => true,
    }
}

/// The priority rank of a prior *machine* icon, or `None` when the stored icon
/// is empty (no value) — an empty slot is always writable.
fn machine_rank(existing: &Value) -> Option<u8> {
    let has_value =
        existing.get("value").and_then(Value::as_str).is_some_and(|v| !v.trim().is_empty());
    if !has_value {
        return None;
    }
    existing.get("source").and_then(Value::as_str).and_then(source_rank)
}

/// Decide the icon to persist for a project. Deterministic — the same inputs
/// always yield the same decision.
///
/// Priority (first hit wins):
/// 1. **explicit** — an author/About-form icon is kept as-is ([`IconDecision::Keep`]).
/// 2. **logo / image file** — any asset path from the filesystem walk
///    (`scan_icons`), stored as `kind:"image"` with the repo-relative path.
/// 3. **kanji from stack** — a domain kanji for the dominant mapped stack.
/// 4. **letter** — the project name's first alphabetic character, uppercased.
/// 5. **default** — 場, only for a nameless, stackless, logoless project.
///
/// A prior machine icon is only replaced by a strictly higher-priority source,
/// so re-scans neither churn nor flip-flop between a project's member repos.
pub fn infer_icon(
    name: &str,
    stack: &[String],
    existing_icon: &Value,
    present_asset_paths: &[String],
) -> IconDecision {
    // 1. Never override a human choice.
    if is_author_set(existing_icon) {
        return IconDecision::Keep;
    }

    let candidate = choose_icon(name, stack, present_asset_paths);

    // Only upgrade a prior machine icon — equal or lower priority is a no-op.
    if let Some(existing) = machine_rank(existing_icon)
        && source_rank(candidate.source).unwrap_or(u8::MAX) >= existing
    {
        return IconDecision::Keep;
    }
    IconDecision::Set(candidate)
}

/// The best fresh icon for the current repo state, ignoring what is stored.
fn choose_icon(name: &str, stack: &[String], present_asset_paths: &[String]) -> Icon {
    // 2. Logo / image file — the first non-empty path the walk surfaced.
    if let Some(path) = present_asset_paths.iter().find(|p| !p.trim().is_empty()) {
        return Icon { kind: "image", value: path.clone(), source: SRC_LOGO };
    }
    // 3. Kanji from the dominant mapped stack.
    if let Some(k) = kanji_from_stack(stack) {
        return Icon { kind: "kanji", value: k.to_string(), source: SRC_KANJI };
    }
    // 4. Letter from the name (rides on kind:"kanji" so it renders today).
    if let Some(letter) = letter_from_name(name) {
        return Icon { kind: "kanji", value: letter, source: SRC_LETTER };
    }
    // 5. Documented last resort.
    Icon { kind: "kanji", value: DEFAULT_KANJI.to_string(), source: SRC_DEFAULT }
}

/// The first alphabetic character of `name` uppercased, falling back to the
/// first alphanumeric character. `None` for an empty / symbol-only name.
fn letter_from_name(name: &str) -> Option<String> {
    name.chars()
        .find(|c| c.is_alphabetic())
        .or_else(|| name.chars().find(|c| c.is_alphanumeric()))
        .map(|c| c.to_uppercase().to_string())
}

/// A domain kanji for the dominant stack — the first stack label (in order)
/// that has a mapping. `None` when no label is mapped (→ letter fallback).
pub fn kanji_from_stack(stack: &[String]) -> Option<&'static str> {
    stack.iter().find_map(|s| kanji_for(s))
}

/// The kanji for a single stack label, or `None` if unmapped. Labels arrive
/// lowercase from the detector and frontmatter; normalized here defensively.
fn kanji_for(label: &str) -> Option<&'static str> {
    let l = label.trim().to_ascii_lowercase();
    KANJI_MAP.iter().find(|(k, _)| *k == l).map(|(_, v)| *v)
}

// ── serve-side: resolve + read an image icon's bytes ──────────────────────
//
// The daemon serves an inferred image icon at `GET /api/projects/{id}/icon`.
// Because that streams a file from a repo-relative path, path-traversal is the
// risk: the resolver must never read outside the repo root. Safety is split so
// the security boundary is unit-testable:
//   * [`resolve_icon_path`]  — PURE lexical check (no disk): rejects `..`,
//     absolute paths, and non-image extensions.
//   * [`read_icon_bytes`]    — disk read that ALSO canonicalizes and asserts
//     the file stays inside the (canonicalized) root, defeating symlink-out.

/// Content-Type for a served icon extension, or `None` for a non-image
/// extension. This allowlist IS the security boundary for the serve route: an
/// unlisted extension is never resolved, read, or streamed.
pub fn icon_content_type(ext: &str) -> Option<&'static str> {
    match ext.to_ascii_lowercase().as_str() {
        "svg" => Some("image/svg+xml"),
        "png" => Some("image/png"),
        "ico" => Some("image/x-icon"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "webp" => Some("image/webp"),
        "gif" => Some("image/gif"),
        _ => None,
    }
}

/// Lexically resolve a repo-relative icon path against a repo `root`, rejecting
/// anything that could escape the root. PURE — no disk, no canonicalize — so
/// traversal rejection is unit-testable in isolation. Returns `None` when `rel`
/// is empty, absolute, contains any non-`Normal` component (`..`, `.`, a root,
/// or a Windows prefix), or has a non-image extension. A returned path is only
/// *lexically* safe — the caller ([`read_icon_bytes`]) still canonicalizes and
/// asserts the result stays inside the root to defeat a symlink that points out
/// of the tree.
pub fn resolve_icon_path(root: &Path, rel: &str) -> Option<PathBuf> {
    let rel = rel.trim();
    if rel.is_empty() {
        return None;
    }
    let rel_path = Path::new(rel);
    // Every component must be a plain name — this rejects `..` traversal,
    // absolute paths, `.`, and prefixes lexically, before any disk access.
    if !rel_path.components().all(|c| matches!(c, Component::Normal(_))) {
        return None;
    }
    // Allowlist image extensions only.
    let ext = rel_path.extension().and_then(|e| e.to_str())?;
    icon_content_type(ext)?;
    Some(root.join(rel_path))
}

/// Read an image icon's bytes for a repo-relative path, enforcing that the
/// resolved file stays inside `root` on disk. Canonicalizes both sides so a
/// symlink pointing out of the repo is rejected (`canonical.starts_with(root)`).
/// Returns the Content-Type + bytes, or `None` when the path is unsafe, has a
/// non-image extension, or the file is missing/unreadable. Blocking disk I/O —
/// call from `spawn_blocking`.
pub fn read_icon_bytes(root: &Path, rel: &str) -> Option<(&'static str, Vec<u8>)> {
    let candidate = resolve_icon_path(root, rel)?;
    let ext = candidate.extension().and_then(|e| e.to_str())?;
    let content_type = icon_content_type(ext)?;
    // Canonicalize both sides and assert containment — defeats symlink-out and
    // any residual traversal the lexical check couldn't see.
    let canon = candidate.canonicalize().ok()?;
    let root_canon = root.canonicalize().ok()?;
    if !canon.starts_with(&root_canon) {
        return None;
    }
    let bytes = std::fs::read(&canon).ok()?;
    Some((content_type, bytes))
}

/// Serve an inferred image icon by trying each repo root in turn — the stored
/// relative path belongs to exactly one of a project's repos. Returns the first
/// root where the path resolves safely (inside the root, image extension) and
/// the file reads. Blocking disk I/O — call from `spawn_blocking`.
pub fn serve_icon_from_roots(roots: &[String], rel: &str) -> Option<(&'static str, Vec<u8>)> {
    roots.iter().find_map(|root| read_icon_bytes(Path::new(root), rel))
}

/// Stack label → single domain kanji. Covers every label the scanner's
/// `detect_stack` can emit, plus common frontmatter-supplied stacks.
///
/// Motivated where possible: 鉄 iron (rust), 蛇 snake (python), 象 elephant
/// (php), 燕 swift-bird (swift), 雪 snow (svelte), 碁 the game go, 玉 gem (ruby),
/// 次 next (nextjs), 網 net (dotnet), 珈 coffee (java), 築 build (gradle/maven),
/// 型 type / 文 script, 景 view (vue), 応 reaction (react).
const KANJI_MAP: &[(&str, &str)] = &[
    // Languages / runtimes the detector emits.
    ("rust", "鉄"),
    ("go", "碁"),
    ("python", "蛇"),
    ("ruby", "玉"),
    ("php", "象"),
    ("swift", "燕"),
    ("java", "珈"),
    ("dotnet", "網"),
    ("typescript", "型"),
    // Build tools (emitted alongside "java", so rarely dominant).
    ("gradle", "築"),
    ("maven", "築"),
    // Frontend frameworks the npm adapter emits.
    ("svelte", "雪"),
    ("react", "応"),
    ("vue", "景"),
    ("nextjs", "次"),
    // Common frontmatter-supplied labels (spec-listed).
    ("javascript", "文"),
    ("sql", "庫"),
    ("docs", "書"),
    ("tauri", "匠"),
];

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn paths(p: &[&str]) -> Vec<String> {
        p.iter().map(|s| s.to_string()).collect()
    }
    fn stack(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }

    // ── priority order ────────────────────────────────────────────────

    #[test]
    fn explicit_author_icon_wins() {
        // An About-form kanji (no source) is kept even when a logo exists.
        let existing = json!({ "kind": "kanji", "value": "禅" });
        assert_eq!(
            infer_icon("proj", &stack(&["rust"]), &existing, &paths(&["logo.svg"])),
            IconDecision::Keep
        );
        // A non-machine source is also an author choice.
        let picked = json!({ "kind": "image", "value": "x.png", "source": "user_upload" });
        assert_eq!(
            infer_icon("proj", &stack(&["rust"]), &picked, &paths(&["logo.svg"])),
            IconDecision::Keep
        );
    }

    #[test]
    fn logo_file_beats_kanji() {
        let d = infer_icon("proj", &stack(&["rust"]), &json!({}), &paths(&["assets/logo.svg"]));
        assert_eq!(
            d,
            IconDecision::Set(Icon {
                kind: "image",
                value: "assets/logo.svg".into(),
                source: SRC_LOGO,
            })
        );
    }

    #[test]
    fn logo_picks_first_present_path() {
        // The pure chooser takes the first non-empty candidate (the walk ranks).
        let d = infer_icon("proj", &[], &json!({}), &paths(&["", "logo.png", "icon.svg"]));
        match d {
            IconDecision::Set(Icon { kind: "image", value, .. }) => assert_eq!(value, "logo.png"),
            other => panic!("expected image, got {other:?}"),
        }
    }

    #[test]
    fn kanji_from_stack_for_each_mapped_stack() {
        let cases = [
            ("rust", "鉄"),
            ("go", "碁"),
            ("python", "蛇"),
            ("ruby", "玉"),
            ("php", "象"),
            ("swift", "燕"),
            ("java", "珈"),
            ("dotnet", "網"),
            ("typescript", "型"),
            ("svelte", "雪"),
            ("react", "応"),
            ("vue", "景"),
            ("nextjs", "次"),
        ];
        for (label, glyph) in cases {
            let d = infer_icon("proj", &stack(&[label]), &json!({}), &[]);
            assert_eq!(
                d,
                IconDecision::Set(Icon { kind: "kanji", value: glyph.into(), source: SRC_KANJI }),
                "stack {label} should map to {glyph}"
            );
        }
    }

    #[test]
    fn dominant_stack_is_first_mapped_label() {
        // sensei's monorepo frontmatter stack — rust wins over sveltekit.
        let d = infer_icon("sensei", &stack(&["rust", "sveltekit"]), &json!({}), &[]);
        assert_eq!(
            d,
            IconDecision::Set(Icon { kind: "kanji", value: "鉄".into(), source: SRC_KANJI })
        );
    }

    #[test]
    fn unmapped_stack_falls_through_to_letter() {
        // "cobol" has no kanji → letter from the name.
        let d = infer_icon("cobolapp", &stack(&["cobol"]), &json!({}), &[]);
        assert_eq!(
            d,
            IconDecision::Set(Icon { kind: "kanji", value: "C".into(), source: SRC_LETTER })
        );
    }

    #[test]
    fn letter_fallback_uppercases_first_alpha() {
        let d = infer_icon("sensei", &[], &json!({}), &[]);
        assert_eq!(
            d,
            IconDecision::Set(Icon { kind: "kanji", value: "S".into(), source: SRC_LETTER })
        );
        // Leading non-alpha is skipped; first alphabetic char is used.
        assert_eq!(letter_from_name("42tools").as_deref(), Some("T"));
        // A CJK name yields its first glyph.
        assert_eq!(letter_from_name("禅道").as_deref(), Some("禅"));
        // No alpha → first alphanumeric.
        assert_eq!(letter_from_name("42").as_deref(), Some("4"));
    }

    #[test]
    fn empty_or_symbol_name_with_no_stack_uses_documented_default() {
        for name in ["", "   ", "@#$"] {
            let d = infer_icon(name, &[], &json!({}), &[]);
            assert_eq!(
                d,
                IconDecision::Set(Icon {
                    kind: "kanji",
                    value: DEFAULT_KANJI.into(),
                    source: SRC_DEFAULT,
                }),
                "name {name:?} should fall to the documented default"
            );
        }
    }

    // ── re-scan / idempotence guard ───────────────────────────────────

    #[test]
    fn same_machine_icon_is_kept_no_churn() {
        // A prior kanji_map icon, re-inferred identically → Keep (no re-write).
        let existing = json!({ "kind": "kanji", "value": "鉄", "source": SRC_KANJI });
        assert_eq!(infer_icon("proj", &stack(&["rust"]), &existing, &[]), IconDecision::Keep);
    }

    #[test]
    fn higher_priority_source_upgrades_a_machine_icon() {
        // A kanji_map icon is upgraded when a logo appears.
        let existing = json!({ "kind": "kanji", "value": "鉄", "source": SRC_KANJI });
        let d = infer_icon("proj", &stack(&["rust"]), &existing, &paths(&["logo.svg"]));
        assert_eq!(
            d,
            IconDecision::Set(Icon { kind: "image", value: "logo.svg".into(), source: SRC_LOGO })
        );
    }

    #[test]
    fn lower_priority_source_never_downgrades() {
        // An image icon is not replaced by a kanji when the logo path is gone
        // this pass (multi-repo flip-flop / transient-miss guard).
        let existing = json!({ "kind": "image", "value": "logo.svg", "source": SRC_LOGO });
        assert_eq!(infer_icon("proj", &stack(&["rust"]), &existing, &[]), IconDecision::Keep);
    }

    #[test]
    fn empty_icon_is_always_written() {
        // The default `{}` slot is writable.
        assert!(matches!(
            infer_icon("proj", &stack(&["rust"]), &json!({}), &[]),
            IconDecision::Set(_)
        ));
        assert!(matches!(
            infer_icon("proj", &stack(&["rust"]), &Value::Null, &[]),
            IconDecision::Set(_)
        ));
    }

    // ── kanji map totality ────────────────────────────────────────────

    /// Every stack label `scan_logic::detect_stack` + the manifest adapters can
    /// emit. If a new detector label lands without a kanji, this test fails —
    /// forcing a mapping or a documented fallthrough decision.
    const DETECTOR_STACK_LABELS: &[&str] = &[
        "rust",
        "ruby",
        "php",
        "svelte",
        "react",
        "vue",
        "nextjs",
        "typescript",
        "python",
        "dotnet",
        "java",
        "gradle",
        "maven",
        "go",
        "swift",
    ];

    #[test]
    fn kanji_map_is_total_for_detector_labels() {
        for label in DETECTOR_STACK_LABELS {
            assert!(
                kanji_for(label).is_some(),
                "detector stack label {label} has no kanji mapping"
            );
        }
    }

    #[test]
    fn icon_serializes_to_the_wire_shape() {
        let icon = Icon { kind: "kanji", value: "鉄".into(), source: SRC_KANJI };
        assert_eq!(
            serde_json::to_value(&icon).unwrap(),
            json!({ "kind": "kanji", "value": "鉄", "source": "kanji_map" })
        );
    }

    // ── serve-side: content-type allowlist ────────────────────────────

    #[test]
    fn content_type_maps_known_image_extensions() {
        assert_eq!(icon_content_type("svg"), Some("image/svg+xml"));
        assert_eq!(icon_content_type("SVG"), Some("image/svg+xml")); // case-insensitive
        assert_eq!(icon_content_type("png"), Some("image/png"));
        assert_eq!(icon_content_type("ico"), Some("image/x-icon"));
        assert_eq!(icon_content_type("jpg"), Some("image/jpeg"));
        assert_eq!(icon_content_type("jpeg"), Some("image/jpeg"));
        assert_eq!(icon_content_type("webp"), Some("image/webp"));
        assert_eq!(icon_content_type("gif"), Some("image/gif"));
    }

    #[test]
    fn content_type_rejects_non_image_extensions() {
        for ext in ["txt", "exe", "sh", "html", "js", "", "svgz"] {
            assert_eq!(icon_content_type(ext), None, "{ext} must not be servable");
        }
    }

    // ── serve-side: lexical path safety (no disk) ─────────────────────

    #[test]
    fn resolve_accepts_a_safe_relative_image_path() {
        let root = Path::new("/repo");
        assert_eq!(resolve_icon_path(root, "logo.svg"), Some(PathBuf::from("/repo/logo.svg")));
        assert_eq!(
            resolve_icon_path(root, "assets/logo.png"),
            Some(PathBuf::from("/repo/assets/logo.png"))
        );
        // Leading/trailing whitespace is trimmed.
        assert_eq!(resolve_icon_path(root, "  logo.svg  "), Some(PathBuf::from("/repo/logo.svg")));
    }

    #[test]
    fn resolve_rejects_parent_traversal() {
        let root = Path::new("/repo");
        // `..` at any position escapes the root — must be rejected lexically.
        assert_eq!(resolve_icon_path(root, "../secret.svg"), None);
        assert_eq!(resolve_icon_path(root, "assets/../../secret.png"), None);
        assert_eq!(resolve_icon_path(root, "a/../../b.svg"), None);
    }

    #[test]
    fn resolve_rejects_absolute_and_dotslash_and_prefix() {
        let root = Path::new("/repo");
        assert_eq!(resolve_icon_path(root, "/etc/passwd.png"), None); // absolute
        assert_eq!(resolve_icon_path(root, "./logo.svg"), None); // CurDir component
    }

    #[test]
    fn resolve_rejects_non_image_extension() {
        let root = Path::new("/repo");
        assert_eq!(resolve_icon_path(root, "logo.txt"), None);
        assert_eq!(resolve_icon_path(root, "run.sh"), None);
        assert_eq!(resolve_icon_path(root, "noext"), None);
    }

    #[test]
    fn resolve_rejects_empty() {
        let root = Path::new("/repo");
        assert_eq!(resolve_icon_path(root, ""), None);
        assert_eq!(resolve_icon_path(root, "   "), None);
    }

    // ── serve-side: on-disk read + containment (canonicalize) ─────────

    #[test]
    fn read_serves_a_file_inside_the_root_with_its_content_type() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("assets")).unwrap();
        std::fs::write(dir.path().join("assets/logo.svg"), b"<svg/>").unwrap();

        let got = read_icon_bytes(dir.path(), "assets/logo.svg");
        assert_eq!(got, Some(("image/svg+xml", b"<svg/>".to_vec())));
    }

    #[test]
    fn read_returns_none_for_a_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(read_icon_bytes(dir.path(), "logo.svg"), None);
    }

    #[test]
    fn read_returns_none_for_traversal_even_when_target_exists() {
        // A real file outside the root must NOT be reachable via `..`.
        let outer = tempfile::tempdir().unwrap();
        std::fs::write(outer.path().join("secret.svg"), b"secret").unwrap();
        let root = outer.path().join("repo");
        std::fs::create_dir_all(&root).unwrap();
        assert_eq!(read_icon_bytes(&root, "../secret.svg"), None);
    }

    #[cfg(unix)]
    #[test]
    fn read_rejects_a_symlink_pointing_out_of_the_root() {
        // Lexically the path is a plain name, but it symlinks OUT of the repo —
        // the canonicalize + starts_with check is what rejects it.
        let outer = tempfile::tempdir().unwrap();
        std::fs::write(outer.path().join("secret.svg"), b"secret").unwrap();
        let root = outer.path().join("repo");
        std::fs::create_dir_all(&root).unwrap();
        std::os::unix::fs::symlink(outer.path().join("secret.svg"), root.join("logo.svg")).unwrap();

        // resolve_icon_path passes (lexically safe), read must still reject it.
        assert!(resolve_icon_path(&root, "logo.svg").is_some());
        assert_eq!(read_icon_bytes(&root, "logo.svg"), None);
    }

    #[test]
    fn serve_from_roots_picks_the_repo_that_has_the_file() {
        let a = tempfile::tempdir().unwrap();
        let b = tempfile::tempdir().unwrap();
        // Only repo `b` actually has the asset.
        std::fs::write(b.path().join("logo.png"), b"PNG").unwrap();

        let roots =
            vec![a.path().to_string_lossy().to_string(), b.path().to_string_lossy().to_string()];
        assert_eq!(serve_icon_from_roots(&roots, "logo.png"), Some(("image/png", b"PNG".to_vec())));
        // No repo has it → None (→ the handler 404s).
        assert_eq!(serve_icon_from_roots(&roots, "missing.svg"), None);
    }
}
